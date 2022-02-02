extern crate core;
#[macro_use]
extern crate assert_float_eq;

mod astro_math;
pub mod config;
pub mod enums;
pub mod errors;
pub mod guide;
pub mod observing_pos;
pub mod parking;
pub mod pointing_pos;
pub mod slew;
pub mod sync;
pub mod target;
pub mod tracking;

use crate::astro_math::{Degrees, Hours};
use crate::config::{Config, TelescopeDetails};
use crate::enums::{
    AlignmentMode, EquatorialCoordinateType, MotionState, PierSide, SlewingState, Target,
    TrackingRate, TrackingState,
};
use crate::errors::{AlpacaError, ErrorType, Result};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use synscan;
use synscan::motors::DriveMode;
use synscan::util::{AutoGuideSpeed, SingleChannel};
use tokio::sync::{watch, RwLock};
use tokio::task;

const RA_CHANNEL: &SingleChannel = &SingleChannel::Channel1;

#[derive(Debug)]
struct State {
    observation_location: config::ObservingLocation,
    date_offset: chrono::Duration,

    park_pos: Degrees,
    // Pos
    hour_angle_offset: Hours, // pos+offset = ha
    declination: Degrees,

    pier_side: PierSide,

    tracking_rate: TrackingRate,
    post_slew_settle_time: f64,
    target: Target,
    motion_state: MotionState,
    autoguide_speed: AutoGuideSpeed,
}

pub struct StarAdventurer {
    driver: Arc<Mutex<synscan::MotorController<'static>>>,
    telescope_details: TelescopeDetails,
    state: Arc<RwLock<State>>,
}

impl StarAdventurer {
    pub async fn new(config: &Config) -> Result<Self> {
        let mut driver = synscan::MotorController::new(
            config.com_settings.path.as_str(),
            config.com_settings.baud_rate,
            Duration::from_millis(config.com_settings.timeout_millis as u64),
        )?;
        let motor_params = *driver.get_motor_parameters();
        let status = driver.get_status(RA_CHANNEL)?;
        let step_period = driver.get_step_period(RA_CHANNEL)?;

        let mut tracking_rate = TrackingRate::Sidereal;
        let mut is_goto_slewing = false;
        let mut goto_slewing_target = 0.;

        let motion_state = match status.mode {
            DriveMode::Tracking => {
                match TrackingRate::determine_from_period(
                    step_period,
                    motor_params.get_timer_interrupt_frequency(),
                    motor_params.get_counts_per_revolution(*RA_CHANNEL),
                ) {
                    Some(rate) => {
                        tracking_rate = rate;
                    }
                    None => {
                        tracking_rate = TrackingRate::Sidereal;
                        driver.set_step_period(
                            RA_CHANNEL,
                            TrackingRate::Sidereal.determine_step_period(
                                motor_params.get_timer_interrupt_frequency(),
                                motor_params.get_counts_per_revolution(*RA_CHANNEL),
                            ),
                        )?
                    }
                };

                if status.running {
                    MotionState::Tracking(TrackingState::Tracking(None))
                } else {
                    MotionState::Tracking(TrackingState::Stationary(false))
                }
            }
            DriveMode::Goto => {
                if status.running {
                    goto_slewing_target = driver.get_goto_target(RA_CHANNEL)?;
                    is_goto_slewing = true;

                    // placeholder until we have state object
                    MotionState::Tracking(TrackingState::Stationary(false))
                } else {
                    let direction =
                        Self::get_tracking_direction(config.observation_location.latitude);
                    driver.set_motion_mode(RA_CHANNEL, DriveMode::Tracking, false, direction)?;
                    driver.set_step_period(
                        RA_CHANNEL,
                        tracking_rate.determine_step_period(
                            motor_params.get_timer_interrupt_frequency(),
                            motor_params.get_counts_per_revolution(*RA_CHANNEL),
                        ),
                    )?;
                    MotionState::Tracking(TrackingState::Stationary(false))
                }
            }
        };

        driver.set_autoguide_speed(RA_CHANNEL, AutoGuideSpeed::Half)?;

        let state = State {
            observation_location: config.observation_location,
            park_pos: 0.0, // TODO: Put an angular park pos in config (that only works after alignment)
            declination: 0.0, // Set only by sync or goto
            hour_angle_offset: 0.0, // Set only by sync or goto
            autoguide_speed: AutoGuideSpeed::Half, // Write only, so default to half b/c most standard
            pier_side: PierSide::Unknown,          // TODO use this?
            date_offset: chrono::Duration::zero(), // Assume using actual time
            post_slew_settle_time: 0.0,            // Default to zero
            target: Default::default(),            // No target initially
            tracking_rate,
            motion_state,
        };

        let driver_arc = Arc::new(Mutex::new(driver));
        let state_arc = Arc::new(RwLock::new(state));

        // Spawn task to track goto position
        if is_goto_slewing {
            let state_arc_clone = state_arc.clone();
            let mut state_lock = state_arc_clone.write().await;
            let (cancel_tx, cancel_rx) = watch::channel(false);
            let _goto_task = task::spawn(Self::goto_task(
                state_arc.clone(),
                driver_arc.clone(),
                cancel_rx,
            ));
            state_lock.motion_state = MotionState::Slewing(SlewingState::GotoSlewing(
                goto_slewing_target,
                TrackingState::Stationary(false),
                cancel_tx,
            ));
        }

        Ok(StarAdventurer {
            driver: driver_arc,
            state: state_arc,
            telescope_details: config.telescope_details,
        })
    }

