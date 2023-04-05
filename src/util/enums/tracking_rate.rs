use super::Degrees;
use crate::rotation_direction::{RotationDirection, RotationDirectionKey};
use crate::tracking_direction::TrackingDirection;
use crate::MotionRate;
use ascom_alpaca::api::DriveRate;
use num_enum::TryFromPrimitive;

pub trait TrackingRateExt {
    fn determine_step_period(&self, frequency: u32, steps_per_rotation: u32) -> u32;
    fn determine_from_period(period: u32, frequency: u32, steps_per_rotation: u32) -> Option<Self>
    where
        Self: Sized;
    fn into_motion_rate(self, key: RotationDirectionKey) -> MotionRate;
    fn to_degrees(&self) -> f64;
}

impl TrackingRateExt for DriveRate {
    // const SIDEREAL_PERIOD: u32 = 110_359;
    // const LUNAR_PERIOD: u32 = 114_581;
    // const SOLAR_PERIOD: u32 = 110_662;
    // const KING_PERIOD: u32 = 110_390;

    fn determine_step_period(&self, frequency: u32, steps_per_rotation: u32) -> u32 {
        let exact_period: f64 =
            (frequency as f64 / steps_per_rotation as f64) * (360. / self.to_degrees());
        exact_period.round() as u32
    }

    fn determine_from_period(period: u32, frequency: u32, steps_per_rotation: u32) -> Option<Self> {
        if DriveRate::Sidereal.determine_step_period(frequency, steps_per_rotation) == period {
            Some(DriveRate::Sidereal)
        } else if DriveRate::Lunar.determine_step_period(frequency, steps_per_rotation) == period {
            Some(DriveRate::Lunar)
        } else if DriveRate::Solar.determine_step_period(frequency, steps_per_rotation) == period {
            Some(DriveRate::Solar)
        } else if DriveRate::King.determine_step_period(frequency, steps_per_rotation) == period {
            Some(DriveRate::King)
        } else {
            None
        }
    }

    fn into_motion_rate(self, key: RotationDirectionKey) -> MotionRate {
        MotionRate::new(
            self.to_degrees(),
            TrackingDirection::WithTracking.using(key).into(),
        )
    }

    fn to_degrees(&self) -> f64 {
        match self {
            DriveRate::Sidereal => 0.00417809,
            DriveRate::Lunar => 0.004024138,
            DriveRate::Solar => 0.00416665,
            DriveRate::King => 0.00417692,
        }
    }
}
