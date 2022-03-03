use crate::util::*;

#[derive(Debug, Copy, Clone)]
pub enum MotorState {
    Stationary,
    Moving(MotionRate),
    Gotoing(Degrees),
    Changing,
}

impl MotorState {
    pub fn get_rate(&self) -> MotionRate {
        match self {
            Self::Stationary => MotionRate::ZERO,
            Self::Moving(rate) => *rate,
            Self::Gotoing(_) => panic!("No rate while Gotoing"),
            Self::Changing => panic!("No rate while Changing"),
        }
    }
}
