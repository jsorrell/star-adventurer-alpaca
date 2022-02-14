#[macro_use]
extern crate assert_float_eq;
extern crate retry;
mod alpaca_general;
mod alpaca_telescope;
mod astro_math;
mod request;
mod response;
mod telescope_control;
mod util;

use alpaca_general::*;
use alpaca_telescope::*;
use rocket::tokio::sync::RwLock;
use rocket::{Build, Rocket};
use std::sync::atomic::AtomicU32;
use telescope_control::config::Config;
use telescope_control::StarAdventurer;
use util::*;

#[macro_use]
extern crate rocket;

pub struct AlpacaState {
    pub sa: RwLock<Option<StarAdventurer>>,
    pub sti: AtomicU32,
}

#[launch]
async fn rocket() -> Rocket<Build> {
    env_logger::init();

    let state = AlpacaState {
        sa: RwLock::new(None),
        sti: AtomicU32::new(0),
    };

    rocket::build()
        .mount(
            "/api/v1/telescope/0",
            routes![
                put_action,
                put_command_blind,
                put_command_bool,
                put_command_string,
                get_connected,
                put_connected,
                get_description,
                get_driver_info,
                get_driver_version,
                get_interface_version,
                get_name,
                get_supported_actions,
                get_alignment_mode,
                get_altitude,
                get_aperture_area,
                get_aperture_diameter,
                get_at_home,
                get_at_park,
                get_azimuth,
                get_can_find_home,
                get_can_park,
                get_can_pulse_guide,
                get_can_set_declination_rate,
                get_can_set_guide_rates,
                get_can_set_park,
                get_can_set_pier_side,
                get_can_set_right_ascension_rate,
                get_can_set_tracking,
                get_can_slew,
                get_can_slew_alt_az,
                get_can_slew_alt_az_async,
                get_can_slew_async,
                get_can_sync,
                get_can_sync_alt_az,
                get_can_unpark,
                get_declination,
                get_declination_rate,
                put_declination_rate,
                get_does_refraction,
                put_does_refraction,
                get_equatorial_system,
                get_focal_length,
                get_guide_rate_declination,
                put_guide_rate_declination,
                get_guide_rate_right_ascension,
                put_guide_rate_right_ascension,
                get_is_pulse_guiding,
                get_right_ascension,
                get_right_ascension_rate,
                put_right_ascension_rate,
                get_side_of_pier,
                put_side_of_pier,
                get_sidereal_time,
                get_site_elevation,
                put_site_elevation,
                get_site_latitude,
                put_site_latitude,
                get_site_longitude,
                put_site_longitude,
                get_slewing,
                get_slew_settle_time,
                put_slew_settle_time,
                get_target_declination,
                put_target_declination,
                get_target_right_ascension,
                put_target_right_ascension,
                get_tracking,
                put_tracking,
                get_tracking_rate,
                put_tracking_rate,
                get_tracking_rates,
                get_utc_date,
                put_utc_date,
                put_abort_slew,
                get_axis_rates,
                get_can_move_axis,
                get_destination_side_of_pier,
                put_find_home,
                put_move_axis,
                put_park,
                put_pulse_guide,
                put_set_park,
                put_slew_to_alt_az,
                put_slew_to_alt_az_async,
                put_slew_to_coordinates,
                put_slew_to_coordinates_async,
                put_slew_to_target,
                put_slew_to_target_async,
                put_sync_to_alt_az,
                put_sync_to_coordinates,
                put_sync_to_target,
                put_unpark,
            ],
        )
        .manage(state)
}
