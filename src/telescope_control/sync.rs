use crate::astro_math;
use crate::telescope_control::driver::Driver;
use crate::telescope_control::StarAdventurer;
use crate::util::*;

impl StarAdventurer {
    /// True if this telescope is capable of programmed synching to equatorial coordinates.
    pub async fn can_sync(&self) -> AscomResult<bool> {
        Ok(true)
    }

    pub(in crate::telescope_control) async fn get_hour_angle_offset(
        hour_angle: Hours,
        driver: Driver,
    ) -> AscomResult<Hours> {
        let driver_pos = driver.get_pos().await? as Degrees;
        Ok(hour_angle - astro_math::deg_to_hours(driver_pos))
    }

    /// Matches the scope's equatorial coordinates to the given equatorial coordinates.
    pub async fn sync_to_coordinates(&self, ra: Hours, dec: Degrees) -> AscomResult<()> {
        check_ra(ra)?;
        check_dec(dec)?;

        let mut state = self.state.write().await;

        if state.motor_state.is_parked() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidWhileParked,
                "Can't sync while parked".to_string(),
            ));
        }

        // Syncing to ra/dec sets the target as well
        state.target.right_ascension = Some(ra);
        state.target.declination = Some(dec);

        let hour_angle = astro_math::calculate_hour_angle(
            Self::calculate_utc_date(state.date_offset),
            state.observation_location.longitude,
            ra,
        );

        state.hour_angle_offset =
            Self::get_hour_angle_offset(hour_angle, self.driver.clone()).await?;
        state.declination = dec;

        Ok(())
    }

    /// True if this telescope is capable of programmed synching to local horizontal coordinates.
    pub async fn can_sync_alt_az(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Matches the scope's local horizontal coordinates to the given local horizontal coordinates.
    pub async fn sync_to_alt_az(&self, alt: Degrees, az: Degrees) -> AscomResult<()> {
        check_alt(alt)?;
        check_az(az)?;
        let mut state = self.state.write().await;
        if state.motor_state.is_parked() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidWhileParked,
                "Can't sync while parked".to_string(),
            ));
        }

        let (ha, dec) =
            astro_math::calculate_ha_dec_from_alt_az(alt, az, state.observation_location.latitude);
        state.hour_angle_offset = Self::get_hour_angle_offset(ha, self.driver.clone()).await?;
        state.declination = dec;
        Ok(())
    }

    /// Matches the scope's equatorial coordinates to the TargetRightAscension and TargetDeclination equatorial coordinates.
    pub async fn sync_to_target(&self) -> AscomResult<()> {
        let mut state = self.state.write().await;
        if state.motor_state.is_parked() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidWhileParked,
                "Can't sync while parked".to_string(),
            ));
        }

        let hour_angle = astro_math::calculate_hour_angle(
            Self::calculate_utc_date(state.date_offset),
            state.observation_location.longitude,
            state.target.try_get_right_ascension()?,
        );

        state.hour_angle_offset =
            Self::get_hour_angle_offset(hour_angle, self.driver.clone()).await?;
        state.declination = state.target.try_get_declination()?;

        Ok(())
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
