use crate::Degrees;
use std::ops::{Add, Sub};
use synscan::Direction;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct MotionRate {
    clockwise_rate: Degrees,
}

impl MotionRate {
    pub const ZERO: Self = MotionRate { clockwise_rate: 0. };
    pub fn new(rate: Degrees, direction: Direction) -> Self {
        MotionRate {
            clockwise_rate: match direction {
                Direction::Clockwise => rate,
                Direction::CounterClockwise => -rate,
            },
        }
    }

    pub fn is_zero(&self) -> bool {
        self.clockwise_rate == 0.
    }

    pub fn rate(&self) -> Degrees {
        self.clockwise_rate.abs()
    }

    /// If negative, will flip direction
    pub fn set_rate(&mut self, rate: Degrees) {
        if self.clockwise_rate < 0. {
            self.clockwise_rate = -rate
        } else {
            self.clockwise_rate = rate
        }
    }

    pub fn direction(&self) -> Direction {
        if self.clockwise_rate < 0. {
            Direction::CounterClockwise
        } else {
            Direction::Clockwise
        }
    }
}

impl Add<MotionRate> for MotionRate {
    type Output = Self;

    fn add(self, rhs: MotionRate) -> Self::Output {
        Self {
            clockwise_rate: self.clockwise_rate + rhs.clockwise_rate,
        }
    }
}

impl Sub<MotionRate> for MotionRate {
    type Output = Self;

    fn sub(self, rhs: MotionRate) -> Self::Output {
        Self {
            clockwise_rate: self.clockwise_rate - rhs.clockwise_rate,
        }
    }
}
