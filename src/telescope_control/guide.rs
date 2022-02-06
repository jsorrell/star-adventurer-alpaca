use crate::astro_math::Degrees;
use crate::telescope_control::{StarAdventurer, RA_CHANNEL};
use crate::util::enums::*;
use crate::util::result::{AscomError, AscomErrorType, AscomResult};
use std::time::Duration;
use synscan::motors::DriveMode;
use synscan::util::AutoGuideSpeed;
use tokio::sync::watch;
use tokio::task;
use tokio::time::sleep;

impl StarAdventurer {
    /// True if the guide rate properties used for PulseGuide(GuideDirections, Int32) can ba adjusted.
    pub async fn can_set_guide_rates(&self) -> AscomResult<bool> {
        Ok(false)
    }

    /// The current Declination movement rate offset for telescope guiding (degrees/sec)
    pub async fn get_guide_rate_declination(&self) -> AscomResult<Degrees> {
        Ok(0.)
    }

    /// Sets the current Declination movement rate offset for telescope guiding (degrees/sec).
    pub async fn set_guide_rate_declination(&self, _rate: Degrees) -> AscomResult<()> {
        Err(AscomError::from_msg(
            AscomErrorType::ActionNotImplemented,
            format!("Declination tracking not available"),
        ))
    }

    #[inline]
    fn calc_guide_rate(autoguide_speed: AutoGuideSpeed, tracking_rate: TrackingRate) -> Degrees {
        autoguide_speed.multiplier() * tracking_rate.as_deg()
    }

    /// The current RightAscension movement rate offset for telescope guiding (degrees/sec)
    pub async fn get_guide_rate_ra(&self) -> AscomResult<Degrees> {
        let state = self.state.read().await;
        Ok(Self::calc_guide_rate(
            state.autoguide_speed,
            state.tracking_rate,
        ))
    }

