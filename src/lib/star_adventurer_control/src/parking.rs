use crate::errors::{AlpacaError, ErrorType, Result};
use crate::{MotionState, StarAdventurer, TrackingState, RA_CHANNEL};

impl StarAdventurer {
    /// True if this telescope is capable of programmed parking (Park() method)
    pub fn can_park(&self) -> Result<bool> {
        Ok(true)
    }

    /// True if this telescope is capable of programmed unparking (UnPark() method)
    pub fn can_unpark(&self) -> Result<bool> {
        Ok(true)
    }

    /// True if the telescope has been put into the parked state by the seee Park() method.
    /// Set False by calling the Unpark() method.
    pub fn is_parked(&self) -> Result<bool> {
        Ok(match self.state.read().unwrap().motion_state {
            MotionState::Tracking(TrackingState::Stationary(true)) => true,
            _ => false,
        })
    }

    /// True if this telescope is capable of programmed setting of its park position (SetPark() method)
    pub fn can_set_park_pos(&self) -> Result<bool> {
        Ok(true)
    }

    /// Sets the telescope's park position to be its current position.
    pub fn set_park_pos(&mut self) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let mut driver = self.driver.lock().unwrap();
        state.park_pos = driver.get_pos(RA_CHANNEL)?;
        Ok(())
    }

    /// Move the telescope to its park position, stop all motion (or restrict to a small safe range), and set AtPark to True.
    pub fn park(&mut self) -> Result<()> {
        let mut state = self.state.write().unwrap();
        Self::check_current_state_for_slewing(&state.motion_state)?;
        let mut driver = self.driver.lock().unwrap();
        let park_pos = state.park_pos;

        Self::slew_motor_to_angle(
            &self.state,
            &mut state,
            &self.driver,
            &mut driver,
            park_pos,
            TrackingState::Stationary(true),
        )
    }

    /// Takes telescope out of the Parked state.
    pub fn unpark(&mut self) -> Result<()> {
        let mut state = self.state.write().unwrap();
        match state.motion_state {
            MotionState::Tracking(TrackingState::Stationary(true)) => {
                state.motion_state = MotionState::Tracking(TrackingState::Stationary(false));
                Ok(())
            }
            _ => Err(AlpacaError::from_msg(
                ErrorType::InvalidOperation,
                "Not parked".to_string(),
            )),
        }
    }
}
