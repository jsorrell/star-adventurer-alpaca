use crate::telescope_control::StarAdventurer;
use crate::util::*;
use ascom_alpaca::api::DriveRate;
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult};

impl StarAdventurer {
    /// True if the Tracking property can be changed, turning telescope sidereal tracking on and off.
    pub async fn can_set_tracking(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    /// The right ascension tracking rate (arcseconds per second, default = 0.0)
    pub async fn get_ra_rate(&self) -> ASCOMResult<f64> {
        Ok(0.)
    }

    /// True if the RightAscensionRate property can be changed to provide offset tracking in the right ascension axis.
    pub async fn can_set_ra_rate(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Sets the right ascension tracking rate (arcseconds per second)
    pub async fn set_ra_rate(&self, _rate: f64) -> ASCOMResult<()> {
        Err(ASCOMError::new(
            ASCOMErrorCode::NOT_IMPLEMENTED,
            "Setting RA tracking rate is not supported".to_string(),
        ))
    }

    /// The declination tracking rate (arcseconds per second, default = 0.0)
    pub async fn get_declination_rate(&self) -> ASCOMResult<f64> {
        Ok(0.)
    }

    /// True if the DeclinationRate property can be changed to provide offset tracking in the declination axis
    pub async fn can_set_declination_rate(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Sets the declination tracking rate (arcseconds per second)
    pub async fn set_declination_rate(&self, _rate: f64) -> ASCOMResult<()> {
        Err(ASCOMError::new(
            ASCOMErrorCode::NOT_IMPLEMENTED,
            "Declination tracking not available".to_string(),
        ))
    }

    /// Returns an array of supported DriveRates values that describe the permissible values of the DriveRate property for this telescope type.
    pub async fn get_tracking_rates(&self) -> ASCOMResult<Vec<DriveRate>> {
        Ok(vec![
            DriveRate::Sidereal,
            DriveRate::Lunar,
            DriveRate::Solar,
            DriveRate::King,
        ])
    }

    /// The current tracking rate of the telescope's sidereal drive.
    pub async fn get_tracking_rate(&self) -> ASCOMResult<DriveRate> {
        Ok(*self.settings.tracking_rate.read().await)
    }

    /// Sets the tracking rate of the telescope's sidereal drive
    pub async fn set_tracking_rate(&self, tracking_rate: DriveRate) -> ASCOMResult<()> {
        // No change needed
        let mut lock = self.settings.tracking_rate.write().await;
        if *lock == tracking_rate {
            return Ok(());
        }

        *lock = tracking_rate;

        let tracking_motion_rate = tracking_rate.into_motion_rate(
            self.settings
                .observation_location
                .read()
                .await
                .get_rotation_direction_key(),
        );

        self.connection
            .update_tracking_rate(tracking_motion_rate)
            .await?;

        Ok(())
    }

    /// Returns the state of the telescope's sidereal tracking drive.
    pub async fn is_tracking(&self) -> ASCOMResult<bool> {
        Ok(self.connection.is_tracking().await?)
    }

    /// Sets the state of the telescope's sidereal tracking drive.
    /// TODO does setting tracking to true stop gotos?
    /// TODO Does it change what they'll do when the gotos are over?
    /// TODO Going with can only set it while not gotoing
    pub async fn set_is_tracking(&self, should_track: bool) -> ASCOMResult<()> {
        if should_track {
            let tracking_rate = self.settings.tracking_rate.read().await;
            let key = self
                .settings
                .observation_location
                .read()
                .await
                .get_rotation_direction_key();

            self.connection
                .start_tracking(tracking_rate.into_motion_rate(key))
                .await?
        } else {
            self.connection.stop_tracking().await?;
        }
        Ok(())
    }
}
