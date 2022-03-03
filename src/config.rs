use crate::astro_math::Degrees;
use crate::rotation_direction::RotationDirectionKey;
use serde::{Deserialize, Serialize};

/* Config */
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub com_settings: ComSettings,
    pub observation_location: ObservingLocation,
    pub telescope_details: TelescopeDetails,
    pub other_settings: OtherSettings,
}

/* Serial Port Settings */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComSettings {
    pub path: Option<String>, // None for automatic
    pub timeout_millis: u32,
}

impl Default for ComSettings {
    fn default() -> Self {
        Self {
            path: None,
            timeout_millis: 50,
        }
    }
}

/* Location */
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ObservingLocation {
    pub latitude: Degrees,
    pub longitude: Degrees,
    pub elevation: f64,
}

impl ObservingLocation {
    pub fn in_north(&self) -> bool {
        0. < self.latitude
    }

    pub fn get_rotation_direction_key(&self) -> RotationDirectionKey {
        RotationDirectionKey::from_hemisphere(self.in_north())
    }
}

impl Default for ObservingLocation {
    fn default() -> Self {
        Self {
            latitude: 51.47,
            longitude: 0.0,
            elevation: 15.0,
        }
    }
}

/* Telescope Settings */
#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TelescopeDetails {
    pub aperture: Option<f64>,
    pub aperture_area: Option<f64>,
    pub focal_length: Option<f64>,
}

/* Other Settings */
#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct OtherSettings {
    pub slew_settle_time: u32,
    pub instant_dec_slew: bool,
}
