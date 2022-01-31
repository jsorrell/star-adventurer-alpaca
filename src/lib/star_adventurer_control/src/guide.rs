use crate::astro_math::Degrees;
use crate::enums::*;
use crate::errors::{AlpacaError, ErrorType, Result};
use crate::{StarAdventurer, RA_CHANNEL};
use std::sync::Arc;
use std::time::Duration;
use synscan::util::AutoGuideSpeed;
use tokio::task;
use tokio::time::sleep;

impl StarAdventurer {
    /// True if the guide rate properties used for PulseGuide(GuideDirections, Int32) can ba adjusted.
    pub fn can_set_guide_rates(&self) -> Result<bool> {
        Ok(false)
    }

    /// The current Declination movement rate offset for telescope guiding (degrees/sec)
    pub fn get_guide_rate_declination(&self) -> Result<Degrees> {
        Ok(0.)
    }

    /// Sets the current Declination movement rate offset for telescope guiding (degrees/sec).
    pub fn set_guide_rate_declination(&self, _rate: Degrees) -> Result<()> {
        Err(AlpacaError::from_msg(
            ErrorType::ActionNotImplemented,
            format!("Declination tracking not available"),
        ))
    }

    #[inline]
    fn calc_guide_rate(autoguide_speed: AutoGuideSpeed, tracking_rate: TrackingRate) -> Degrees {
        autoguide_speed.multiplier() * tracking_rate.as_deg()
    }

    /// The current RightAscension movement rate offset for telescope guiding (degrees/sec)
    pub fn get_guide_rate_ra(&self) -> Result<Degrees> {
        let state = self.state.read().unwrap();
        Ok(Self::calc_guide_rate(
            state.autoguide_speed,
            state.tracking_rate,
        ))
    }

    /// Sets the current RightAscension movement rate offset for telescope guiding (degrees/sec).
    pub async fn set_guide_rate_ra(&mut self, rate: Degrees) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let lowest_guide_rate = AutoGuideSpeed::Eighth.multiplier() * state.tracking_rate.as_deg();
        let highest_guide_rate = AutoGuideSpeed::One.multiplier() * state.tracking_rate.as_deg();
        if rate < lowest_guide_rate * 0.9 || highest_guide_rate * 1.1 < rate {
            return Err(AlpacaError::from_msg(
                ErrorType::InvalidValue,
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

        let mut driver = self.driver.lock().unwrap();
        driver.set_autoguide_speed(RA_CHANNEL, *best_speed)?;
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
    pub fn can_pulse_guide(&self) -> Result<bool> {
        Ok(true)
    }

    /// Moves the scope in the given direction for the given interval or time at the rate given by the corresponding guide rate property
    pub fn pulse_guide(&mut self, guide_direction: GuideDirection, duration: u32) -> Result<()> {
        if guide_direction == GuideDirection::North || guide_direction == GuideDirection::South {
            return Err(AlpacaError::from_msg(
                ErrorType::ActionNotImplemented,
                "Can't guide in declination".to_string(),
            ));
        }

        let mut state = self.state.write().unwrap();
        let mut driver = self.driver.lock().unwrap();

        let state_to_restore = {
            let (tracking_rate, state_to_restore) = match &state.motion_state {
                MotionState::Slewing(_) => {
                    return Err(AlpacaError::from_msg(
                        ErrorType::InvalidOperation,
                        "Can't guide while slewing".to_string(),
                    ))
                }
                MotionState::Tracking(TrackingState::Stationary(true)) => {
                    return Err(AlpacaError::from_msg(
                        ErrorType::InvalidWhileParked,
                        "Can't guide while parked".to_string(),
                    ))
                }
                MotionState::Tracking(TrackingState::Tracking(Some(_))) => {
                    return Err(AlpacaError::from_msg(
                        ErrorType::InvalidOperation,
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

            let new_motion_rate = Self::tracking_rate_with_guiding(
                tracking_rate,
                state.autoguide_speed,
                guide_direction,
            );
            driver.set_motion_rate_degrees(RA_CHANNEL, new_motion_rate, false)?;
            state_to_restore
        };

        let state_arc = Arc::clone(&self.state);
        let driver_arc = Arc::clone(&self.driver);

        let guide_task = task::spawn(async move {
            sleep(Duration::from_millis(duration as u64)).await;

            let mut state = state_arc.write().unwrap();
            let mut driver = driver_arc.lock().unwrap();
            Self::restore_tracking_state(&mut driver, &mut state, state_to_restore)?;
            Ok(())
        });

        state.motion_state = MotionState::Tracking(TrackingState::Tracking(Some(guide_task)));
        Ok(())
    }

    /// True if a PulseGuide(GuideDirections, Int32) command is in progress, False otherwise
    pub fn is_pulse_guiding(&self) -> Result<bool> {
        let state = self.state.read().unwrap();
        Ok(match &state.motion_state {
            MotionState::Tracking(TrackingState::Tracking(guide_task)) => guide_task.is_some(),
            _ => false,
        })
    }
}
