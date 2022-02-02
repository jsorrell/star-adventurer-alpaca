use chrono::{TimeZone, Utc};
use star_adventurer_control::config::Config;
use star_adventurer_control::StarAdventurer;
use std::time::Duration;
#[macro_use]
extern crate assert_float_eq;

async fn create_sa(config: Option<Config>) -> StarAdventurer {
    let config = config.unwrap_or(confy::load_path("tests/test_config.toml").unwrap());
    StarAdventurer::new(&config).await.unwrap()
}

#[tokio::test]
async fn test_date() {
    let mut sa = create_sa(None).await;

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
    let mut sa = create_sa(None).await;

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

#[tokio::test]
async fn test_sync() {
    let mut sa = create_sa(None).await;
    sa.sync_to_coordinates(18., 33.).await.unwrap();
    assert_float_eq::assert_float_absolute_eq!(sa.get_ra().await.unwrap(), 18., 1E-4);
    assert_float_eq::assert_float_absolute_eq!(sa.get_dec().await.unwrap(), 33., 1E-4);
    sa.sync_to_alt_az(33., -22.).await.unwrap();
    assert_float_eq::assert_float_absolute_eq!(sa.get_altitude().await.unwrap(), 33., 1E-4);
    assert_float_eq::assert_float_absolute_eq!(sa.get_azimuth().await.unwrap(), -22., 1E-4);
    sa.set_target_ra(12.).await.unwrap();
    sa.set_target_dec(-87.).await.unwrap();
    sa.sync_to_target().await.unwrap();
    assert_float_eq::assert_float_absolute_eq!(sa.get_ra().await.unwrap(), 12., 1E-4);
    assert_float_eq::assert_float_absolute_eq!(sa.get_dec().await.unwrap(), -87., 1E-4);
}

#[tokio::test]
async fn test_slew() {
    let mut sa = create_sa(None).await;
    sa.sync_to_coordinates(0., 30.).await.unwrap();
    sa.slew_to_coordinates(-1., 14.).await.unwrap();
}
