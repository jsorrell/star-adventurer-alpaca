use crate::astro_math::{Degrees, Hours};
use crate::errors::Result;
use crate::{astro_math, StarAdventurer, RA_CHANNEL};
use std::sync::{Arc, Mutex};
use synscan::MotorController;
use tokio::task;

impl StarAdventurer {
    /// True if this telescope is capable of programmed synching to equatorial coordinates.
    pub fn can_sync(&self) -> Result<bool> {
        Ok(true)
    }

    pub async fn get_hour_angle_offset(
        hour_angle: Hours,
        driver: &Arc<Mutex<MotorController<'static>>>,
    ) -> Result<Hours> {
        let driver_clone = driver.clone();
        let driver_pos = task::spawn_blocking(move || {
            let mut driver = driver_clone.lock().unwrap();
            driver.get_pos(RA_CHANNEL)
        })
        .await
        .unwrap()? as Degrees;
        Ok(hour_angle - astro_math::deg_to_hours(driver_pos))
    }

    /// Matches the scope's equatorial coordinates to the given equatorial coordinates.
    pub async fn sync_to_coordinates(&mut self, ra: Hours, dec: Degrees) -> Result<()> {
        let mut state = self.state.write().await;

        let hour_angle = astro_math::calculate_hour_angle(
            Self::calculate_utc_date(state.date_offset),
            state.observation_location.longitude,
            ra,
        );

        state.hour_angle_offset = Self::get_hour_angle_offset(hour_angle, &self.driver).await?;
        state.declination = dec;

        Ok(())
    }

    /// True if this telescope is capable of programmed synching to local horizontal coordinates.
    pub fn can_sync_alt_az(&self) -> Result<bool> {
        Ok(true)
    }

    /// Matches the scope's local horizontal coordinates to the given local horizontal coordinates.
    pub async fn sync_to_alt_az(&self, alt: Degrees, az: Degrees) -> Result<()> {
        let mut state = self.state.write().await;
        let (ha, dec) =
            astro_math::calculate_ha_dec_from_alt_az(alt, az, state.observation_location.latitude);
        state.hour_angle_offset = Self::get_hour_angle_offset(ha, &self.driver).await?;
        state.declination = dec;
        Ok(())
    }

    /// Matches the scope's equatorial coordinates to the TargetRightAscension and TargetDeclination equatorial coordinates.
    pub async fn sync_to_target(&self) -> Result<()> {
        let mut state = self.state.write().await;

        let hour_angle = astro_math::calculate_hour_angle(
            Self::calculate_utc_date(state.date_offset),
            state.observation_location.longitude,
            state.target.right_ascension,
        );

        state.hour_angle_offset = Self::get_hour_angle_offset(hour_angle, &self.driver).await?;
        state.declination = state.target.declination;

        Ok(())
    }
}
