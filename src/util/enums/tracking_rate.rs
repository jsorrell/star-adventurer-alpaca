use super::Degrees;
use crate::rotation_direction::{RotationDirection, RotationDirectionKey};
use crate::tracking_direction::TrackingDirection;
use crate::MotionRate;
use num_enum::TryFromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(
    Debug,
    Eq,
    PartialEq,
    Copy,
    Clone,
    Serialize_repr,
    Deserialize_repr,
    FromFormField,
    TryFromPrimitive,
)]
#[repr(i32)]
pub enum TrackingRate {
    #[field(value = "0")]
    Sidereal = 0,
    #[field(value = "1")]
    Lunar = 1,
    #[field(value = "2")]
    Solar = 2,
    #[field(value = "3")]
    King = 3,
}

impl TrackingRate {
    // const SIDEREAL_PERIOD: u32 = 110_359;
    // const LUNAR_PERIOD: u32 = 114_581;
    // const SOLAR_PERIOD: u32 = 110_662;
    // const KING_PERIOD: u32 = 110_390;

    pub fn determine_step_period(&self, frequency: u32, steps_per_rotation: u32) -> u32 {
        let exact_period: f64 =
            (frequency as f64 / steps_per_rotation as f64) * (360. / Degrees::from(*self));
        exact_period.round() as u32
    }

    pub fn determine_from_period(
        period: u32,
        frequency: u32,
        steps_per_rotation: u32,
    ) -> Option<Self> {
        if TrackingRate::Sidereal.determine_step_period(frequency, steps_per_rotation) == period {
            Some(TrackingRate::Sidereal)
        } else if TrackingRate::Lunar.determine_step_period(frequency, steps_per_rotation) == period
        {
            Some(TrackingRate::Lunar)
        } else if TrackingRate::Solar.determine_step_period(frequency, steps_per_rotation) == period
        {
            Some(TrackingRate::Solar)
        } else if TrackingRate::King.determine_step_period(frequency, steps_per_rotation) == period
        {
            Some(TrackingRate::King)
        } else {
            None
        }
    }

    pub fn into_motion_rate(self, key: RotationDirectionKey) -> MotionRate {
        MotionRate::new(
            self.into(),
            TrackingDirection::WithTracking.using(key).into(),
        )
    }
}

impl From<TrackingRate> for Degrees {
    fn from(rate: TrackingRate) -> Self {
        match rate {
            TrackingRate::Sidereal => 0.00417809,
            TrackingRate::Lunar => 0.004024138,
            TrackingRate::Solar => 0.00416665,
            TrackingRate::King => 0.00417692,
        }
    }
}
