use crate::rotation_direction::*;

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum TrackingDirection {
    WithTracking,
    AgainstTracking,
}

impl From<ResolvedDirection> for TrackingDirection {
    fn from(r: ResolvedDirection) -> Self {
        match (r.key.0, r.motor_direction) {
            (RotationDirectionHemisphere::North, Clockwise) => Self::WithTracking,
            (RotationDirectionHemisphere::North, CounterClockwise) => Self::AgainstTracking,
            (RotationDirectionHemisphere::South, Clockwise) => Self::AgainstTracking,
            (RotationDirectionHemisphere::South, CounterClockwise) => Self::WithTracking,
        }
    }
}

impl RotationDirection for TrackingDirection {
    fn using(self, key: RotationDirectionKey) -> ResolvedDirection {
        let motor_direction = match (key.0, self) {
            (RotationDirectionHemisphere::North, Self::WithTracking) => Clockwise,
            (RotationDirectionHemisphere::North, Self::AgainstTracking) => CounterClockwise,
            (RotationDirectionHemisphere::South, Self::WithTracking) => CounterClockwise,
            (RotationDirectionHemisphere::South, Self::AgainstTracking) => Clockwise,
        };
        ResolvedDirection {
            key,
            motor_direction,
        }
    }
}
