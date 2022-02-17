use synscan::Direction;
pub use synscan::Direction::*;

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct RotationDirectionKey(pub(in crate::util::enums) RotationDirectionHemisphere);

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub(in crate::util::enums) enum RotationDirectionHemisphere {
    North,
    South,
}

impl RotationDirectionKey {
    pub fn from_hemisphere(in_north: bool) -> Self {
        if in_north {
            RotationDirectionKey(RotationDirectionHemisphere::North)
        } else {
            RotationDirectionKey(RotationDirectionHemisphere::South)
        }
    }

    pub fn reverse(self) -> Self {
        RotationDirectionKey(match self.0 {
            RotationDirectionHemisphere::North => RotationDirectionHemisphere::South,
            RotationDirectionHemisphere::South => RotationDirectionHemisphere::North,
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ResolvedDirection {
    pub(in crate::util::enums) key: RotationDirectionKey,
    pub(in crate::util::enums) motor_direction: Direction,
}

impl ResolvedDirection {
    #[allow(unused)]
    pub(crate) fn reverse(self) -> Self {
        Self {
            key: self.key.reverse(),
            motor_direction: self.motor_direction,
        }
    }
}

pub trait RotationDirection {
    fn using(self, key: RotationDirectionKey) -> ResolvedDirection;
}

impl From<ResolvedDirection> for Direction {
    fn from(r: ResolvedDirection) -> Self {
        r.motor_direction
    }
}

impl RotationDirection for Direction {
    fn using(self, key: RotationDirectionKey) -> ResolvedDirection {
        ResolvedDirection {
            key,
            motor_direction: self,
        }
    }
}

impl PartialEq<Self> for ResolvedDirection {
    fn eq(&self, other: &Self) -> bool {
        (self.key == other.key) == (self.motor_direction == other.motor_direction)
    }
}

impl Eq for ResolvedDirection {}
