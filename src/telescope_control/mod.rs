pub mod config;
mod driver;
pub mod guide;
pub mod observing_pos;
pub mod parking;
pub mod pointing_pos;
pub mod slew;
pub mod sync;
pub mod target;
pub(in crate::telescope_control) mod test_util;
pub mod tracking;

use crate::rotation_direction::RotationDirectionKey;
use crate::telescope_control::driver::{Driver, DriverBuilder, Status};
use crate::util::*;
use config::{Config, TelescopeDetails};
use std::sync::Arc;
use std::time::Duration;
use synscan::AutoGuideSpeed;
use target::Target;
use tokio::sync::{oneshot, RwLock};
use tokio::task;

type StateArc = Arc<RwLock<State>>;

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
    post_slew_settle_time: u32,
    rotation_direction_key: RotationDirectionKey,
    target: Target,
    motor_state: MotorState,
    autoguide_speed: AutoGuideSpeed,
}

pub struct StarAdventurer {
    driver: Driver,
    telescope_details: TelescopeDetails,
    state: StateArc,
}

impl StarAdventurer {
    pub async fn new(config: &Config) -> AscomResult<Self> {
        let mut db = DriverBuilder::new().with_timeout(Duration::from_millis(
            config.com_settings.timeout_millis as u64,
        ));

        if config.com_settings.path.is_some() {
            db = db.with_path(config.com_settings.path.clone().unwrap());
        }

        let driver = db.create();

        if let Err(e) = driver {
            // TODO Invalid value?
            return AscomResult::Err(AscomError::from_msg(AscomErrorType::InvalidValue, e));
        }

        let driver = driver.unwrap();

        driver.set_autoguide_speed(AutoGuideSpeed::Half).await?;

        let state = State {
            observation_location: config.observation_location,
            park_pos: 0.0, // TODO: Put an angular park pos in config (that only works after alignment)
            declination: 0.0, // Set only by sync or goto
            hour_angle_offset: 0.0, // Set only by sync or goto
            autoguide_speed: AutoGuideSpeed::Half, // Write only, so default to half b/c most standard
            pier_side: PierSide::East,             // TODO use this?
            date_offset: chrono::Duration::zero(), // Assume using computer time
            post_slew_settle_time: config.other_settings.slew_settle_time,
            rotation_direction_key: RotationDirectionKey::from_latitude(
                config.observation_location.latitude,
            ),
            target: Target::default(), // No target initially
            tracking_rate: TrackingRate::Sidereal,
            motor_state: MotorState::Stationary(StationaryState::Parked), // temporary value
        };

        let sa = StarAdventurer {
            driver,
            state: Arc::new(RwLock::new(state)),
            telescope_details: config.telescope_details,
        };

        sa.determine_state_from_driver().await?;

        Ok(sa)
    }

    pub async fn determine_state_from_driver(&self) -> AscomResult<()> {
        let mut state = self.state.write().await;
        let status = self.driver.get_status().await?;

        match status {
            Status::Slewing(target) => {
                let (cancel_tx, cancel_rx) = oneshot::channel();
                let goto_complete = self.driver.clone().track_goto();
                state.motor_state =
                    MotorState::Moving(MovingState::Slewing(SlewingState::Gotoing {
                        destination: target,
                        canceller: cancel_tx,
                        after_state: AfterSlewState::Stationary,
                    }));
                task::spawn(Self::complete_slew(
                    self.state.clone(),
                    self.driver.clone(),
                    cancel_rx,
                    goto_complete,
                ));
            }
            Status::Moving(direction) => {
                let tracking_rate = self.driver.determine_tracking_rate().await?;
                if direction == Self::get_tracking_direction(state.observation_location.latitude)
                    && tracking_rate.is_a()
                {
                    state.tracking_rate = tracking_rate.get_a().unwrap();
                    state.motor_state = MotorState::Moving(MovingState::Constant {
                        state: ConstantMotionState::Tracking,
                        guiding_state: GuidingState::Idle,
                        motion_rate: MotionRate::new(state.tracking_rate.into(), direction),
                    });
                } else {
                    let rate = tracking_rate
                        .get_b()
                        .unwrap_or_else(|| tracking_rate.get_a().unwrap().into());
                    state.motor_state = MotorState::Moving(MovingState::Constant {
                        state: ConstantMotionState::MoveAxis {
                            after_state: AfterSlewState::Stationary,
                        },
                        guiding_state: GuidingState::Idle,
                        motion_rate: MotionRate::new(rate, direction),
                    });
                }
            }
            Status::Stationary => {
                state.tracking_rate = self
                    .driver
                    .determine_tracking_rate()
                    .await?
                    .a_or(state.tracking_rate);
                state.motor_state =
                    MotorState::Stationary(StationaryState::Unparked(GuidingState::Idle));
            }
        };

        Ok(())
    }

    /// Returns the alignment mode of the mount (Alt/Az, Polar, German Polar)
    pub async fn get_alignment_mode(&self) -> AscomResult<AlignmentMode> {
        Ok(AlignmentMode::GermanPolar)
    }

    /// Returns the current equatorial coordinate system used by this telescope (e.g. Topocentric or J2000).
    pub async fn get_equatorial_system(&self) -> AscomResult<EquatorialCoordinateType> {
        Ok(EquatorialCoordinateType::Topocentric)
    }

    /// The telescope's effective aperture diameter (meters)
    pub async fn get_aperture(&self) -> AscomResult<f64> {
        self.telescope_details.aperture.ok_or_else(|| {
            AscomError::from_msg(
                AscomErrorType::ValueNotSet,
                "Aperture not defined".to_string(),
            )
        })
    }

    /// The area of the telescope's aperture, taking into account any obstructions (square meters)
    pub async fn get_aperture_area(&self) -> AscomResult<f64> {
        self.telescope_details.aperture_area.ok_or_else(|| {
            AscomError::from_msg(
                AscomErrorType::ValueNotSet,
                "Aperture area not defined".to_string(),
            )
        })
    }

    /// The telescope's focal length in meters
    pub async fn get_focal_length(&self) -> AscomResult<f64> {
        self.telescope_details.focal_length.ok_or_else(|| {
            AscomError::from_msg(
                AscomErrorType::ValueNotSet,
                "Focal length not defined".to_string(),
            )
        })
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
            AscomErrorType::PropertyOrMethodNotImplemented,
            "Refraction calculations not available".to_string(),
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
            AscomErrorType::PropertyOrMethodNotImplemented,
            "Side of pier control not implemented".to_string(),
        ))
    }
}
