use crate::request::*;
use crate::response::AlpacaResponse;
use crate::util::*;
use crate::AlpacaState;
use crate::AxisRateRange;
use chrono::{DateTime, Utc};
use proc_macros::alpaca_handler;
use rocket::State;

#[alpaca_handler]
pub async fn get_alignment_mode(state: &AlpacaState) -> AscomResult<AlignmentMode> {
    state.sa.get_alignment_mode().await
}

#[alpaca_handler]
pub async fn get_altitude(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_altitude().await
}

#[alpaca_handler]
pub async fn get_aperture_area(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_aperture_area().await
}

#[alpaca_handler]
pub async fn get_aperture_diameter(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_aperture().await
}

#[alpaca_handler]
pub async fn get_at_home(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.is_home().await
}

#[alpaca_handler]
pub async fn get_at_park(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.is_parked().await
}

#[alpaca_handler]
pub async fn get_azimuth(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_azimuth().await
}

#[alpaca_handler]
pub async fn get_can_find_home(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_find_home().await
}

#[alpaca_handler]
pub async fn get_can_park(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_park().await
}

#[alpaca_handler]
pub async fn get_can_pulse_guide(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_pulse_guide().await
}

#[alpaca_handler]
pub async fn get_can_set_declination_rate(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_set_declination_rate().await
}

#[alpaca_handler]
pub async fn get_can_set_guide_rates(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_set_guide_rates().await
}

#[alpaca_handler]
pub async fn get_can_set_park(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_park().await
}

#[alpaca_handler]
pub async fn get_can_set_pier_side(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_set_side_of_pier().await
}

#[alpaca_handler]
pub async fn get_can_set_right_ascension_rate(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_set_ra_rate().await
}

#[alpaca_handler]
pub async fn get_can_set_tracking(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_set_tracking().await
}

#[alpaca_handler]
pub async fn get_can_slew(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_slew().await
}

#[alpaca_handler]
pub async fn get_can_slew_alt_az(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_slew_alt_az().await
}

#[alpaca_handler]
pub async fn get_can_slew_alt_az_async(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_slew_alt_az_async().await
}

#[alpaca_handler]
pub async fn get_can_slew_async(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_slew_async().await
}

#[alpaca_handler]
pub async fn get_can_sync(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_sync().await
}

#[alpaca_handler]
pub async fn get_can_sync_alt_az(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_sync_alt_az().await
}

#[alpaca_handler]
pub async fn get_can_unpark(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_unpark().await
}

#[alpaca_handler]
pub async fn get_declination(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_dec().await
}

#[alpaca_handler]
pub async fn get_declination_rate(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_declination_rate().await
}

#[alpaca_handler]
pub async fn put_declination_rate(
    data: DeclinationRateData,
    state: &AlpacaState,
) -> AscomResult<()> {
    state.sa.set_declination_rate(data.declination_rate).await
}

#[alpaca_handler]
pub async fn get_does_refraction(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.does_refraction().await
}

#[alpaca_handler]
pub async fn put_does_refraction(data: DoesRefractionData, state: &AlpacaState) -> AscomResult<()> {
    state.sa.set_does_refraction(data.does_refraction).await
}

#[alpaca_handler]
pub async fn get_equatorial_system(state: &AlpacaState) -> AscomResult<EquatorialCoordinateType> {
    state.sa.get_equatorial_system().await
}

#[alpaca_handler]
pub async fn get_focal_length(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_focal_length().await
}

#[alpaca_handler]
pub async fn get_guide_rate_declination(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_guide_rate_declination().await
}

#[alpaca_handler]
pub async fn put_guide_rate_declination(
    data: GuideRateDeclinationData,
    state: &AlpacaState,
) -> AscomResult<()> {
    state
        .sa
        .set_guide_rate_declination(data.guide_rate_declination)
        .await
}

#[alpaca_handler]
pub async fn get_guide_rate_right_ascension(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_guide_rate_ra().await
}

#[alpaca_handler]
pub async fn put_guide_rate_right_ascension(
    data: GuideRateRightAscensionData,
    state: &AlpacaState,
) -> AscomResult<()> {
    state
        .sa
        .set_guide_rate_ra(data.guide_rate_right_ascension)
        .await
}

#[alpaca_handler]
pub async fn get_is_pulse_guiding(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.is_pulse_guiding().await
}

#[alpaca_handler]
pub async fn get_right_ascension(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_ra().await
}

