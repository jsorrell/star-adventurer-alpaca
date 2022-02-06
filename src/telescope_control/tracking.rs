use crate::astro_math::Degrees;
use crate::telescope_control::{StarAdventurer, RA_CHANNEL};
use crate::util::enums::*;
use crate::util::result::*;
use synscan::motors::Direction;
use tokio::task;

impl StarAdventurer {
    /// True if the Tracking property can be changed, turning telescope sidereal tracking on and off.
    pub async fn can_set_tracking(&self) -> AscomResult<bool> {
        Ok(true)
    }

    #[inline]
    pub(crate) fn get_tracking_direction(latitude: Degrees) -> Direction {
        if Self::in_north(latitude) {
            Direction::Clockwise
        } else {
            Direction::CounterClockwise
        }
    }

    /// The right ascension tracking rate (arcseconds per second, default = 0.0)
    pub async fn get_ra_rate(&self) -> AscomResult<f64> {
        Ok(0.)
    }

    /// True if the RightAscensionRate property can be changed to provide offset tracking in the right ascension axis.
    pub async fn can_set_ra_rate(&self) -> AscomResult<bool> {
        Ok(false)
    }

    /// Sets the right ascension tracking rate (arcseconds per second)
    pub async fn set_ra_rate(&self, _rate: f64) -> AscomResult<()> {
        Err(AscomError::from_msg(
            AscomErrorType::InvalidOperation,
            "Setting RA tracking rate is not supported".to_string(),
        ))
    }

    /// The declination tracking rate (arcseconds per second, default = 0.0)
    pub async fn get_declination_rate(&self) -> AscomResult<f64> {
        Ok(0.)
    }

    /// True if the DeclinationRate property can be changed to provide offset tracking in the declination axis
    pub async fn can_set_declination_rate(&self) -> AscomResult<bool> {
        Ok(false)
    }

    /// Sets the declination tracking rate (arcseconds per second)
    pub async fn set_declination_rate(&self, _rate: f64) -> AscomResult<()> {
        Err(AscomError::from_msg(
            AscomErrorType::ActionNotImplemented,
            format!("Declination tracking not available"),
        ))
    }

    /// Returns an array of supported DriveRates values that describe the permissible values of the TrackingRate property for this telescope type.
    pub async fn get_tracking_rates(&self) -> AscomResult<Vec<TrackingRate>> {
        Ok(vec![
            TrackingRate::Sidereal,
            TrackingRate::Lunar,
            TrackingRate::Solar,
            TrackingRate::King,
        ])
    }

    /// The current tracking rate of the telescope's sidereal drive.
    pub async fn get_tracking_rate(&self) -> AscomResult<TrackingRate> {
        Ok(self.state.read().await.tracking_rate)
    }

    /// Sets the tracking rate of the telescope's sidereal drive
    pub async fn set_tracking_rate(&self, tracking_rate: TrackingRate) -> AscomResult<()> {
        let mut state = self.state.write().await;
        state.tracking_rate = tracking_rate;
        match &state.motion_state {
            MotionState::Tracking(TrackingState::Tracking(guiding)) => {
                match guiding {
                    Some(t) => t.send(true).unwrap(),
                    None => (),
                }
                let driver_clone = self.driver.clone();
                task::spawn_blocking(move || {
                    let mut driver = driver_clone.lock().unwrap();
                    driver.set_motion_rate_degrees(RA_CHANNEL, tracking_rate.as_deg(), false)?;
                    AscomResult::Ok(())
                })
                .await
                .unwrap()?;
            }
            _ => (),
        };

        Ok(())
    }

    /// Returns the state of the telescope's sidereal tracking drive.
    /// TODO is it tracking while goto? Going with no for now
    pub async fn is_tracking(&self) -> AscomResult<bool> {
        Ok(match self.state.read().await.motion_state {
            MotionState::Tracking(TrackingState::Tracking(_)) => true,
            _ => false,
        })
    }

    /// Sets the state of the telescope's sidereal tracking drive.
    /// TODO does setting tracking to true stop gotos?
    /// TODO Does it change what they'll do when the gotos are over?
    /// TODO Going with can only set it while not gotoing
    pub async fn set_is_tracking(&self, should_track: bool) -> AscomResult<()> {
        let mut state = self.state.write().await;
        match (&state.motion_state, should_track) {
            (MotionState::Tracking(TrackingState::Tracking(guiding)), false) => {
                match guiding {
                    Some(t) => t.send(true).unwrap(),
                    None => (),
                };

                let driver_clone = self.driver.clone();

                task::spawn_blocking(move || {
                    driver_clone.lock().unwrap().stop_motion(RA_CHANNEL, false)
                })
                .await
                .unwrap()?;

                state.motion_state = MotionState::Tracking(TrackingState::Stationary(false));
                Ok(())
            }
            (MotionState::Tracking(TrackingState::Stationary(false)), true) => {
                let tracking_rate = state.tracking_rate.as_deg();

                let driver_clone = self.driver.clone();
                task::spawn_blocking(move || {
                    let mut driver = driver_clone.lock().unwrap();
                    // direction and mode should already be set
                    driver.set_motion_rate_degrees(RA_CHANNEL, tracking_rate, false)?;
                    driver.start_motion(RA_CHANNEL)
                })
                .await
                .unwrap()?;
                state.motion_state = MotionState::Tracking(TrackingState::Tracking(None));
                Ok(())
            }
            (MotionState::Tracking(TrackingState::Stationary(true)), true) => {
                Err(AscomError::from_msg(
                    AscomErrorType::InvalidWhileParked,
                    "Invalid while parked".to_string(),
                ))
            }
            (MotionState::Slewing(_), true) => Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Invalid while slewing".to_string(),
            )),
            _ => Ok(()),
        }
    }
}
