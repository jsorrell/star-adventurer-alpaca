use crate::util::*;
use crate::{astro_math, StarAdventurer};
use chrono::{DateTime, Utc};
use tokio::join;

use crate::util::result::{AscomError, AscomErrorType, AscomResult};

impl StarAdventurer {
    /*** Date ***/

    pub(crate) fn calculate_utc_date(date_offset: chrono::Duration) -> DateTime<Utc> {
        Utc::now() + date_offset
    }

    /// The UTC date/time of the telescope's internal clock in ISO 8601 format including fractional seconds.
    /// The general format (in Microsoft custom date format style) is yyyy-MM-ddTHH:mm:ss.fffffffZ E.g. 2016-03-04T17:45:31.1234567Z or 2016-11-14T07:03:08.1234567Z
    /// Please note the compulsory trailing Z indicating the 'Zulu', UTC time zone.
    pub async fn get_utc_date(&self) -> AscomResult<DateTime<Utc>> {
        Ok(Self::calculate_utc_date(
            self.state.read().await.date_offset,
        ))
    }

    /// The UTC date/time of the telescope's internal clock in ISO 8601 format including fractional seconds. The general format (in Microsoft custom date format style) is yyyy-MM-ddTHH:mm:ss.fffffffZ E.g. 2016-03-04T17:45:31.1234567Z or 2016-11-14T07:03:08.1234567Z Please note the compulsary trailing Z indicating the 'Zulu', UTC time zone.
    pub async fn set_utc_date(&self, time: DateTime<Utc>) -> AscomResult<()> {
        self.state.write().await.date_offset = time - Utc::now();
        Ok(())
    }

    /*** Latitude ***/

    /// The geodetic(map) latitude (degrees, positive North, WGS84) of the site at which the telescope is located.
    pub async fn get_latitude(&self) -> AscomResult<Degrees> {
        Ok(self.state.read().await.observation_location.latitude)
    }

    /// Sets the observing site's latitude (degrees).
    pub async fn set_latitude(&self, latitude: Degrees) -> AscomResult<()> {
        if latitude < -90. || 90. < latitude {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidValue,
                format!(
                    "Latitude of {} is outside the valid range of -90 to 90",
                    latitude
                ),
            ));
        }
        self.state.write().await.observation_location.latitude = latitude;
        Ok(())
    }

    /*** Longitude ***/

    /// The longitude (degrees, positive East, WGS84) of the site at which the telescope is located.
    pub async fn get_longitude(&self) -> AscomResult<Degrees> {
        Ok(self.state.read().await.observation_location.longitude)
    }

    /// Sets the observing site's longitude (degrees, positive East, WGS84).
    pub async fn set_longitude(&self, longitude: Degrees) -> AscomResult<()> {
        if longitude < -180. || 180. < longitude {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidValue,
                format!(
                    "Longitude of {} is outside the valid range of -180 to 180",
                    longitude
                ),
            ));
        }
        self.state.write().await.observation_location.longitude = longitude;
        Ok(())
    }

    /*** Elevation ***/

    /// The elevation above mean sea level (meters) of the site at which the telescope is located
    pub async fn get_elevation(&self) -> AscomResult<f64> {
        Ok(self.state.read().await.observation_location.elevation)
    }

    /// Sets the elevation above mean sea level (metres) of the site at which the telescope is located.
    pub async fn set_elevation(&self, elevation: f64) -> AscomResult<()> {
        if elevation < -300. || 10000. < elevation {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidValue,
                format!(
                    "Elevation of {} is outside the valid range of -300 to 10000",
                    elevation
                ),
            ));
        }
        self.state.write().await.observation_location.elevation = elevation;
        Ok(())
    }

    /*** LST ***/

    /// The local apparent sidereal time from the telescope's internal clock (hours, sidereal)
    pub async fn get_sidereal_time(&self) -> AscomResult<Hours> {
        let (date, longitude) = join!(self.get_utc_date(), self.get_longitude());
        Ok(astro_math::calculate_local_sidereal_time(date?, longitude?))
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util;
    use chrono::{TimeZone, Utc};
    use std::time::Duration;

    #[tokio::test]
    async fn test_date() {
        let sa = test_util::create_sa(None).await;

        let test_date = Utc.ymd(2222, 01, 01).and_hms(10, 00, 00);
        sa.set_utc_date(test_date).await.unwrap();
        assert!(sa.get_utc_date().await.unwrap() - test_date < chrono::Duration::milliseconds(1));
        std::thread::sleep(Duration::from_millis(1000));
        assert!(
            sa.get_utc_date().await.unwrap() - test_date - chrono::Duration::milliseconds(1000)
                < chrono::Duration::milliseconds(5)
        );
    }

    #[tokio::test]
    async fn test_observing_location() {
        let sa = test_util::create_sa(None).await;

        let test_lat0 = 59.8843434;
        let test_lat1 = -33.;

        let test_long = 77.;

        let test_elevation = 999.;

        sa.set_latitude(test_lat0).await.unwrap();
        assert_eq!(sa.get_latitude().await.unwrap(), test_lat0);

        sa.set_longitude(test_long).await.unwrap();
        assert_eq!(sa.get_longitude().await.unwrap(), test_long);
        assert_eq!(sa.get_latitude().await.unwrap(), test_lat0);

        sa.set_elevation(test_elevation).await.unwrap();
        assert_eq!(sa.get_longitude().await.unwrap(), test_long);
        assert_eq!(sa.get_latitude().await.unwrap(), test_lat0);
        assert_eq!(sa.get_elevation().await.unwrap(), test_elevation);

        sa.set_latitude(test_lat1).await.unwrap();
        assert_eq!(sa.get_longitude().await.unwrap(), test_long);
        assert_eq!(sa.get_latitude().await.unwrap(), test_lat1);
        assert_eq!(sa.get_elevation().await.unwrap(), test_elevation);
    }
}
