use crate::astro_math::{Degrees, Hours};
use crate::errors::Result;
use crate::{astro_math, StarAdventurer, RA_CHANNEL};
use std::sync::MutexGuard;
use synscan::MotorController;

impl StarAdventurer {
    fn get_hour_angle(
        driver: &mut MutexGuard<MotorController>,
        hour_angle_offset: Hours,
    ) -> Result<Hours> {
        let unmoduloed_angle =
            astro_math::deg_to_hours(driver.get_pos(RA_CHANNEL)?) + hour_angle_offset;
        Ok(astro_math::modulo(unmoduloed_angle, 24.))
    }

    fn calculate_ra(local_sidereal_time: Hours, hour_angle: Hours) -> Hours {
        astro_math::modulo(local_sidereal_time - hour_angle, 24.)
    }

    /// The right ascension (hours) of the mount's current equatorial coordinates,
    /// in the coordinate system given by the EquatorialSystem property
    pub async fn get_ra(&mut self) -> Result<Hours> {
        let state = self.state.read().await;
        let lst = astro_math::calculate_local_sidereal_time(
            Self::calculate_utc_date(state.date_offset),
            state.observation_location.longitude,
        );

        // FIXME spawn task for driver operation

        let mut driver = self.driver.lock().unwrap();
        Ok(Self::calculate_ra(
            lst,
            Self::get_hour_angle(&mut driver, state.hour_angle_offset)?,
        ))
    }

    /// The declination (degrees) of the mount's current equatorial coordinates, in the coordinate system given by the EquatorialSystem property.
    /// Reading the property will raise an error if the value is unavailable.
    pub async fn get_dec(&self) -> Result<Degrees> {
        Ok(self.state.read().await.declination)
    }

    /// The altitude above the local horizon of the mount's current position (degrees, positive up)
    pub async fn get_altitude(&mut self) -> Result<Degrees> {
        let state = self.state.read().await;
        let mut driver = self.driver.lock().unwrap();

        let hour_angle = Self::get_hour_angle(&mut driver, state.hour_angle_offset)?;

        Ok(astro_math::calculate_alt_from_ha_dec(
            hour_angle,
            state.declination,
            state.observation_location.latitude,
        ))
    }

    /// The azimuth at the local horizon of the mount's current position (degrees, North-referenced, positive East/clockwise).
    pub async fn get_azimuth(&mut self) -> Result<f64> {
        let state = self.state.read().await;
        let mut driver = self.driver.lock().unwrap();

        let hour_angle = Self::get_hour_angle(&mut driver, state.hour_angle_offset)?;

        Ok(astro_math::calculate_az_from_ha_dec(
            hour_angle,
            state.declination,
            state.observation_location.latitude,
        ))
    }
}
