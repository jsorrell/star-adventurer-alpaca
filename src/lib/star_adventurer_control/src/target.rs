use crate::astro_math::{Degrees, Hours};
use crate::errors::Result;
use crate::StarAdventurer;

impl StarAdventurer {
    /// The declination (degrees, positive North) for the target of an equatorial slew or sync operation
    pub async fn get_target_declination(&self) -> Result<Degrees> {
        Ok(self.state.read().await.target.declination)
    }

    /// Sets the declination (degrees, positive North) for the target of an equatorial slew or sync operation
    pub async fn set_target_dec(&mut self, declination: Degrees) -> Result<()> {
        self.state.write().await.target.declination = declination;
        Ok(())
    }

    /// The right ascension (hours) for the target of an equatorial slew or sync operation
    pub async fn get_target_ra(&self) -> Result<Hours> {
        Ok(self.state.read().await.target.right_ascension)
    }

    /// Sets the right ascension (hours) for the target of an equatorial slew or sync operation
    pub async fn set_target_ra(&mut self, ra: Hours) -> Result<()> {
        self.state.write().await.target.right_ascension = ra;
        Ok(())
    }
}
