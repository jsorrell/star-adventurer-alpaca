use crate::astro_math::Degrees;
use crate::errors::{AlpacaError, ErrorType, Result};
use crate::{MotionState, StarAdventurer, State, TrackingRate, TrackingState, RA_CHANNEL};
use std::sync::MutexGuard;
use synscan::motors::{Direction, DriveMode};
use synscan::MotorController;

impl StarAdventurer {
    /// True if the Tracking property can be changed, turning telescope sidereal tracking on and off.
    pub fn can_set_tracking(&self) -> Result<bool> {
        Ok(true)
    }

    #[inline]
    pub(crate) fn get_tracking_direction(latitude: Degrees) -> Direction {
        if 0. <= latitude {
            Direction::Clockwise
        } else {
            Direction::CounterClockwise
        }
    }

    /// The right ascension tracking rate (arcseconds per second, default = 0.0)
    pub fn get_ra_rate(&mut self) -> Result<f64> {
        Ok(0.)
    }

    /// True if the RightAscensionRate property can be changed to provide offset tracking in the right ascension axis.
    pub fn can_set_ra_rate(&self) -> Result<bool> {
        Ok(false)
    }

    /// Sets the right ascension tracking rate (arcseconds per second)
    pub fn set_ra_rate(&mut self, rate: f64) -> Result<()> {
        todo!();
        Err(AlpacaError::from_msg(
            ErrorType::InvalidOperation,
            "Setting RA tracking rate is not supported".to_string(),
        ))
    }

    /// The declination tracking rate (arcseconds per second, default = 0.0)
    pub fn get_declination_rate(&self) -> Result<f64> {
        Ok(0.)
    }

    /// True if the DeclinationRate property can be changed to provide offset tracking in the declination axis
    pub fn can_set_declination_rate(&self) -> Result<bool> {
        Ok(false)
    }

    /// Sets the declination tracking rate (arcseconds per second)
    pub fn set_declination_rate(&self, _rate: f64) -> Result<()> {
        Err(AlpacaError::from_msg(
            ErrorType::ActionNotImplemented,
            format!("Declination tracking not available"),
        ))
    }

    /// Returns an array of supported DriveRates values that describe the permissible values of the TrackingRate property for this telescope type.
    pub fn get_tracking_rates(&self) -> Result<Vec<TrackingRate>> {
        Ok(vec![
            TrackingRate::Sidereal,
            TrackingRate::Lunar,
            TrackingRate::Solar,
            TrackingRate::King,
        ])
    }

    /// The current tracking rate of the telescope's sidereal drive.
    pub fn get_tracking_rate(&mut self) -> Result<TrackingRate> {
        Ok(self.state.read().unwrap().tracking_rate)
    }

    /// Sets the tracking rate of the telescope's sidereal drive
    pub fn set_tracking_rate(&mut self, tracking_rate: TrackingRate) -> Result<()> {
        let mut state = self.state.write().unwrap();
        state.tracking_rate = tracking_rate;
        match &state.motion_state {
            MotionState::Tracking(TrackingState::Tracking(guiding)) => {
                if guiding.is_some() {
                    // TODO stop guiding
                }
                let mut driver = self.driver.lock().unwrap();
                driver.set_motion_rate_degrees(RA_CHANNEL, tracking_rate.as_deg(), false)?;
            }
            _ => (),
        };

        Ok(())
    }

    /// Returns the state of the telescope's sidereal tracking drive.
    /// TODO is it tracking while goto? Going with no for now
    pub fn is_tracking(&self) -> Result<bool> {
        Ok(match self.state.read().unwrap().motion_state {
            MotionState::Tracking(TrackingState::Tracking(_)) => true,
            _ => false,
        })
    }

    /// Sets the state of the telescope's sidereal tracking drive.
    /// TODO does setting tracking to true stop gotos?
    /// TODO Does it change what they'll do when the gotos are over?
    /// TODO Going with can only set it while not gotoing
    pub fn set_is_tracking(&mut self, should_track: bool) -> Result<()> {
        let mut state = self.state.write().unwrap();
        match (&state.motion_state, should_track) {
            (MotionState::Tracking(TrackingState::Tracking(guiding)), false) => {
                // TODO stop guider thread
                if guiding.is_some() {
                    todo!();
                }
                self.driver.lock().unwrap().stop_motion(RA_CHANNEL, false)?;
                state.motion_state = MotionState::Tracking(TrackingState::Stationary(false));
                Ok(())
            }
            (MotionState::Tracking(TrackingState::Stationary(false)), true) => {
                let tracking_rate = state.tracking_rate.as_deg();
                let mut driver = self.driver.lock().unwrap();
                // direction and mode should already be set
                driver.set_motion_rate_degrees(RA_CHANNEL, tracking_rate, false)?;
                driver.start_motion(RA_CHANNEL)?;
                state.motion_state = MotionState::Tracking(TrackingState::Tracking(None));
                Ok(())
            }
            (MotionState::Tracking(TrackingState::Stationary(true)), true) => {
                Err(AlpacaError::from_msg(
                    ErrorType::InvalidWhileParked,
                    "Invalid while parked".to_string(),
                ))
            }
            (MotionState::Slewing(_), true) => Err(AlpacaError::from_msg(
                ErrorType::InvalidOperation,
                "Invalid while slewing".to_string(),
            )),
            _ => Ok(()),
        }
    }

    pub(crate) fn restore_tracking_state(
        driver: &mut MutexGuard<MotorController>,
        state: &mut std::sync::RwLockWriteGuard<'_, State>,
        state_to_restore: TrackingState,
    ) -> Result<()> {
        driver.set_motion_mode(
            RA_CHANNEL,
            DriveMode::Tracking,
            false,
            Self::get_tracking_direction(state.latitude),
        )?;
        driver.set_motion_rate_degrees(RA_CHANNEL, state.tracking_rate.as_deg(), false)?;

        match state_to_restore {
            TrackingState::Tracking(_) => driver.start_motion(RA_CHANNEL)?,
            TrackingState::Stationary(_) => driver.stop_motion(RA_CHANNEL, false)?,
        };

        state.motion_state = MotionState::Tracking(state_to_restore);

        Ok(())
    }
}
