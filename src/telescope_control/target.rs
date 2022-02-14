use crate::astro_math::{Degrees, Hours};
use crate::telescope_control::StarAdventurer;
use crate::util::result::*;

#[derive(Debug, Default, PartialEq, Copy, Clone)]
pub struct Target {
    pub right_ascension: Option<Hours>,
    pub declination: Option<Degrees>,
}

impl Target {
    pub fn try_get_right_ascension(&self) -> AscomResult<Hours> {
        match self.right_ascension {
            Some(t) => Ok(t),
            None => Err(AscomError::from_msg(
                AscomErrorType::ValueNotSet,
                "Target ra not set".to_string(),
            )),
        }
    }

    pub fn try_get_declination(&self) -> AscomResult<Degrees> {
        match self.declination {
            Some(t) => Ok(t),
            None => Err(AscomError::from_msg(
                AscomErrorType::ValueNotSet,
                "Target dec not set".to_string(),
            )),
        }
    }
}

impl StarAdventurer {
    /// The declination (degrees, positive North) for the target of an equatorial slew or sync operation
    pub async fn get_target_declination(&self) -> AscomResult<Degrees> {
        let state = self.state.read().await;
        Ok(state.target.try_get_declination()?)
    }

    /// Sets the declination (degrees, positive North) for the target of an equatorial slew or sync operation
    pub async fn set_target_dec(&self, dec: Degrees) -> AscomResult<()> {
        check_dec(dec)?;
        self.state.write().await.target.declination = Some(dec);
        Ok(())
    }

    /// The right ascension (hours) for the target of an equatorial slew or sync operation
    pub async fn get_target_ra(&self) -> AscomResult<Hours> {
        let state = self.state.read().await;
        Ok(state.target.try_get_right_ascension()?)
    }

    /// Sets the right ascension (hours) for the target of an equatorial slew or sync operation
    pub async fn set_target_ra(&self, ra: Hours) -> AscomResult<()> {
        check_ra(ra)?;
        self.state.write().await.target.right_ascension = Some(ra);
        Ok(())
    }
}