#[alpaca_handler]
pub async fn get_right_ascension_rate(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_ra_rate().await
}

#[alpaca_handler]
pub async fn put_right_ascension_rate(
    data: RightAscensionRateData,
    state: &AlpacaState,
) -> AscomResult<()> {
    state.sa.set_ra_rate(data.right_ascension_rate).await
}

#[alpaca_handler]
pub async fn get_side_of_pier(state: &AlpacaState) -> AscomResult<PierSide> {
    state.sa.get_side_of_pier().await
}

#[alpaca_handler]
pub async fn put_side_of_pier(data: SideOfPierData, state: &AlpacaState) -> AscomResult<()> {
    state.sa.set_side_of_pier(data.side_of_pier).await
}

#[alpaca_handler]
pub async fn get_sidereal_time(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_sidereal_time().await
}

#[alpaca_handler]
pub async fn get_site_elevation(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_elevation().await
}

#[alpaca_handler]
pub async fn put_site_elevation(data: SiteElevationData, state: &AlpacaState) -> AscomResult<()> {
    state.sa.set_elevation(data.site_elevation).await
}

#[alpaca_handler]
pub async fn get_site_latitude(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_latitude().await
}

#[alpaca_handler]
pub async fn put_site_latitude(data: SiteLatitudeData, state: &AlpacaState) -> AscomResult<()> {
    state.sa.set_latitude(data.site_latitude).await
}

#[alpaca_handler]
pub async fn get_site_longitude(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_longitude().await
}

#[alpaca_handler]
pub async fn put_site_longitude(data: SiteLongitudeData, state: &AlpacaState) -> AscomResult<()> {
    state.sa.set_longitude(data.site_longitude).await
}

#[alpaca_handler]
pub async fn get_slewing(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.is_slewing().await
}

#[alpaca_handler]
pub async fn get_slew_settle_time(state: &AlpacaState) -> AscomResult<u32> {
    state.sa.get_slew_settle_time().await
}

#[alpaca_handler]
pub async fn put_slew_settle_time(
    data: SlewSettleTimeData,
    state: &AlpacaState,
) -> AscomResult<()> {
    if data.slew_settle_time < 0 {
        return Err(AscomError::from_msg(
            AscomErrorType::InvalidValue,
            "Slew settle time must be nonegative".to_string(),
        ));
    }
    state
        .sa
        .set_slew_settle_time(data.slew_settle_time as u32)
        .await
}

#[alpaca_handler]
pub async fn get_target_declination(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_target_declination().await
}

#[alpaca_handler]
pub async fn put_target_declination(
    data: TargetDeclinationData,
    state: &AlpacaState,
) -> AscomResult<()> {
    state.sa.set_target_dec(data.target_declination).await
}

#[alpaca_handler]
pub async fn get_target_right_ascension(state: &AlpacaState) -> AscomResult<f64> {
    state.sa.get_target_ra().await
}

#[alpaca_handler]
pub async fn put_target_right_ascension(
    data: TargetRightAscensionData,
    state: &AlpacaState,
) -> AscomResult<()> {
    state.sa.set_target_ra(data.target_right_ascension).await
}

#[alpaca_handler]
pub async fn get_tracking(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.is_tracking().await
}

#[alpaca_handler]
pub async fn put_tracking(data: TrackingData, state: &AlpacaState) -> AscomResult<()> {
    state.sa.set_is_tracking(data.tracking).await
}

#[alpaca_handler]
pub async fn get_tracking_rate(state: &AlpacaState) -> AscomResult<TrackingRate> {
    state.sa.get_tracking_rate().await
}

#[alpaca_handler]
pub async fn put_tracking_rate(data: TrackingRateData, state: &AlpacaState) -> AscomResult<()> {
    match TrackingRate::try_from(data.tracking_rate) {
        Ok(r) => state.sa.set_tracking_rate(r).await,
        Err(_e) => Err(AscomError::from_msg(
            AscomErrorType::InvalidValue,
            "Invalid Tracking Rate".to_string(),
        )),
    }
}

#[alpaca_handler]
pub async fn get_tracking_rates(state: &AlpacaState) -> AscomResult<Vec<TrackingRate>> {
    state.sa.get_tracking_rates().await
}

#[alpaca_handler]
pub async fn get_utc_date(state: &AlpacaState) -> AscomResult<DateTime<Utc>> {
    state.sa.get_utc_date().await
}

