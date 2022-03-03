use rocket::futures::join;

use crate::astro_math;
use crate::rotation_direction::{RotationDirection, RotationDirectionKey};
use crate::telescope_control::star_adventurer::StarAdventurer;
use crate::tracking_direction::TrackingDirection;
use crate::util::*;

impl StarAdventurer {
    pub fn calc_ha(
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

    pub(in crate::telescope_control) async fn get_ha(&self) -> AscomResult<Hours> {
        let pos = self.connection.get_pos().await?;
        let (hour_angle_offset, obs_loc) = join!(
            async { *self.settings.hour_angle_offset.read().await },
            async { *self.settings.observation_location.read().await },
        );

        Ok(Self::calc_ha(
            pos,
            hour_angle_offset,
            obs_loc.get_rotation_direction_key(),
        ))
    }

    pub fn calc_ra(ha: Hours, longitude: Degrees, date_offset: chrono::Duration) -> Hours {
        let lst = astro_math::calculate_local_sidereal_time(
            Self::calculate_utc_date(date_offset),
            longitude,
        );

        astro_math::modulo(lst - ha, 24.)
    }

    /// The right ascension (hours) of the mount's current equatorial coordinates,
    /// in the coordinate system given by the EquatorialSystem property
    pub async fn get_ra(&self) -> AscomResult<Hours> {
        let pos = self.connection.get_pos().await?;
        let (observation_location, hour_angle_offset, date_offset) = join!(
            async { *self.settings.observation_location.read().await },
            async { *self.settings.hour_angle_offset.read().await },
            async { *self.settings.date_offset.read().await },
        );
        let ha = Self::calc_ha(
            pos,
            hour_angle_offset,
            observation_location.get_rotation_direction_key(),
        );
        Ok(Self::calc_ra(
            ha,
            observation_location.longitude,
            date_offset,
        ))
    }

    /// The declination (degrees) of the mount's current equatorial coordinates, in the coordinate system given by the EquatorialSystem property.
    /// Reading the property will raise an error if the value is unavailable.
    pub async fn get_dec(&self) -> AscomResult<Degrees> {
        Ok(*self.settings.declination.read().await)
    }

    /// The altitude above the local horizon of the mount's current position (degrees, positive up)
    pub async fn get_altitude(&self) -> AscomResult<Degrees> {
        let hour_angle = self.get_ha().await?;

        Ok(astro_math::calculate_alt_from_ha_dec(
            hour_angle,
            *self.settings.declination.read().await,
            self.settings.observation_location.read().await.latitude,
        ))
    }

    /// The azimuth at the local horizon of the mount's current position (degrees, North-referenced, positive East/clockwise).
    pub async fn get_azimuth(&self) -> AscomResult<f64> {
        let hour_angle = self.get_ha().await?;

        Ok(astro_math::calculate_az_from_ha_dec(
            hour_angle,
            *self.settings.declination.read().await,
            self.settings.observation_location.read().await.latitude,
        ))
    }
}
