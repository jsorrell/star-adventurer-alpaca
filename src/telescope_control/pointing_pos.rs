use crate::astro_math;
use crate::telescope_control::driver::Driver;
use crate::telescope_control::StarAdventurer;
use crate::util::*;

impl StarAdventurer {
    async fn get_hour_angle(driver: Driver, hour_angle_offset: Hours) -> AscomResult<Hours> {
        let unmoduloed_angle =
            astro_math::deg_to_hours(driver.get_pos().await?) + hour_angle_offset;
        Ok(astro_math::modulo(unmoduloed_angle, 24.))
    }

    fn calculate_ra(local_sidereal_time: Hours, hour_angle: Hours) -> Hours {
        astro_math::modulo(local_sidereal_time - hour_angle, 24.)
    }

    /// The right ascension (hours) of the mount's current equatorial coordinates,
    /// in the coordinate system given by the EquatorialSystem property
    pub async fn get_ra(&self) -> AscomResult<Hours> {
        let state = self.state.read().await;
        let lst = astro_math::calculate_local_sidereal_time(
            Self::calculate_utc_date(state.date_offset),
            state.observation_location.longitude,
        );

        Ok(Self::calculate_ra(
            lst,
            Self::get_hour_angle(self.driver.clone(), state.hour_angle_offset).await?,
        ))
    }

    /// The declination (degrees) of the mount's current equatorial coordinates, in the coordinate system given by the EquatorialSystem property.
    /// Reading the property will raise an error if the value is unavailable.
    pub async fn get_dec(&self) -> AscomResult<Degrees> {
        Ok(self.state.read().await.declination)
    }

    /// The altitude above the local horizon of the mount's current position (degrees, positive up)
    pub async fn get_altitude(&self) -> AscomResult<Degrees> {
        let state = self.state.read().await;
        let hour_angle = Self::get_hour_angle(self.driver.clone(), state.hour_angle_offset).await?;

        Ok(astro_math::calculate_alt_from_ha_dec(
            hour_angle,
            state.declination,
            state.observation_location.latitude,
        ))
    }

    /// The azimuth at the local horizon of the mount's current position (degrees, North-referenced, positive East/clockwise).
    pub async fn get_azimuth(&self) -> AscomResult<f64> {
        let state = self.state.read().await;
        let hour_angle = Self::get_hour_angle(self.driver.clone(), state.hour_angle_offset).await?;

        Ok(astro_math::calculate_az_from_ha_dec(
            hour_angle,
            state.declination,
            state.observation_location.latitude,
        ))
    }
}
