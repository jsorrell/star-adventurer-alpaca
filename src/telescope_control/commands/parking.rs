use crate::astro_math;
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
        self.connection.is_parked().await
    }

    /// True if this telescope is capable of programmed setting of its park position (SetPark() method)
    pub async fn can_set_park_pos(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Sets the telescope's park position to be its current position.
    pub async fn set_park_pos(&self) -> AscomResult<()> {
        let motor_pos = self.connection.get_pos().await?;
        *self.settings.park_pos.write().await = astro_math::modulo(motor_pos, 360.);
        Ok(())
    }

    /// Move the telescope to its park position, stop all motion (or restrict to a small safe range), and set AtPark to True.
    pub async fn park(&self) -> AscomResult<()> {
        let park_pos = *self.settings.park_pos.read().await;
        let _completed = self.connection.park(park_pos).await?.await.unwrap()?;
        Ok(())
    }

    /// Takes telescope out of the Parked state.
    pub async fn unpark(&self) -> AscomResult<()> {
        self.connection.unpark().await?;
        Ok(())
    }
}
