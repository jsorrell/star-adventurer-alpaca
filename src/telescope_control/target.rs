use crate::astro_math::{Degrees, Hours};
use crate::telescope_control::StarAdventurer;
use crate::util::result::*;

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Target {
    pub right_ascension: f64,
    pub declination: f64,
}

impl Default for Target {
    fn default() -> Self {
        Target {
            right_ascension: 0.,
            declination: 0.,
        }
    }
}

impl StarAdventurer {
    /// The declination (degrees, positive North) for the target of an equatorial slew or sync operation
    pub async fn get_target_declination(&self) -> AscomResult<Degrees> {
        Ok(self.state.read().await.target.declination)
    }

    /// Sets the declination (degrees, positive North) for the target of an equatorial slew or sync operation
    pub async fn set_target_dec(&self, declination: Degrees) -> AscomResult<()> {
        self.state.write().await.target.declination = declination;
        Ok(())
    }

    /// The right ascension (hours) for the target of an equatorial slew or sync operation
    pub async fn get_target_ra(&self) -> AscomResult<Hours> {
        Ok(self.state.read().await.target.right_ascension)
    }

    /// Sets the right ascension (hours) for the target of an equatorial slew or sync operation
    pub async fn set_target_ra(&self, ra: Hours) -> AscomResult<()> {
        self.state.write().await.target.right_ascension = ra;
        Ok(())
    }
}
