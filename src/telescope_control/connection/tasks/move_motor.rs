use crate::telescope_control::connection::ascom_state::*;

use super::*;
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult};

pub struct MoveMotorTask {
    rate: MotionRate,
}

impl MoveMotorTask {
    pub fn new(rate: MotionRate) -> Self {
        Self { rate }
    }
}

#[async_trait]
impl ShortTask for MoveMotorTask {
    async fn run<L, T>(&mut self, locker: &L) -> MotorResult<ASCOMResult<()>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let mut lock = locker.write().await;
        let cs = HasCS::get_mut(&mut *lock)?;

        let restorable_state = match &cs.ascom_state {
            AscomState::Parked => {
                return Ok(Err(ASCOMError::new(
                    ASCOMErrorCode::INVALID_WHILE_PARKED,
                    "Can't move axis while parked".to_string(),
                )));
            }
            AscomState::Idle(GuideState::Idle) => RestorableState::Idle,
            AscomState::Tracking(GuideState::Idle) => {
                let rate = cs.motor.state.get_rate();
                RestorableState::Tracking(rate)
            }
            AscomState::Slewing(SlewingState::MoveAxis(rs, GuideState::Idle)) => {
                // Nothing to do
                if cs.motor.state.get_rate() == self.rate {
                    return Ok(Ok(()));
                } else {
                    *rs
                }
            }
            AscomState::Idle(GuideState::Guiding) => unreachable!(),
            AscomState::Tracking(GuideState::Guiding) => unreachable!(),
            AscomState::Slewing(SlewingState::MoveAxis(_, GuideState::Guiding)) => unreachable!(),
            AscomState::Slewing(SlewingState::SlewTo) => unreachable!(),
        };

        let change_rate_task = cs.motor.change_rate(locker.clone(), self.rate).await?;
        cs.ascom_state =
            AscomState::Slewing(SlewingState::MoveAxis(restorable_state, GuideState::Idle));
        drop(lock);
        change_rate_task.await?;

        Ok(Ok(()))
    }
}