    /// Returns the alignment mode of the mount (Alt/Az, Polar, German Polar)
    pub fn get_alignment_mode(&self) -> Result<AlignmentMode> {
        return Ok(AlignmentMode::GermanPolar);
    }

    /// Returns the current equatorial coordinate system used by this telescope (e.g. Topocentric or J2000).
    pub fn get_equatorial_system(&self) -> Result<EquatorialCoordinateType> {
        Ok(EquatorialCoordinateType::Topocentric)
    }

    /// The telescope's effective aperture diameter (meters)
    pub fn get_aperture(&self) -> Result<f64> {
        self.telescope_details.aperture.ok_or(AlpacaError::from_msg(
            ErrorType::ValueNotSet,
            format!("Aperture not defined"),
        ))
    }

    /// The area of the telescope's aperture, taking into account any obstructions (square meters)
    pub fn get_aperture_area(&self) -> Result<f64> {
        self.telescope_details
            .aperture_area
            .ok_or(AlpacaError::from_msg(
                ErrorType::ValueNotSet,
                format!("Aperture area not defined"),
            ))
    }

    /// The telescope's focal length in meters
    pub fn get_focal_length(&self) -> Result<f64> {
        self.telescope_details
            .focal_length
            .ok_or(AlpacaError::from_msg(
                ErrorType::ValueNotSet,
                format!("Focal length not defined"),
            ))
    }

    /// True if the mount is stopped in the Home position. Set only following a FindHome() operation, and reset with any slew operation.
    /// This property must be False if the telescope does not support homing.
    pub fn is_home(&self) -> Result<bool> {
        Ok(false)
    }

    /// True if the telescope or driver applies atmospheric refraction to coordinates.
    pub fn does_refraction(&self) -> Result<bool> {
        Ok(false)
    }

    /// Tell the telescope or driver whether to apply atmospheric refraction to coordinates.
    pub fn set_does_refraction(&mut self, _does_refraction: bool) -> Result<()> {
        // TODO implement these?
        Err(AlpacaError::from_msg(
            ErrorType::ActionNotImplemented,
            format!("Refraction calculations not available"),
        ))
    }

    /// Indicates the pointing state of the mount
    pub async fn get_side_of_pier(&self) -> Result<PierSide> {
        Ok(self.state.read().await.pier_side)
    }

    /// True if the SideOfPier property can be set, meaning that the mount can be forced to flip.
    pub fn can_set_side_of_pier(&self) -> Result<bool> {
        Ok(false) // FIXME revisit
    }

    /// Sets the pointing state of the mount
    pub fn set_side_of_pier(&self, _side: PierSide) -> Result<()> {
        Err(AlpacaError::from_msg(
            ErrorType::ActionNotImplemented,
            format!("Side of pier control not implemented"),
        ))
    }
}
