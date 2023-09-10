use crate::telescope_control::connection::ascom_state::*;

use super::*;
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult};

pub struct StopTrackingTask {}

impl StopTrackingTask {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ShortTask for StopTrackingTask {
    async fn run<L, T>(&mut self, locker: &L) -> MotorResult<ASCOMResult<()>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let mut lock = locker.write().await;
        let cs = HasCS::get_mut(&mut *lock)?;

        match &cs.ascom_state {
            AscomState::Parked => {
                return Ok(Err(ASCOMError::new(
                    ASCOMErrorCode::INVALID_WHILE_PARKED,
                    "Can't stop tracking while parked".to_string(),
                )));
            }
            AscomState::Idle(GuideState::Idle) => {
                return Ok(Ok(()));
            }
            AscomState::Slewing(SlewingState::MoveAxis(_, GuideState::Idle)) => {
                return Ok(Err(ASCOMError::invalid_operation("Can't stop tracking while moving axis")));
            }
            AscomState::Idle(GuideState::Guiding) => unreachable!(),
            AscomState::Tracking(GuideState::Guiding) => unreachable!(),
            AscomState::Slewing(SlewingState::MoveAxis(_, GuideState::Guiding)) => unreachable!(),
            AscomState::Slewing(SlewingState::SlewTo) => unreachable!(),
            AscomState::Tracking(GuideState::Idle) => {
                // Only state we do anything
            }
        }

        let change_rate_task = cs
            .motor
            .change_rate(locker.clone(), MotionRate::ZERO)
            .await?;
        cs.ascom_state = AscomState::Idle(GuideState::Idle);
        drop(lock);
        change_rate_task.await?;

        Ok(Ok(()))
    }
}

/// Can also used to change tracking rate
pub struct StartTrackingTask {
    rate: MotionRate,
}

impl StartTrackingTask {
    pub fn new(rate: MotionRate) -> Self {
        Self { rate }
    }
}

#[async_trait]
impl ShortTask for StartTrackingTask {
    async fn run<L, T>(&mut self, locker: &L) -> MotorResult<ASCOMResult<()>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let mut lock = locker.write().await;
        let cs = HasCS::get_mut(&mut *lock)?;

        match &cs.ascom_state {
            AscomState::Parked => {
                return Ok(Err(ASCOMError::new(
                    ASCOMErrorCode::INVALID_WHILE_PARKED,
                    "Can't start tracking while parked".to_string(),
                )));
            }
            AscomState::Idle(GuideState::Idle) => {}
            AscomState::Tracking(GuideState::Idle) => {
                if cs.motor.state.get_rate() == self.rate {
                    return Ok(Ok(()));
                }
            }
            AscomState::Slewing(SlewingState::MoveAxis(_, GuideState::Idle)) => {
                return Ok(Err(ASCOMError::invalid_operation("Can't start tracking while moving axis")));
            }
            AscomState::Idle(GuideState::Guiding) => unreachable!(),
            AscomState::Tracking(GuideState::Guiding) => unreachable!(),
            AscomState::Slewing(SlewingState::MoveAxis(_, GuideState::Guiding)) => unreachable!(),
            AscomState::Slewing(SlewingState::SlewTo) => unreachable!(),
        }

        let change_rate_task = cs.motor.change_rate(locker.clone(), self.rate).await?;
        cs.ascom_state = AscomState::Tracking(GuideState::Idle);
        drop(lock);
        change_rate_task.await?;

        Ok(Ok(()))
    }
}

/// Can also used to change tracking rate
pub struct UpdateTrackingRateTask {
    rate: MotionRate,
}

impl UpdateTrackingRateTask {
    pub fn new(rate: MotionRate) -> Self {
        Self { rate }
    }
}

#[async_trait]
impl ShortTask for UpdateTrackingRateTask {
    async fn run<L, T>(&mut self, locker: &L) -> MotorResult<ASCOMResult<()>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let mut lock = locker.write().await;
        let cs = HasCS::get_mut(&mut *lock)?;

        match &cs.ascom_state {
            AscomState::Parked => return Ok(Ok(())),
            AscomState::Idle(GuideState::Idle) => return Ok(Ok(())),
            AscomState::Tracking(GuideState::Idle) => {
                if cs.motor.state.get_rate() == self.rate {
                    return Ok(Ok(()));
                }
                // Only do anything if tracking currently
            }
            AscomState::Slewing(SlewingState::MoveAxis(_, GuideState::Idle)) => return Ok(Ok(())),
            AscomState::Idle(GuideState::Guiding) => unreachable!(),
            AscomState::Tracking(GuideState::Guiding) => unreachable!(),
            AscomState::Slewing(SlewingState::MoveAxis(_, GuideState::Guiding)) => unreachable!(),
            AscomState::Slewing(SlewingState::SlewTo) => unreachable!(),
        }

        let change_rate_task = cs.motor.change_rate(locker.clone(), self.rate).await?;
        cs.ascom_state = AscomState::Tracking(GuideState::Idle);
        drop(lock);
        change_rate_task.await?;

        Ok(Ok(()))
    }
}
