use crate::astro_math;
use crate::rotation_direction::RotationDirection;
use crate::telescope_control::slew_def::Slew;
use crate::telescope_control::StarAdventurer;
use crate::util::*;
use ascom_alpaca::ASCOMResult;
use tokio::join;

impl StarAdventurer {
    /// True if this telescope is capable of programmed parking (Park() method)
    pub async fn can_park(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    /// True if this telescope is capable of programmed unparking (UnPark() method)
    pub async fn can_unpark(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    /// True if the telescope has been put into the parked state by the seee Park() method.
    /// Set False by calling the Unpark() method.
    pub async fn is_parked(&self) -> ASCOMResult<bool> {
        self.connection.is_parked().await
    }

    /// True if this telescope is capable of programmed setting of its park position (SetPark() method)
    pub async fn can_set_park_pos(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    /// Sets the telescope's park position to be its current position.
    pub async fn set_park_pos(&self) -> ASCOMResult<()> {
        *self.settings.park_ha.write().await = self.get_mech_ha().await?;
        Ok(())
    }

    /// Move the telescope to its park position, stop all motion, and set AtPark to True.
    pub async fn park(&self) -> ASCOMResult<()> {
        let current_motor_pos = self.connection.get_pos().await?;

        let (park_ha, key, mech_ha_offset, mount_limits) = join!(
            async { *self.settings.park_ha.read().await },
            async {
                self.settings
                    .observation_location
                    .read()
                    .await
                    .get_rotation_direction_key()
            },
            async { *self.settings.mech_ha_offset.read().await },
            async { *self.settings.mount_limits.read().await },
        );

        let current_mech_ha = Self::calc_mech_ha(current_motor_pos, mech_ha_offset, key);

        let slew = Slew::to_mech_ha(current_mech_ha, park_ha, mount_limits);
        let motor_direction = MotorEncodingDirection::from(slew.direction().using(key));
        let pos_change = astro_math::hours_to_deg(slew.distance()) * motor_direction.get_sign_f64();
        let dest_motor_pos = current_motor_pos + pos_change;

        let _completed = self.connection.park(dest_motor_pos).await?.await.unwrap()?;
        Ok(())
    }

    /// Takes telescope out of the Parked state.
    pub async fn unpark(&self) -> ASCOMResult<()> {
        self.connection.unpark().await?;
        Ok(())
    }
}
