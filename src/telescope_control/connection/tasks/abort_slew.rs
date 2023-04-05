use crate::telescope_control::connection::ascom_state::*;

use super::*;
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use async_trait::async_trait;

pub struct AbortSlewTask {}

impl AbortSlewTask {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ShortTask for AbortSlewTask {
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
                    "Can't abort slew while parked".to_string(),
                )));
            }
            AscomState::Idle(GuideState::Idle) => return Ok(Ok(())),
            AscomState::Tracking(GuideState::Idle) => return Ok(Ok(())),
            AscomState::Slewing(SlewingState::SlewTo) => return Ok(Ok(())), // The slew aborts when we cancel the task

            AscomState::Idle(GuideState::Guiding) => unreachable!(),
            AscomState::Tracking(GuideState::Guiding) => unreachable!(),
            AscomState::Slewing(SlewingState::MoveAxis(_, GuideState::Guiding)) => unreachable!(),

            AscomState::Slewing(SlewingState::MoveAxis(rs, GuideState::Idle)) => {
                // Actually do something
                *rs
            }
        };

        let change_rate_task = if let RestorableState::Tracking(mr) = restorable_state {
            cs.ascom_state = AscomState::Tracking(GuideState::Idle);
            cs.motor.change_rate(locker.clone(), mr).await?
        } else {
            cs.ascom_state = AscomState::Idle(GuideState::Idle);
            cs.motor
                .change_rate(locker.clone(), MotionRate::ZERO)
                .await?
        };
        drop(lock);
        change_rate_task.await?;

        Ok(Ok(()))
    }
}
