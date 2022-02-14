use crate::astro_math;
use crate::telescope_control::slew::CompletionResult;
use crate::telescope_control::StarAdventurer;
use crate::util::*;

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
        Ok(self.state.read().await.motor_state.is_parked())
    }

    /// True if this telescope is capable of programmed setting of its park position (SetPark() method)
    pub async fn can_set_park_pos(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Sets the telescope's park position to be its current position.
    pub async fn set_park_pos(&self) -> AscomResult<()> {
        let mut state = self.state.write().await;

        let motor_pos = self.driver.get_pos().await?;

        state.park_pos = astro_math::modulo(motor_pos, 360.);
        Ok(())
    }

    /// Move the telescope to its park position, stop all motion (or restrict to a small safe range), and set AtPark to True.
    pub async fn park(&self) -> AscomResult<()> {
        let mut state = self.state.write().await;
        // Already parked
        if state.motor_state.is_parked() {
            return Ok(());
        }

        if state.motor_state.is_slewing() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Can't park while slewing".to_string(),
            ));
        }

        // See if already in park position
        if matches!(
            state.motor_state,
            MotorState::Stationary(StationaryState::Unparked(_))
        ) {
            let current_pos = self.driver.get_pos().await?;
            if current_pos == state.park_pos {
                state.motor_state = MotorState::Stationary(StationaryState::Parked);
                return Ok(());
            }
        }

        let park_pos = state.park_pos;

        let slew_task = Self::slew_motor_to_angle(
            self.state.clone(),
            &mut state,
            self.driver.clone(),
            park_pos,
            AfterSlewState::Parked,
        )
        .await?;

        std::mem::drop(state);

        match slew_task.await {
            CompletionResult::Completed(()) => Ok(()),
            CompletionResult::Cancelled => Ok(()), // TODO Should this be success or failure?
        }
    }

    /// Takes telescope out of the Parked state.
    pub async fn unpark(&self) -> AscomResult<()> {
        let mut state = self.state.write().await;
        state.motor_state.unpark();
        Ok(())
    }
}
