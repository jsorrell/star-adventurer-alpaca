use crate::astro_math::{Degrees, Hours};
use crate::{astro_math, StarAdventurer};
use chrono::{DateTime, Utc};

use crate::errors::{AlpacaError, ErrorType, Result};

impl StarAdventurer {
    /*** Date ***/

    pub(crate) fn calculate_utc_date(date_offset: chrono::Duration) -> DateTime<Utc> {
        Utc::now() + date_offset
    }

    /// The UTC date/time of the telescope's internal clock in ISO 8601 format including fractional seconds.
    /// The general format (in Microsoft custom date format style) is yyyy-MM-ddTHH:mm:ss.fffffffZ E.g. 2016-03-04T17:45:31.1234567Z or 2016-11-14T07:03:08.1234567Z
    /// Please note the compulsory trailing Z indicating the 'Zulu', UTC time zone.
    pub fn get_utc_date(&self) -> Result<DateTime<Utc>> {
        Ok(Self::calculate_utc_date(
            self.state.read().unwrap().date_offset,
        ))
    }

    /// The UTC date/time of the telescope's internal clock in ISO 8601 format including fractional seconds. The general format (in Microsoft custom date format style) is yyyy-MM-ddTHH:mm:ss.fffffffZ E.g. 2016-03-04T17:45:31.1234567Z or 2016-11-14T07:03:08.1234567Z Please note the compulsary trailing Z indicating the 'Zulu', UTC time zone.
    pub fn set_utc_date(&mut self, time: DateTime<Utc>) -> Result<()> {
        self.state.write().unwrap().date_offset = time - Utc::now();
        Ok(())
    }

    /*** Latitude ***/

    #[inline]
    fn in_north(latitude: Degrees) -> bool {
        0. <= latitude
    }

    /// The geodetic(map) latitude (degrees, positive North, WGS84) of the site at which the telescope is located.
    pub fn get_latitude(&self) -> Result<Degrees> {
        Ok(self.state.read().unwrap().latitude)
    }

    /// Sets the observing site's latitude (degrees).
    pub fn set_latitude(&mut self, latitude: Degrees) -> Result<()> {
        self.state.write().unwrap().latitude = latitude;
        Ok(())
    }

    /*** Longitude ***/

    /// The longitude (degrees, positive East, WGS84) of the site at which the telescope is located.
    pub fn get_longitude(&self) -> Result<Degrees> {
        Ok(self.state.read().unwrap().longitude)
    }

    /// Sets the observing site's longitude (degrees, positive East, WGS84).
    pub fn set_longitude(&mut self, longitude: Degrees) -> Result<()> {
        self.state.write().unwrap().longitude = longitude;
        Ok(())
    }

    /*** Elevation ***/

    /// The elevation above mean sea level (meters) of the site at which the telescope is located
    pub fn get_elevation(&self) -> Result<f64> {
        Ok(self.state.read().unwrap().elevation)
    }

    /// Sets the elevation above mean sea level (metres) of the site at which the telescope is located.
    pub fn set_elevation(&mut self, elevation: f64) -> Result<()> {
        if elevation < -300. || 10000. < elevation {
            return Err(AlpacaError::from_msg(
                ErrorType::InvalidValue,
                format!(
                    "Elevation of {} is outside the valid range of -300 to 10000",
                    elevation
                ),
            ));
        }
        self.state.write().unwrap().elevation = elevation;
        Ok(())
    }

    /*** LST ***/

    /// The local apparent sidereal time from the telescope's internal clock (hours, sidereal)
    pub fn get_sidereal_time(&self) -> Result<Hours> {
        Ok(astro_math::calculate_local_sidereal_time(
            self.get_utc_date()?,
            self.get_longitude()?,
        ))
    }
}
