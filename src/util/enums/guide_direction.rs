use crate::rotation_direction::*;
use ascom_alpaca::api::PutPulseGuideDirection as GuideDirection;
use std::fmt::Formatter;

impl Into<GuideDirection> for ResolvedDirection {
    fn into(self) -> GuideDirection {
        match (self.motor_direction, self.key.0) {
            (Clockwise, RotationDirectionHemisphere::North) => GuideDirection::West,
            (Clockwise, RotationDirectionHemisphere::South) => GuideDirection::East,
            (CounterClockwise, RotationDirectionHemisphere::North) => GuideDirection::East,
            (CounterClockwise, RotationDirectionHemisphere::South) => GuideDirection::West,
        }
    }
}

impl RotationDirection for GuideDirection {
    fn using(self, key: RotationDirectionKey) -> ResolvedDirection {
        let d = match (self, key.0) {
            (GuideDirection::West, RotationDirectionHemisphere::North) => Clockwise,
            (GuideDirection::West, RotationDirectionHemisphere::South) => CounterClockwise,
            (GuideDirection::East, RotationDirectionHemisphere::North) => CounterClockwise,
            (GuideDirection::East, RotationDirectionHemisphere::South) => Clockwise,
            _ => panic!("Tried to get a rotation direction from {:?}", self),
        };

        ResolvedDirection {
            motor_direction: d,
            key,
        }
    }
}
