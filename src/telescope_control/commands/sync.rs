use super::target::Target;
use crate::astro_math;
use crate::telescope_control::StarAdventurer;
use crate::util::*;

impl StarAdventurer {
    /// Raw helper function that performs no checks
    async fn sync_to_ra_dec(&self, ra: Hours, dec: Degrees) -> AscomResult<()> {
        let hour_angle = astro_math::calculate_hour_angle(
            Self::calculate_utc_date(*self.settings.date_offset.read().await),
            self.settings.observation_location.read().await.longitude,
            ra,
        );

        *self.settings.hour_angle_offset.write().await =
            Self::calc_hour_angle_offset(hour_angle, self.connection.get_pos().await?);
        *self.settings.declination.write().await = dec;
        Ok(())
    }

    /// True if this telescope is capable of programmed synching to equatorial coordinates.
    pub async fn can_sync(&self) -> AscomResult<bool> {
        Ok(true)
    }

    #[inline]
    pub(in crate::telescope_control) fn calc_hour_angle_offset(
        hour_angle: Hours,
        motor_pos: Degrees,
    ) -> Hours {
        hour_angle - astro_math::deg_to_hours(motor_pos)
    }

    /// Matches the scope's equatorial coordinates to the given equatorial coordinates.
    pub async fn sync_to_coordinates(&self, ra: Hours, dec: Degrees) -> AscomResult<()> {
        check_ra(ra)?;
        check_dec(dec)?;

        if self.connection.is_parked().await? {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidWhileParked,
                "Can't sync while parked".to_string(),
            ));
        }

        if !self.connection.is_tracking().await? {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Can't sync to coords unless tracking".to_string(),
            ));
        }

        // Syncing to ra/dec sets the target as well
        *self.settings.target.write().await = Target {
            right_ascension: Some(ra),
            declination: Some(dec),
        };

        self.sync_to_ra_dec(ra, dec).await
    }

    /// True if this telescope is capable of programmed synching to local horizontal coordinates.
    pub async fn can_sync_alt_az(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Matches the scope's local horizontal coordinates to the given local horizontal coordinates.
    pub async fn sync_to_alt_az(&self, alt: Degrees, az: Degrees) -> AscomResult<()> {
        check_alt(alt)?;
        check_az(az)?;

        if self.connection.is_parked().await? {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidWhileParked,
                "Can't sync while parked".to_string(),
            ));
        }

        let (ha, dec) = astro_math::calculate_ha_dec_from_alt_az(
            alt,
            az,
            self.settings.observation_location.read().await.latitude,
        );
        *self.settings.hour_angle_offset.write().await =
            Self::calc_hour_angle_offset(ha, self.connection.get_pos().await?);
        *self.settings.declination.write().await = dec;
        Ok(())
    }

    /// Matches the scope's equatorial coordinates to the TargetRightAscension and TargetDeclination equatorial coordinates.
    pub async fn sync_to_target(&self) -> AscomResult<()> {
        if self.connection.is_parked().await? {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidWhileParked,
                "Can't sync while parked".to_string(),
            ));
        }

        if !self.connection.is_tracking().await? {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Can't sync to coords unless tracking".to_string(),
            ));
        }

        let target = self.settings.target.read().await;

        self.sync_to_ra_dec(
            target.try_get_right_ascension()?,
            target.try_get_declination()?,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util;

    #[tokio::test]
    async fn test_sync() {
        let sa = test_util::create_sa(None).await;
        sa.sync_to_coordinates(18., 33.).await.unwrap();
        assert_float_eq::assert_float_absolute_eq!(sa.get_ra().await.unwrap(), 18., 1E-4);
        assert_float_eq::assert_float_absolute_eq!(sa.get_dec().await.unwrap(), 33., 1E-4);
        sa.sync_to_alt_az(33., -22.).await.unwrap();
        assert_float_eq::assert_float_absolute_eq!(sa.get_altitude().await.unwrap(), 33., 1E-4);
        assert_float_eq::assert_float_absolute_eq!(sa.get_azimuth().await.unwrap(), -22., 1E-4);
        sa.set_target_ra(12.).await.unwrap();
        sa.set_target_dec(-87.).await.unwrap();
        sa.sync_to_target().await.unwrap();
        assert_float_eq::assert_float_absolute_eq!(sa.get_ra().await.unwrap(), 12., 1E-4);
        assert_float_eq::assert_float_absolute_eq!(sa.get_dec().await.unwrap(), -87., 1E-4);
    }
}
