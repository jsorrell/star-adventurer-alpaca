use crate::astro_math;
use crate::rotation_direction::{RotationDirection, RotationDirectionKey};
use crate::telescope_control::driver::Driver;
use crate::telescope_control::{StarAdventurer, State};
use crate::tracking_direction::TrackingDirection;
use crate::util::*;

impl StarAdventurer {
    pub fn calc_hour_angle(
        driver_pos: Degrees,
        hour_angle_offset: Hours,
        key: RotationDirectionKey,
    ) -> Hours {
        let tracking_direction: MotorEncodingDirection =
            TrackingDirection::WithTracking.using(key).into();
        let unmoduloed_angle = hour_angle_offset
            + tracking_direction.get_sign_f64() * astro_math::deg_to_hours(driver_pos);
        astro_math::modulo(unmoduloed_angle, 24.)
    }

    pub(in crate::telescope_control) async fn inquire_ha(
        state: &State,
        driver: Driver,
    ) -> AscomResult<Hours> {
        Ok(Self::calc_hour_angle(
            driver.get_pos().await?,
            state.hour_angle_offset,
            state.observation_location.get_rotation_direction_key(),
        ))
    }

    pub(in crate::telescope_control) async fn inquire_ra(
        state: &State,
        driver: Driver,
    ) -> AscomResult<Hours> {
        let lst = astro_math::calculate_local_sidereal_time(
            Self::calculate_utc_date(state.date_offset),
            state.observation_location.longitude,
        );

        Ok(astro_math::modulo(
            lst - Self::inquire_ha(state, driver).await?,
            24.,
        ))
    }

    /// The right ascension (hours) of the mount's current equatorial coordinates,
    /// in the coordinate system given by the EquatorialSystem property
    pub async fn get_ra(&self) -> AscomResult<Hours> {
        let state = self.state.read().await;
        Ok(Self::inquire_ra(&state, self.driver.clone()).await?)
    }

    /// The declination (degrees) of the mount's current equatorial coordinates, in the coordinate system given by the EquatorialSystem property.
    /// Reading the property will raise an error if the value is unavailable.
    pub async fn get_dec(&self) -> AscomResult<Degrees> {
        Ok(self.state.read().await.declination)
    }

    /// The altitude above the local horizon of the mount's current position (degrees, positive up)
    pub async fn get_altitude(&self) -> AscomResult<Degrees> {
        let state = self.state.read().await;
        let hour_angle = Self::inquire_ha(&state, self.driver.clone()).await?;

        Ok(astro_math::calculate_alt_from_ha_dec(
            hour_angle,
            state.declination,
            state.observation_location.latitude,
        ))
    }

    /// The azimuth at the local horizon of the mount's current position (degrees, North-referenced, positive East/clockwise).
    pub async fn get_azimuth(&self) -> AscomResult<f64> {
        let state = self.state.read().await;
        let hour_angle = Self::inquire_ha(&state, self.driver.clone()).await?;

        Ok(astro_math::calculate_az_from_ha_dec(
            hour_angle,
            state.declination,
            state.observation_location.latitude,
        ))
    }
}