    /// Sets the current RightAscension movement rate offset for telescope guiding (degrees/sec).
    pub async fn set_guide_rate_ra(&self, rate: Degrees) -> AscomResult<()> {
        let mut state = self.state.write().await;
        let lowest_guide_rate = AutoGuideSpeed::Eighth.multiplier() * state.tracking_rate.as_deg();
        let highest_guide_rate = AutoGuideSpeed::One.multiplier() * state.tracking_rate.as_deg();
        if rate < lowest_guide_rate * 0.9 || highest_guide_rate * 1.1 < rate {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidValue,
                format!(
                    "Guide rate must be between {} and {}",
                    lowest_guide_rate, highest_guide_rate
                ),
            ));
        }

        let (best_speed, _distance) = [
            AutoGuideSpeed::Eighth,
            AutoGuideSpeed::Quarter,
            AutoGuideSpeed::Half,
            AutoGuideSpeed::ThreeQuarters,
            AutoGuideSpeed::One,
        ]
        .iter()
        .fold(
            (&AutoGuideSpeed::Eighth, 99999.),
            |(closest, distance), try_speed| {
                let try_distance =
                    (try_speed.multiplier() * state.tracking_rate.as_deg() - rate).abs();
                if try_distance < distance {
                    (try_speed, try_distance)
                } else {
                    (closest, distance)
                }
            },
        );
        state.autoguide_speed = *best_speed;

        let driver_clone = self.driver.clone();

        task::spawn_blocking(move || {
            let mut driver = driver_clone.lock().unwrap();
            driver.set_autoguide_speed(RA_CHANNEL, *best_speed)
        })
        .await
        .unwrap()?;

        Ok(())
    }

    /// returns the tracking rate in degrees with guiding applied
    #[inline]
    fn tracking_rate_with_guiding(
        tracking_rate: Degrees,
        guide_speed: AutoGuideSpeed,
        guide_direction: GuideDirection,
    ) -> Degrees {
        if guide_direction == GuideDirection::North || guide_direction == GuideDirection::South {
            panic!("Tried to guide North or South")
        }

        if guide_direction == GuideDirection::West {
            tracking_rate * (1. + guide_speed.multiplier())
        } else {
            tracking_rate * (1. - guide_speed.multiplier())
        }
    }

    /// True if this telescope is capable of software-pulsed guiding (via the PulseGuide(GuideDirections, Int32) method)
    pub async fn can_pulse_guide(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Moves the scope in the given direction for the given interval or time at the rate given by the corresponding guide rate property
    pub async fn pulse_guide(
        &self,
        guide_direction: GuideDirection,
        duration: u32,
    ) -> AscomResult<()> {
        if guide_direction == GuideDirection::North || guide_direction == GuideDirection::South {
            return Err(AscomError::from_msg(
                AscomErrorType::ActionNotImplemented,
                "Can't guide in declination".to_string(),
            ));
        }

        let mut state = self.state.write().await;
        let (tracking_rate, state_to_restore) = match &state.motion_state {
            MotionState::Slewing(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidOperation,
                    "Can't guide while slewing".to_string(),
                ))
            }
            MotionState::Tracking(TrackingState::Stationary(true)) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidWhileParked,
                    "Can't guide while parked".to_string(),
                ))
            }
            MotionState::Tracking(TrackingState::Tracking(Some(_))) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidOperation,
                    "Already guiding".to_string(),
                ))
            }
            MotionState::Tracking(TrackingState::Stationary(false)) => {
                (0., TrackingState::Stationary(false))
            }
            MotionState::Tracking(TrackingState::Tracking(None)) => {
                (state.tracking_rate.as_deg(), TrackingState::Tracking(None))
            }
        };

        let new_motion_rate =
            Self::tracking_rate_with_guiding(tracking_rate, state.autoguide_speed, guide_direction);

        let driver_clone = self.driver.clone();
        task::spawn_blocking(move || {
            let mut driver = driver_clone.lock().unwrap();
            driver.set_motion_rate_degrees(RA_CHANNEL, new_motion_rate, false)
        })
        .await
        .unwrap()?;

        let state_clone = self.state.clone();
        let driver_clone = self.driver.clone();

        let (cancel_tx, mut cancel_rx) = watch::channel(false);

        /* Start task that will stop guiding when it's done */
        let _guide_task = task::spawn(async move {
            tokio::select! {
                _ = sleep(Duration::from_millis(duration as u64)) => {
                    let mut state = state_clone.write().await;

                    let latitude = state.observation_location.latitude;
                    let tracking_rate = state.tracking_rate.as_deg();
                    let running = match state_to_restore {
                        TrackingState::Tracking(_) => true,
                        TrackingState::Stationary(_) => false,
                    };

                    task::spawn_blocking(move || {
                        let mut driver = driver_clone.lock().unwrap();
                        driver.set_motion_mode(
                            RA_CHANNEL,
                            DriveMode::Tracking,
                            false,
                            Self::get_tracking_direction(latitude),
                        )?;
                        driver.set_motion_rate_degrees(RA_CHANNEL, tracking_rate, false)?;

                        if running {
                            driver.start_motion(RA_CHANNEL)
                        } else {
                            driver.stop_motion(RA_CHANNEL, false)
                        }
                    })
                    .await
                    .unwrap()?;

                    state.motion_state = MotionState::Tracking(state_to_restore);

                    Ok(())
                },
                _ = cancel_rx.changed() => Err(AscomError::from_msg(AscomErrorType::InvalidOperation, "Cancelled".to_string())),
            }
        });

        state.motion_state = MotionState::Tracking(TrackingState::Tracking(Some(cancel_tx)));
        Ok(())
    }

    /// True if a PulseGuide(GuideDirections, Int32) command is in progress, False otherwise
    pub async fn is_pulse_guiding(&self) -> AscomResult<bool> {
        let state = self.state.read().await;
        Ok(match &state.motion_state {
            MotionState::Tracking(TrackingState::Tracking(guide_task)) => guide_task.is_some(),
            _ => false,
        })
    }
}
