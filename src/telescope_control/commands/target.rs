use crate::astro_math::{Degrees, Hours};
use crate::telescope_control::StarAdventurer;
use crate::util::*;
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult};

#[derive(Debug, Default, PartialEq, Copy, Clone)]
pub struct Target {
    pub right_ascension: Option<Hours>,
    pub declination: Option<Degrees>,
}

impl Target {
    pub fn try_get_right_ascension(&self) -> ASCOMResult<Hours> {
        match self.right_ascension {
            Some(t) => Ok(t),
            None => Err(ASCOMError::new(
                ASCOMErrorCode::VALUE_NOT_SET,
                "Target ra not set".to_string(),
            )),
        }
    }

    pub fn try_get_declination(&self) -> ASCOMResult<Degrees> {
        match self.declination {
            Some(t) => Ok(t),
            None => Err(ASCOMError::new(
                ASCOMErrorCode::VALUE_NOT_SET,
                "Target dec not set".to_string(),
            )),
        }
    }
}

impl StarAdventurer {
    /// The declination (degrees, positive North) for the target of an equatorial slew or sync operation
    pub async fn get_target_declination(&self) -> ASCOMResult<Degrees> {
        self.settings.target.read().await.try_get_declination()
    }

    /// Sets the declination (degrees, positive North) for the target of an equatorial slew or sync operation
    pub async fn set_target_dec(&self, dec: Degrees) -> ASCOMResult<()> {
        check_dec(dec)?;
        self.settings.target.write().await.declination = Some(dec);
        Ok(())
    }

    /// The right ascension (hours) for the target of an equatorial slew or sync operation
    pub async fn get_target_ra(&self) -> ASCOMResult<Hours> {
        self.settings.target.read().await.try_get_right_ascension()
    }

    /// Sets the right ascension (hours) for the target of an equatorial slew or sync operation
    pub async fn set_target_ra(&self, ra: Hours) -> ASCOMResult<()> {
        check_ra(ra)?;
        self.settings.target.write().await.right_ascension = Some(ra);
        Ok(())
    }
}
