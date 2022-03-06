use std::mem;

use tokio::task;

use crate::telescope_control::connection::ascom_state::*;
use crate::telescope_control::connection::motor::MotorState;

use super::*;

pub struct SlewToTask {
    target_pos: Degrees,
    after_state: RestorableState,
    motor_goto_task: Option<AbortableTask<MotorResult<()>, MotorResult<()>>>,
}

impl SlewToTask {
    /// pos in degrees relative to turning on mount
    /// pos can be negative or positive or past 360 deg
    pub fn new(target_pos: Degrees) -> Self {
        Self {
            target_pos,
            after_state: RestorableState::Idle, // unused initiator
            motor_goto_task: None,
        }
    }
}

#[async_trait]
impl LongTask for SlewToTask {
    /// Slews to pos and restores the previous state when complete
    async fn start<L, T>(&mut self, locker: &L) -> MotorResult<AscomResult<WaitableTask<()>>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let mut lock = locker.write().await;
        let mut cs = HasCS::get_mut(&mut *lock)?;

        self.after_state = match &cs.ascom_state {
            AscomState::Parked => {
                return Ok(Err(AscomError::from_msg(
                    AscomErrorType::InvalidWhileParked,
                    "Can't slew to while parked".to_string(),
                )));
            }
            AscomState::Idle(GuideState::Idle) => {
                if cs.motor.get_pos().await? == self.target_pos {
                    return Ok(Ok(WaitableTask::new_completed(())));
                }
                RestorableState::Idle
            }
            AscomState::Tracking(GuideState::Idle) => {
                let rate = cs.motor.state.get_rate();
                RestorableState::Tracking(rate)
            }
            AscomState::Slewing(SlewingState::MoveAxis(rs, GuideState::Idle)) => *rs,
            AscomState::Idle(GuideState::Guiding) => unreachable!(),
            AscomState::Tracking(GuideState::Guiding) => unreachable!(),
            AscomState::Slewing(SlewingState::MoveAxis(_, GuideState::Guiding)) => unreachable!(),
            AscomState::Slewing(SlewingState::SlewTo) => unreachable!(),
        };

        if !matches!(cs.motor.get_state(), MotorState::Stationary) {
            let stop_task = cs
                .motor
                .change_rate(locker.clone(), MotionRate::ZERO)
                .await?;
            drop(lock);
            stop_task.await?;

            lock = locker.write().await;
            cs = HasCS::get_mut(&mut *lock)?;
        }

        let motor_goto_task = cs.motor.goto(locker.clone(), self.target_pos).await?;
        cs.ascom_state = AscomState::Slewing(SlewingState::SlewTo);

        self.motor_goto_task = Some(motor_goto_task.clone());

        let (slew_to_task, finisher) = WaitableTask::new();
        task::spawn(async move {
            let _result = motor_goto_task.await; // this is checked later
            finisher.finish(())
        });

        Ok(Ok(slew_to_task))
    }

    async fn complete<L, T>(&mut self, locker: &L) -> MotorResult<()>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let task = mem::replace(&mut self.motor_goto_task, None);
        if task.is_some() {
            // if none, we were already in the right place and didn't need to slew
            task.unwrap().await.unwrap()?; // Check if the slew failed
        }

        let mut lock = locker.write().await;
        let cs = HasCS::get_mut(&mut *lock)?;
        cs.ascom_state = AscomState::Idle(GuideState::Idle);

        if let RestorableState::Tracking(mr) = &self.after_state {
            let change_rate_task = cs.motor.change_rate(locker.clone(), *mr).await?;
            cs.ascom_state = AscomState::Tracking(GuideState::Idle);
            drop(lock);
            change_rate_task.await?;
        }

        Ok(())
    }

    async fn abort<L, T>(&mut self, locker: &L) -> MotorResult<()>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let task = mem::replace(&mut self.motor_goto_task, None);
        if task.is_some() {
            // if none, we were already in the right place and didn't need to slew
            task.unwrap().abort().await.unwrap()?;
        }
        self.complete(locker).await
    }

    fn get_abortable_task(&self, task: LongRunningTask) -> AbortableTaskType {
        AbortableTaskType::Slewing(task)
    }
}
