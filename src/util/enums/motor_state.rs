use crate::util::*;
use synscan::Direction;

#[derive(Debug, Clone)]
pub enum MotorState {
    Stationary(StationaryState),
    Moving(MovingState),
}

impl MotorState {
    pub fn is_moving(&self) -> bool {
        match self {
            MotorState::Moving(MovingState::Slewing(SlewingState::Gotoing { .. })) => true,
            MotorState::Moving(MovingState::Slewing(SlewingState::Stopping)) => true,
            _ => !self.determine_motion_rate().is_zero(),
        }
    }

    pub fn determine_motion_rate(&self) -> MotionRate {
        match self {
            MotorState::Stationary(StationaryState::Parked) => MotionRate::ZERO,
            MotorState::Stationary(StationaryState::Unparked(guiding_state)) => {
                guiding_state.determine_motion_rate()
            }
            MotorState::Moving(MovingState::Slewing(SlewingState::Settling { .. })) => {
                MotionRate::ZERO
            }
            MotorState::Moving(MovingState::Constant {
                guiding_state,
                motion_rate,
                ..
            }) => guiding_state.determine_motion_rate() + *motion_rate,
            MotorState::Moving(MovingState::Slewing(SlewingState::Gotoing { .. })) => {
                panic!("Unknown motion")
            }
            MotorState::Moving(MovingState::Slewing(SlewingState::Stopping)) => {
                panic!("Unknown motion")
            }
        }
    }

    pub fn is_parked(&self) -> bool {
        matches!(self, MotorState::Stationary(StationaryState::Parked))
    }

    pub fn is_slewing(&self) -> bool {
        matches!(self, MotorState::Moving(MovingState::Slewing(_)))
    }

    pub fn slew_is_stopping(&self) -> bool {
        matches!(
            self,
            MotorState::Moving(MovingState::Slewing(SlewingState::Stopping))
        )
    }

    pub fn is_tracking(&self) -> bool {
        matches!(
            self,
            MotorState::Moving(MovingState::Constant {
                state: ConstantMotionState::Tracking,
                ..
            })
        )
    }

    pub fn is_settling(&self) -> bool {
        matches!(
            self,
            MotorState::Moving(MovingState::Slewing(SlewingState::Settling { .. }))
        )
    }

    pub fn get_after_state(&self) -> Option<AfterSlewState> {
        Some(match self {
            MotorState::Moving(MovingState::Slewing(SlewingState::Gotoing {
                after_state, ..
            })) => *after_state,
            MotorState::Moving(MovingState::Constant {
                state: ConstantMotionState::MoveAxis { after_state },
                ..
            }) => *after_state,
            _ => return None,
        })
    }

    pub fn pulse_guide(
        &mut self,
        canceller: TaskCanceller,
        motion_rate: MotionRate,
    ) -> Result<(), String> {
        let guiding_state = GuidingState::Guiding {
            canceller,
            motion_rate,
        };
        *self = match self {
            MotorState::Stationary(StationaryState::Unparked(GuidingState::Idle)) => {
                MotorState::Stationary(StationaryState::Unparked(guiding_state))
            }
            MotorState::Moving(MovingState::Constant {
                state: cms,
                guiding_state: GuidingState::Idle,
                motion_rate,
            }) => MotorState::Moving(MovingState::Constant {
                state: cms.clone(),
                guiding_state,
                motion_rate: *motion_rate,
            }),
            _ => {
                return Err("State not valid for pulse guiding".to_string());
            }
        };
        Ok(())
    }

    pub fn is_guiding(&self) -> bool {
        matches!(
            self,
            MotorState::Stationary(StationaryState::Unparked(GuidingState::Guiding { .. }))
                | MotorState::Moving(MovingState::Constant {
                    guiding_state: GuidingState::Guiding { .. },
                    ..
                })
        )
    }

    pub fn is_manually_moving_axis(&self) -> bool {
        matches!(
            self,
            MotorState::Moving(MovingState::Constant {
                state: ConstantMotionState::MoveAxis { .. },
                ..
            })
        )
    }

    pub fn unpark(&mut self) {
        if self.is_parked() {
            *self = MotorState::Stationary(StationaryState::Unparked(GuidingState::Idle))
        }
    }

    pub fn as_after_slew_state(&self) -> AfterSlewState {
        match self {
            MotorState::Stationary(StationaryState::Unparked(_)) => AfterSlewState::Stationary,
            MotorState::Moving(MovingState::Constant {
                state: ConstantMotionState::Tracking,
                ..
            }) => AfterSlewState::Tracking,
            MotorState::Moving(MovingState::Constant {
                state: ConstantMotionState::MoveAxis { after_state },
                ..
            }) => *after_state,
            _ => panic!("Can't make a restorable state from {:?}", self),
        }
    }

    pub fn from_after_state(
        rs: AfterSlewState,
        tracking_rate: DriveRate,
        tracking_direction: Direction,
    ) -> Self {
        match rs {
            AfterSlewState::Parked => MotorState::Stationary(StationaryState::Parked),
            AfterSlewState::Stationary => {
                MotorState::Stationary(StationaryState::Unparked(GuidingState::Idle))
            }
            AfterSlewState::Tracking => MotorState::Moving(MovingState::Constant {
                state: ConstantMotionState::Tracking,
                guiding_state: GuidingState::Idle,
                motion_rate: MotionRate::new(tracking_rate.into(), tracking_direction),
            }),
        }
    }

    /// Only call if guiding
    pub fn clone_without_guiding(&self) -> Self {
        match self {
            MotorState::Stationary(StationaryState::Unparked(_)) => {
                MotorState::Stationary(StationaryState::Unparked(GuidingState::Idle))
            }
            MotorState::Moving(MovingState::Constant {
                state, motion_rate, ..
            }) => MotorState::Moving(MovingState::Constant {
                state: state.clone(),
                motion_rate: *motion_rate,
                guiding_state: GuidingState::Idle,
            }),
            _ => panic!("Clone without guiding caled while not guiding"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum StationaryState {
    Parked,
    Unparked(GuidingState),
}

#[derive(Debug, Clone)]
pub enum MovingState {
    Constant {
        state: ConstantMotionState,
        guiding_state: GuidingState,
        motion_rate: MotionRate,
    },
    Slewing(SlewingState),
}

#[derive(Debug)]
pub enum SlewingState {
    Gotoing {
        destination: Degrees,
        canceller: TaskCanceller,
        after_state: AfterSlewState,
    },
    Stopping,
    Settling {
        canceller: TaskCanceller,
    },
}

impl Clone for SlewingState {
    fn clone(&self) -> Self {
        match self {
            Self::Stopping => Self::Stopping,
            _ => panic!("Can't clone while gotoing"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConstantMotionState {
    MoveAxis { after_state: AfterSlewState },
    Tracking,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AfterSlewState {
    Parked,
    Stationary,
    Tracking,
}

#[derive(Debug)]
pub enum GuidingState {
    Guiding {
        canceller: TaskCanceller,
        motion_rate: MotionRate,
    },
    Idle,
}

impl GuidingState {
    pub fn determine_motion_rate(&self) -> MotionRate {
        match self {
            GuidingState::Guiding { motion_rate, .. } => *motion_rate,
            GuidingState::Idle => MotionRate::ZERO,
        }
    }
}

impl Clone for GuidingState {
    fn clone(&self) -> Self {
        match self {
            Self::Idle => Self::Idle,
            _ => panic!("Can't clone while guiding"),
        }
    }
}

impl Default for GuidingState {
    fn default() -> Self {
        GuidingState::Idle
    }
}
