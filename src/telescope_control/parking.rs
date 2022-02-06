use crate::telescope_control::{StarAdventurer, RA_CHANNEL};
use crate::util::enums::*;
use crate::util::result::{AscomError, AscomErrorType, AscomResult};

impl StarAdventurer {
    /// True if this telescope is capable of programmed parking (Park() method)
    pub async fn can_park(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// True if this telescope is capable of programmed unparking (UnPark() method)
    pub async fn can_unpark(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// True if the telescope has been put into the parked state by the seee Park() method.
    /// Set False by calling the Unpark() method.
    pub async fn is_parked(&self) -> AscomResult<bool> {
        Ok(match self.state.read().await.motion_state {
            MotionState::Tracking(TrackingState::Stationary(true)) => true,
            _ => false,
        })
    }

    /// True if this telescope is capable of programmed setting of its park position (SetPark() method)
    pub async fn can_set_park_pos(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Sets the telescope's park position to be its current position.
    pub async fn set_park_pos(&self) -> AscomResult<()> {
        let mut state = self.state.write().await;
        let mut driver = self.driver.lock().unwrap();
        state.park_pos = driver.get_pos(RA_CHANNEL)?;
        Ok(())
    }

    /// Move the telescope to its park position, stop all motion (or restrict to a small safe range), and set AtPark to True.
    pub async fn park(&self) -> AscomResult<()> {
        let waiter = {
            let mut state = self.state.write().await;
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
            )?
        };
        waiter.await.unwrap()
    }

    /// Takes telescope out of the Parked state.
    pub async fn unpark(&self) -> AscomResult<()> {
        let mut state = self.state.write().await;
        match state.motion_state {
            MotionState::Tracking(TrackingState::Stationary(true)) => {
                state.motion_state = MotionState::Tracking(TrackingState::Stationary(false));
                Ok(())
            }
            _ => Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Not parked".to_string(),
            )),
        }
    }
}
