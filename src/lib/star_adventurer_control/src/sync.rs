use crate::astro_math::{Degrees, Hours};
use crate::errors::Result;
use crate::{astro_math, StarAdventurer, RA_CHANNEL};

impl StarAdventurer {
    /// True if this telescope is capable of programmed synching to equatorial coordinates.
    pub fn can_sync(&self) -> Result<bool> {
        Ok(true)
    }

    /// Matches the scope's equatorial coordinates to the given equatorial coordinates.
    pub async fn sync_to_coordinates(&mut self, ra: Hours, dec: Degrees) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let mut driver = self.driver.lock().unwrap();

        let hour_angle = astro_math::calculate_hour_angle(
            Self::calculate_utc_date(state.date_offset),
            state.longitude,
            ra,
        );
        state.hour_angle_offset = hour_angle - driver.get_pos(RA_CHANNEL)?;

        state.declination = dec;

        Ok(())
    }

    /// True if this telescope is capable of programmed synching to local horizontal coordinates.
    pub fn can_sync_alt_az(&self) -> Result<bool> {
        Ok(true)
    }

    /// Matches the scope's local horizontal coordinates to the given local horizontal coordinates.
    pub fn sync_to_alt_az(&self, az: Degrees, alt: Degrees) {
        todo!()
    }

    /// Matches the scope's equatorial coordinates to the TargetRightAscension and TargetDeclination equatorial coordinates.
    pub fn sync_to_target(&self) -> Result<()> {
        let mut state = self.state.write().unwrap();
        let mut driver = self.driver.lock().unwrap();

        let hour_angle = astro_math::calculate_hour_angle(
            Self::calculate_utc_date(state.date_offset),
            state.longitude,
            state.target.right_ascension,
        );
        state.hour_angle_offset = hour_angle - driver.get_pos(RA_CHANNEL)?;

        state.declination = state.target.declination;

        Ok(())
    }
}
