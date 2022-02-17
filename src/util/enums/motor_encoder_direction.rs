use crate::rotation_direction::*;
use crate::MotorEncodingDirection::{Negative, Positive};

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum MotorEncodingDirection {
    Positive,
    Negative,
}

impl From<ResolvedDirection> for MotorEncodingDirection {
    fn from(r: ResolvedDirection) -> Self {
        match r.motor_direction {
            Clockwise => Positive,
            CounterClockwise => Negative,
        }
    }
}

impl RotationDirection for MotorEncodingDirection {
    fn using(self, key: RotationDirectionKey) -> ResolvedDirection {
        let motor_direction = match self {
            Positive => Clockwise,
            Negative => CounterClockwise,
        };
        ResolvedDirection {
            key,
            motor_direction,
        }
    }
}

impl MotorEncodingDirection {
    pub fn get_sign_f64(&self) -> f64 {
        match self {
            Positive => 1.,
            Negative => -1.,
        }
    }
}
