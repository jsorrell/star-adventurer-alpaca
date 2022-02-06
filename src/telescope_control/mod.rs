pub mod config;
pub mod guide;
pub mod observing_pos;
pub mod parking;
pub mod pointing_pos;
pub mod slew;
pub mod sync;
pub mod target;
pub(in crate::telescope_control) mod test_util;
pub mod tracking;

use crate::util::enums::*;
use crate::util::result::*;
use config::{Config, TelescopeDetails};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use synscan;
use synscan::motors::DriveMode;
use synscan::result::SynScanError;
use synscan::util::{AutoGuideSpeed, SingleChannel};
use target::Target;
use tokio::sync::{watch, RwLock};
use tokio::task;

const RA_CHANNEL: &SingleChannel = &SingleChannel::Channel1;

#[derive(Debug)]
pub(in crate::telescope_control) struct State {
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
    pub async fn new(config: &Config) -> AscomResult<Self> {
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
    pub async fn get_alignment_mode(&self) -> AscomResult<AlignmentMode> {
        return Ok(AlignmentMode::GermanPolar);
    }

    /// Returns the current equatorial coordinate system used by this telescope (e.g. Topocentric or J2000).
    pub async fn get_equatorial_system(&self) -> AscomResult<EquatorialCoordinateType> {
        Ok(EquatorialCoordinateType::Topocentric)
    }

    /// The telescope's effective aperture diameter (meters)
    pub async fn get_aperture(&self) -> AscomResult<f64> {
        self.telescope_details.aperture.ok_or(AscomError::from_msg(
            AscomErrorType::ValueNotSet,
            format!("Aperture not defined"),
        ))
    }

    /// The area of the telescope's aperture, taking into account any obstructions (square meters)
    pub async fn get_aperture_area(&self) -> AscomResult<f64> {
        self.telescope_details
            .aperture_area
            .ok_or(AscomError::from_msg(
                AscomErrorType::ValueNotSet,
                format!("Aperture area not defined"),
            ))
    }

    /// The telescope's focal length in meters
    pub async fn get_focal_length(&self) -> AscomResult<f64> {
        self.telescope_details
            .focal_length
            .ok_or(AscomError::from_msg(
                AscomErrorType::ValueNotSet,
                format!("Focal length not defined"),
            ))
    }

    /// True if the mount is stopped in the Home position. Set only following a FindHome() operation, and reset with any slew operation.
    /// This property must be False if the telescope does not support homing.
    pub async fn is_home(&self) -> AscomResult<bool> {
        Ok(false)
    }

    /// True if the telescope or driver applies atmospheric refraction to coordinates.
    pub async fn does_refraction(&self) -> AscomResult<bool> {
        Ok(false)
    }

    /// Tell the telescope or driver whether to apply atmospheric refraction to coordinates.
    pub async fn set_does_refraction(&self, _does_refraction: bool) -> AscomResult<()> {
        // TODO implement these?
        Err(AscomError::from_msg(
            AscomErrorType::ActionNotImplemented,
            format!("Refraction calculations not available"),
        ))
    }

    /// Indicates the pointing state of the mount
    pub async fn get_side_of_pier(&self) -> AscomResult<PierSide> {
        Ok(self.state.read().await.pier_side)
    }

    /// True if the SideOfPier property can be set, meaning that the mount can be forced to flip.
    pub async fn can_set_side_of_pier(&self) -> AscomResult<bool> {
        Ok(false) // FIXME revisit
    }

    /// Sets the pointing state of the mount
    pub async fn set_side_of_pier(&self, _side: PierSide) -> AscomResult<()> {
        Err(AscomError::from_msg(
            AscomErrorType::ActionNotImplemented,
            format!("Side of pier control not implemented"),
        ))
    }

    pub async fn is_connected(&self) -> AscomResult<bool> {
        let _state = self.state.read().await;
        let mut driver = self.driver.lock().unwrap();

        match driver.test_com() {
            Ok(()) => Ok(true),
            Err(SynScanError::CommunicationError(_)) => Ok(false),
            Err(e) => Err(AscomError::from(e)),
        }
    }
}
