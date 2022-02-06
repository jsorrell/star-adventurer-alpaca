use crate::util::enums::*;
use tokio::sync::watch;

pub(crate) type TaskCanceller = watch::Sender<bool>;

#[derive(Debug)]
pub enum MotionState {
    Tracking(TrackingState),
    Slewing(SlewingState),
}

#[derive(Debug)]
pub enum SlewingState {
    ManualSlewing(TrackingState),
    GotoSlewing(Degrees, TrackingState, TaskCanceller), // ra channel target
}

impl SlewingState {
    pub fn get_state_to_restore(&self) -> TrackingState {
        let ts = match self {
            SlewingState::ManualSlewing(ts) => ts,
            SlewingState::GotoSlewing(_, ts, _) => ts,
        };

        match ts {
            TrackingState::Stationary(p) => TrackingState::Stationary(*p),
            TrackingState::Tracking(None) => TrackingState::Tracking(None),
            TrackingState::Tracking(_) => panic!("Kept autoguiding while slewing"),
        }
    }
}

#[derive(Debug)]
pub enum TrackingState {
    Stationary(bool),                // bool for parked
    Tracking(Option<TaskCanceller>), // guiding task
}
