use tokio::join;

use crate::astro_math;
use crate::rotation_direction::RotationDirectionKey;
use crate::telescope_control::star_adventurer::StarAdventurer;
use crate::util::*;
use ascom_alpaca::api::SideOfPier;
use ascom_alpaca::ASCOMResult;

impl StarAdventurer {
    pub fn calc_mech_ha_from_ha(ha: Hours, pier_side: SideOfPier) -> Hours {
        astro_math::modulo(
            match pier_side {
                SideOfPier::East => ha - 6.,
                SideOfPier::West => ha + 6.,
                SideOfPier::Unknown => unreachable!(),
            },
            24.,
        )
    }

    pub fn calc_ha_from_mech_ha(mech_ha: Hours, pier_side: SideOfPier) -> Hours {
        astro_math::modulo(
            match pier_side {
                SideOfPier::East => mech_ha + 6.,
                SideOfPier::West => mech_ha - 6.,
                SideOfPier::Unknown => unreachable!(),
            },
            24.,
        )
    }

    pub(in crate::telescope_control) async fn get_ha(&self) -> ASCOMResult<Hours> {
        let mech_ha = self.get_mech_ha().await?;
        let pier_side = self.get_side_of_pier().await?;
        Ok(Self::calc_ha_from_mech_ha(mech_ha, pier_side))
    }

    // With the telescope pointing at the meridian, this is zero
    pub fn calc_ha(
        motor_pos: Degrees,
        mech_ha_offset: Hours,
        key: RotationDirectionKey,
        pier_side: SideOfPier,
    ) -> Hours {
        let mech_ha = Self::calc_mech_ha(motor_pos, mech_ha_offset, key);
        Self::calc_ha_from_mech_ha(mech_ha, pier_side)
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
    pub async fn get_ra(&self) -> ASCOMResult<Hours> {
        let ha = self.get_ha().await?;
        let (observation_location, date_offset) = join!(
            async { *self.settings.observation_location.read().await },
            async { *self.settings.date_offset.read().await },
        );

        Ok(Self::calc_ra(
            ha,
            observation_location.longitude,
            date_offset,
        ))
    }

    /// The declination (degrees) of the mount's current equatorial coordinates, in the coordinate system given by the EquatorialSystem property.
    /// Reading the property will raise an error if the value is unavailable.
    pub async fn get_dec(&self) -> ASCOMResult<Degrees> {
        Ok(*self.settings.declination.read().await)
    }

    /// The altitude above the local horizon of the mount's current position (degrees, positive up)
    pub async fn get_altitude(&self) -> ASCOMResult<Degrees> {
        let hour_angle = self.get_ha().await?;

        Ok(astro_math::calculate_alt_from_ha_dec(
            hour_angle,
            *self.settings.declination.read().await,
            self.settings.observation_location.read().await.latitude,
        ))
    }

    /// The azimuth at the local horizon of the mount's current position (degrees, North-referenced, positive East/clockwise).
    pub async fn get_azimuth(&self) -> ASCOMResult<f64> {
        let hour_angle = self.get_ha().await?;

        Ok(astro_math::calculate_az_from_ha_dec(
            hour_angle,
            *self.settings.declination.read().await,
            self.settings.observation_location.read().await.latitude,
        ))
    }
}
