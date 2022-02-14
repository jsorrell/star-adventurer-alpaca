use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::Formatter;
use crate::rotation_direction::*;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize_repr, Deserialize_repr, FromFormField)]
#[repr(u8)]
pub enum GuideDirection {
    #[field(value = "0")]
    North = 0,
    #[field(value = "1")]
    South = 1,
    #[field(value = "2")]
    East = 2,
    #[field(value = "3")]
    West = 3,
}

impl core::fmt::Display for GuideDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GuideDirection::North => write!(f, "North"),
            GuideDirection::South => write!(f, "South"),
            GuideDirection::East => write!(f, "East"),
            GuideDirection::West => write!(f, "West"),
        }
    }
}

impl From<ResolvedDirection> for GuideDirection {
    fn from(d: ResolvedDirection) -> Self {
        match (d.motor_direction, d.key.0) {
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
            _ => panic!("Tried to get a rotation direction from {}", self)
        };

        ResolvedDirection{motor_direction: d, key}
    }
}
