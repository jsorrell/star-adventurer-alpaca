use crate::util::*;
use chrono::{DateTime, ParseResult, Utc};
use proc_macros::alpaca_request_data;
use rocket::form::FromForm;

#[alpaca_request_data]
#[derive(FromForm, Copy, Clone)]
pub struct EmptyData {}

#[allow(dead_code)]
#[alpaca_request_data]
#[derive(FromForm, Clone)]
pub struct ActionData {
    #[field(name = "Action")]
    pub action: String,
    #[field(name = "Parameters")]
    pub parameters: String,
}

#[allow(dead_code)]
#[alpaca_request_data]
#[derive(FromForm, Clone)]
pub struct CommandData {
    #[field(name = "Command")]
    pub command: String,
    #[field(name = "Raw")]
    pub raw: bool,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct SetConnectedData {
    #[field(name = "Connected")]
    pub connected: bool,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct DeclinationRateData {
    #[field(name = "DeclinationRate")]
    pub declination_rate: f64,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct DoesRefractionData {
    #[field(name = "DoesRefraction")]
    pub does_refraction: bool,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct GuideRateDeclinationData {
    #[field(name = "GuideRateDeclination")]
    pub guide_rate_declination: f64,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct GuideRateRightAscensionData {
    #[field(name = "GuideRateRightAscension")]
    pub guide_rate_right_ascension: f64,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct RightAscensionRateData {
    #[field(name = "RightAscensionRate")]
    pub right_ascension_rate: f64,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct SideOfPierData {
    #[field(name = "SideOfPier")]
    pub side_of_pier: PierSide,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct SiteElevationData {
    #[field(name = "SiteElevation")]
    pub site_elevation: f64,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct SiteLatitudeData {
    #[field(name = "SiteLatitude")]
    pub site_latitude: f64,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct SiteLongitudeData {
    #[field(name = "SiteLongitude")]
    pub site_longitude: f64,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct SlewSettleTimeData {
    #[field(name = "SlewSettleTime")]
    pub slew_settle_time: i32,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct TargetDeclinationData {
    #[field(name = "TargetDeclination")]
    pub target_declination: f64,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct TargetRightAscensionData {
    #[field(name = "TargetRightAscension")]
    pub target_right_ascension: f64,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct TrackingData {
    #[field(name = "Tracking")]
    pub tracking: bool,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct TrackingRateData {
    #[field(name = "TrackingRate")]
    pub tracking_rate: i32,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Clone)]
pub struct UTCDateData {
    #[field(name = "UTCDate")]
    pub utc_date_string: String,
}

impl UTCDateData {
    pub fn get_utc_date(&self) -> ParseResult<DateTime<Utc>> {
        let t = DateTime::parse_from_str(self.utc_date_string.as_str(), ALPACA_DATE_FMT)?;
        let naive_time = t.naive_utc();
        Ok(DateTime::<Utc>::from_utc(naive_time, Utc))
    }
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct AxisData {
    #[field(name = "Axis")]
    pub axis: Axis,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct MoveAxisData {
    #[field(name = "Axis")]
    pub axis: Axis,
    #[field(name = "Rate")]
    pub rate: f64,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct PulseGuideData {
    #[field(name = "Direction")]
    pub direction: GuideDirection,
    #[field(name = "Duration")]
    pub duration: u32,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct AltAzData {
    #[field(name = "Altitude")]
    pub altitide: f64,
    #[field(name = "Azimuth")]
    pub azimuth: f64,
}

#[alpaca_request_data]
#[derive(FromForm, Debug, Copy, Clone)]
pub struct CoordinateData {
    #[field(name = "RightAscension")]
    pub right_ascension: f64,
    #[field(name = "Declination")]
    pub declination: f64,
}
