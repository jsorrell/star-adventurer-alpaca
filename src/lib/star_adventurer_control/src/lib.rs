extern crate core;
#[macro_use]
extern crate assert_float_eq;

mod astro_math;
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
use crate::enums::{
    AlignmentMode, EquatorialCoordinateType, MotionState, PierSide, SlewingState, Target,
    TrackingRate, TrackingState,
};
use crate::errors::{AlpacaError, ErrorType, Result};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use synscan;
use synscan::motors::{Direction, DriveMode};
use synscan::util::{AutoGuideSpeed, SingleChannel};
use synscan::MotorParameters;

const RA_CHANNEL: &SingleChannel = &SingleChannel::Channel1;

pub struct Config {
    latitude: Degrees,
    longitude: Degrees,
    elevation: f64,
    aperture: Option<f64>,
    aperture_area: Option<f64>,
    focal_length: Option<f64>,
}

#[derive(Debug)]
struct State {
    latitude: Degrees,
    longitude: Degrees,
    elevation: f64,
    park_pos: Degrees,
    declination: Degrees,
    hour_angle_offset: Hours, // pos+offset = ha
    pier_side: PierSide,
    date_offset: chrono::Duration,
    tracking_rate: TrackingRate,
    post_slew_settle_time: f64,
    target: Target,
    motion_state: MotionState,
    autoguide_speed: AutoGuideSpeed,
}

pub struct StarAdventurer {
    driver: Arc<Mutex<synscan::MotorController<'static>>>,
    motor_params: MotorParameters,
    config: Config,
    state: Arc<RwLock<State>>,
}

impl StarAdventurer {
    pub fn new(
        path: &'static str,
        baud_rate: u32,
        timeout: Duration,
        config: Config,
    ) -> Result<Self> {
        let mut driver = synscan::MotorController::new(path, baud_rate, timeout)?;
        let motor_params = *driver.get_motor_parameters();
        let status = driver.get_status(RA_CHANNEL)?;
        let step_period = driver.get_step_period(RA_CHANNEL)?;

        let mut tracking_rate = TrackingRate::Sidereal;
        let motion_state: MotionState;

        match status.mode {
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
                    motion_state = MotionState::Tracking(TrackingState::Tracking(None))
                } else {
                    motion_state = MotionState::Tracking(TrackingState::Stationary(false))
                }
            }
            DriveMode::Goto => {
                if status.running {
                    let target = driver.get_goto_target(RA_CHANNEL)?;
                    // TODO spawn task to track goto
                    motion_state = MotionState::Slewing(SlewingState::GotoSlewing(
                        target,
                        TrackingState::Stationary(false),
                        todo!(),
                    ))
                } else {
                    motion_state = MotionState::Tracking(TrackingState::Stationary(false));
                    let direction = if 0. < config.latitude {
                        Direction::Clockwise
                    } else {
                        Direction::CounterClockwise
                    };
                    driver.set_motion_mode(RA_CHANNEL, DriveMode::Tracking, false, direction)?;
                    driver.set_step_period(
                        RA_CHANNEL,
                        tracking_rate.determine_step_period(
                            motor_params.get_timer_interrupt_frequency(),
                            motor_params.get_counts_per_revolution(*RA_CHANNEL),
                        ),
                    )?;
                }
            }
        }

        driver.set_autoguide_speed(RA_CHANNEL, AutoGuideSpeed::Half)?;

        let state = State {
            latitude: config.latitude,
            longitude: config.longitude,
            elevation: config.elevation,
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
        Ok(StarAdventurer {
            driver: Arc::new(Mutex::new(driver)),
            config,
            state: Arc::new(RwLock::new(state)),
            motor_params,
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
        self.config.aperture.ok_or(AlpacaError::from_msg(
            ErrorType::ValueNotSet,
            format!("Aperture not defined"),
        ))
    }

    /// The area of the telescope's aperture, taking into account any obstructions (square meters)
    pub fn get_aperture_area(&self) -> Result<f64> {
        self.config.aperture_area.ok_or(AlpacaError::from_msg(
            ErrorType::ValueNotSet,
            format!("Aperture area not defined"),
        ))
    }

    /// The telescope's focal length in meters
    pub fn get_focal_length(&self) -> Result<f64> {
        self.config.focal_length.ok_or(AlpacaError::from_msg(
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
    pub fn get_side_of_pier(&self) -> Result<PierSide> {
        Ok(self.state.read().unwrap().pier_side)
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    // fn print_status(sa: &mut StarAdventurer) {
    //     let status = sa.driver.get_status(&SingleChannel::Channel1).unwrap();
    //     let motion_rate = sa.driver.get_motion_rate_degrees(&SingleChannel::Channel1).unwrap();
    //     println!("--------------------------\n{}\nDegrees Per Sec: {}\n--------------------------\n", status, motion_rate);
    // }

    fn create_sa(config: Option<Config>) -> StarAdventurer {
        let config = config.unwrap_or(Config {
            latitude: ***REMOVED***,
            longitude: ***REMOVED***,
            elevation: ***REMOVED***,
            aperture: ***REMOVED***,
            aperture_area: ***REMOVED***,
            focal_length: ***REMOVED***,
        });

        StarAdventurer::new("/dev/ttyUSB0", 115_200, Duration::from_millis(250), config).unwrap()
    }

    #[test]
    fn test_date() {
        let mut sa = create_sa(None);

        let test_date = Utc.ymd(2222, 01, 01).and_hms(10, 00, 00);
        sa.set_utc_date(test_date).unwrap();
        assert!(sa.get_utc_date().unwrap() - test_date < chrono::Duration::milliseconds(1));
        std::thread::sleep(Duration::from_millis(1000));
        assert!(
            sa.get_utc_date().unwrap() - test_date - chrono::Duration::milliseconds(1000)
                < chrono::Duration::milliseconds(5)
        );
    }

    #[test]
    fn test_observing_location() {
        let mut sa = create_sa(None);

        let test_lat0 = 59.8843434;
        let test_lat1 = -33.;

        let test_long = 77.;

        let test_elevation = 999.;

        sa.set_latitude(test_lat0).unwrap();
        assert_eq!(sa.get_latitude().unwrap(), test_lat0);

        sa.set_longitude(test_long).unwrap();
        assert_eq!(sa.get_longitude().unwrap(), test_long);
        assert_eq!(sa.get_latitude().unwrap(), test_lat0);

        sa.set_elevation(test_elevation).unwrap();
        assert_eq!(sa.get_longitude().unwrap(), test_long);
        assert_eq!(sa.get_latitude().unwrap(), test_lat0);
        assert_eq!(sa.get_elevation().unwrap(), test_elevation);

        sa.set_latitude(test_lat1).unwrap();
        assert_eq!(sa.get_longitude().unwrap(), test_long);
        assert_eq!(sa.get_latitude().unwrap(), test_lat1);
        assert_eq!(sa.get_elevation().unwrap(), test_elevation);
    }

    #[test]
    fn test_slew() {}
}
