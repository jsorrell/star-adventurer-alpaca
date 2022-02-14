use crate::astro_math::Degrees;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ObservingLocation {
    pub latitude: Degrees,
    pub longitude: Degrees,
    pub elevation: f64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComSettings {
    pub path: Option<String>,
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

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TelescopeDetails {
    pub aperture: Option<f64>,
    pub aperture_area: Option<f64>,
    pub focal_length: Option<f64>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub com_settings: ComSettings,
    pub observation_location: ObservingLocation,
    pub telescope_details: TelescopeDetails,
}
