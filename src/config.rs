use crate::astro_math::Degrees;
use crate::rotation_direction::RotationDirectionKey;
use crate::{Hours, PierSide};
use serde::{Deserialize, Serialize};
use synscan::AutoGuideSpeed;

/* Config */
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub com: ComSettings,
    pub observation_location: ObservingLocation,
    pub telescope_details: TelescopeDetails,
    pub initialization: Initialization,
    pub other: OtherSettings,
}

/* Serial Port Settings */
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
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
#[serde(rename_all = "kebab-case")]
pub struct ObservingLocation {
    pub latitude: Degrees,
    pub longitude: Degrees,
    pub elevation: f64,
}

impl ObservingLocation {
    // TODO use a config value rather that determining from the latitude
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
#[serde(rename_all = "kebab-case")]
pub struct TelescopeDetails {
    pub aperture: Option<f64>,
    pub aperture_area: Option<f64>,
    pub focal_length: Option<f64>,
}

/* Initialization Settings */
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Initialization {
    #[serde(default)]
    pub hour_angle: Hours, // Mechanical HA
    #[serde(default)]
    pub declination: Degrees,
    #[serde(with = "pier_side")]
    pub pier_side: PierSide,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Default for Initialization {
    fn default() -> Self {
        Initialization {
            // Facing toward the equator and meridian on the east side
            hour_angle: -6.,
            declination: 0.,
            pier_side: PierSide::East,
        }
    }
}

mod pier_side {
    use crate::PierSide;
    use core::fmt::Formatter;
    use serde::de::{Error, Visitor};
    use serde::{Deserializer, Serializer};

    struct PierSideVisitor;
    impl<'de> Visitor<'de> for PierSideVisitor {
        type Value = PierSide;

        fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
            formatter.write_str("East or West")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            let lower = v.to_lowercase();
            Ok(match &*lower {
                "east" => PierSide::East,
                "west" => PierSide::West,
                _ => return Err(E::custom(format!("unknown pier side: \"{}\"", v))),
            })
        }
    }

    pub fn serialize<S>(s: &PierSide, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match s {
            PierSide::East => "East",
            PierSide::West => "West",
            PierSide::Unknown => unreachable!(),
        })
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<PierSide, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(PierSideVisitor)
    }
}

/* Other Settings */
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OtherSettings {
    pub slew_settle_time: u32,
    #[serde(skip_serializing_if = "is_false")]
    pub instant_dec_slew: bool,
    #[serde(default = "auto_guide_speed::default", with = "auto_guide_speed")]
    pub auto_guide_speed: AutoGuideSpeed,
    pub park_hour_angle: Hours,  // Mechanical
    pub mount_limit_east: Hours, // Mechanical
    pub mount_limit_west: Hours, // Mechanical
}

impl Default for OtherSettings {
    fn default() -> Self {
        Self {
            slew_settle_time: 5,
            instant_dec_slew: true,
            auto_guide_speed: auto_guide_speed::default(),
            park_hour_angle: 0.,
            mount_limit_east: 18., // Horizontal on the east
            mount_limit_west: 6.,  // Horizontal on the west
        }
    }
}

mod auto_guide_speed {
    use core::fmt::Formatter;
    use serde::de::{Error, Visitor};
    use serde::{Deserializer, Serializer};
    use synscan::AutoGuideSpeed;

    struct AutoGuideSpeedVisitor;
    impl<'de> Visitor<'de> for AutoGuideSpeedVisitor {
        type Value = AutoGuideSpeed;

        fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
            formatter.write_str("1/8, 1/4, 1/2, 3/4, 1")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            let lower = v.to_lowercase();
            Ok(match &*lower {
                "1/8" => AutoGuideSpeed::Eighth,
                "1/4" => AutoGuideSpeed::Quarter,
                "1/2" => AutoGuideSpeed::Half,
                "3/4" => AutoGuideSpeed::ThreeQuarters,
                "1" => AutoGuideSpeed::One,
                _ => return Err(E::custom(format!("unknown auto-guide speed: \"{}\"", v))),
            })
        }
    }

    pub fn default() -> AutoGuideSpeed {
        AutoGuideSpeed::Half
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<AutoGuideSpeed, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(AutoGuideSpeedVisitor)
    }

    pub fn serialize<S>(s: &AutoGuideSpeed, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match s {
            AutoGuideSpeed::One => "1",
            AutoGuideSpeed::ThreeQuarters => "3/4",
            AutoGuideSpeed::Half => "1/2",
            AutoGuideSpeed::Quarter => "1/4",
            AutoGuideSpeed::Eighth => "1/8",
        })
    }
}