#[alpaca_handler]
pub async fn put_utc_date(data: UTCDateData, state: &AlpacaState) -> AscomResult<()> {
    let d = match data.get_utc_date() {
        Ok(d) => d,
        Err(_e) => {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidValue,
                "Date format is incorrect".to_string(),
            ))
        }
    };

    state.sa.set_utc_date(d).await
}

#[alpaca_handler]
pub async fn put_abort_slew(state: &AlpacaState) -> AscomResult<()> {
    state.sa.abort_slew().await
}

#[alpaca_handler]
pub async fn get_axis_rates(
    data: AxisData,
    state: &AlpacaState,
) -> AscomResult<Vec<AxisRateRange>> {
    state.sa.get_axis_rates(data.axis).await
}

#[alpaca_handler]
pub async fn get_can_move_axis(data: AxisData, state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_move_axis(data.axis).await
}

#[alpaca_handler]
pub async fn get_destination_side_of_pier(
    data: CoordinateData,
    state: &AlpacaState,
) -> AscomResult<PierSide> {
    state
        .sa
        .predict_destination_side_of_pier(data.right_ascension, data.declination)
        .await
}

#[alpaca_handler]
pub async fn put_find_home(state: &AlpacaState) -> AscomResult<()> {
    state.sa.find_home().await
}

#[alpaca_handler]
pub async fn put_move_axis(data: MoveAxisData, state: &AlpacaState) -> AscomResult<()> {
    let result = state.sa.move_axis(data.axis, data.rate).await;
    result
}

#[alpaca_handler]
pub async fn put_park(state: &AlpacaState) -> AscomResult<()> {
    state.sa.park().await
}

#[alpaca_handler]
pub async fn put_pulse_guide(data: PulseGuideData, state: &AlpacaState) -> AscomResult<()> {
    state.sa.pulse_guide(data.direction, data.duration).await
}

#[alpaca_handler]
pub async fn put_set_park(state: &AlpacaState) -> AscomResult<bool> {
    state.sa.can_set_park_pos().await
}

#[alpaca_handler]
pub async fn put_slew_to_alt_az(data: AltAzData, state: &AlpacaState) -> AscomResult<()> {
    state.sa.slew_to_alt_az(data.altitide, data.azimuth).await?;
    Ok(())
}

#[alpaca_handler]
pub async fn put_slew_to_alt_az_async(data: AltAzData, state: &AlpacaState) -> AscomResult<()> {
    let _finish = state
        .sa
        .slew_to_alt_az_async(data.altitide, data.azimuth)
        .await?;
    Ok(())
}

#[alpaca_handler]
pub async fn put_slew_to_coordinates(data: CoordinateData, state: &AlpacaState) -> AscomResult<()> {
    state
        .sa
        .slew_to_coordinates(data.right_ascension, data.declination)
        .await?;
    Ok(())
}

#[alpaca_handler]
pub async fn put_slew_to_coordinates_async(
    data: CoordinateData,
    state: &AlpacaState,
) -> AscomResult<()> {
    let _finish = state
        .sa
        .slew_to_coordinates_async(data.right_ascension, data.declination)
        .await?;
    Ok(())
}

#[alpaca_handler]
pub async fn put_slew_to_target(state: &AlpacaState) -> AscomResult<()> {
    state.sa.slew_to_target().await?;
    Ok(())
}

#[alpaca_handler]
pub async fn put_slew_to_target_async(state: &AlpacaState) -> AscomResult<()> {
    let _finish = state.sa.slew_to_target_async().await?;
    Ok(())
}

#[alpaca_handler]
pub async fn put_sync_to_alt_az(data: AltAzData, state: &AlpacaState) -> AscomResult<()> {
    state.sa.sync_to_alt_az(data.altitide, data.azimuth).await
}

#[alpaca_handler]
pub async fn put_sync_to_coordinates(data: CoordinateData, state: &AlpacaState) -> AscomResult<()> {
    state
        .sa
        .sync_to_coordinates(data.right_ascension, data.declination)
        .await
}

#[alpaca_handler]
pub async fn put_sync_to_target(state: &AlpacaState) -> AscomResult<()> {
    state.sa.sync_to_target().await
}

#[alpaca_handler]
pub async fn put_unpark(state: &AlpacaState) -> AscomResult<()> {
    state.sa.unpark().await
}
