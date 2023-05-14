use crate::rotation_direction::*;
use ascom_alpaca::api::PutPulseGuideDirection;
use std::fmt::Formatter;

impl From<ResolvedDirection> for PutPulseGuideDirection {
    fn from(dir: ResolvedDirection) -> PutPulseGuideDirection {
        match (dir.motor_direction, dir.key.0) {
            (Clockwise, RotationDirectionHemisphere::North) => PutPulseGuideDirection::West,
            (Clockwise, RotationDirectionHemisphere::South) => PutPulseGuideDirection::East,
            (CounterClockwise, RotationDirectionHemisphere::North) => PutPulseGuideDirection::East,
            (CounterClockwise, RotationDirectionHemisphere::South) => PutPulseGuideDirection::West,
        }
    }
}

impl RotationDirection for PutPulseGuideDirection {
    fn using(self, key: RotationDirectionKey) -> ResolvedDirection {
        let d = match (self, key.0) {
            (PutPulseGuideDirection::West, RotationDirectionHemisphere::North) => Clockwise,
            (PutPulseGuideDirection::West, RotationDirectionHemisphere::South) => CounterClockwise,
            (PutPulseGuideDirection::East, RotationDirectionHemisphere::North) => CounterClockwise,
            (PutPulseGuideDirection::East, RotationDirectionHemisphere::South) => Clockwise,
            _ => panic!("Tried to get a rotation direction from {:?}", self),
        };

        ResolvedDirection {
            motor_direction: d,
            key,
        }
    }
}
