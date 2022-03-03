use crate::util::*;

#[derive(Debug, Clone, Copy)]
pub enum AscomState {
    Parked,
    Idle(GuideState),
    Tracking(GuideState),
    Slewing(SlewingState),
}

impl AscomState {
    #[allow(unused)]
    pub fn is_guiding(&self) -> bool {
        matches!(self.guide_ref(), GuideState::Guiding)
    }

    pub fn is_parked(&self) -> bool {
        matches!(self, AscomState::Parked)
    }

    pub fn is_tracking(&self) -> bool {
        matches!(self, AscomState::Tracking(..))
    }

    pub fn is_slewing(&self) -> bool {
        matches!(self, AscomState::Slewing(..))
    }

    pub fn guide_ref(&self) -> &GuideState {
        match self {
            AscomState::Parked => {
                panic!("No guiding on parked")
            }
            AscomState::Idle(gs) => gs,
            AscomState::Tracking(gs) => gs,
            AscomState::Slewing(SlewingState::MoveAxis(_, gs)) => gs,
            AscomState::Slewing(SlewingState::SlewTo, ..) => {
                panic!("No guiding on SlewTo")
            }
        }
    }

    pub fn guide_ref_mut(&mut self) -> &mut GuideState {
        match self {
            AscomState::Parked => {
                panic!("No guiding on parked")
            }
            AscomState::Idle(gs) => gs,
            AscomState::Tracking(gs) => gs,
            AscomState::Slewing(SlewingState::MoveAxis(_, gs)) => gs,
            AscomState::Slewing(SlewingState::SlewTo, ..) => {
                panic!("No guiding on SlewTo")
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SlewingState {
    SlewTo, //TODO Declination slew?
    MoveAxis(RestorableState, GuideState),
}

#[derive(Debug, Clone, Copy)]
pub enum RestorableState {
    Idle,
    Tracking(MotionRate),
}

#[derive(Debug, Clone, Copy)]
pub enum GuideState {
    Idle,
    Guiding,
}
