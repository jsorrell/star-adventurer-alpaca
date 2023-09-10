use std::mem;

use tokio::task;

use crate::telescope_control::connection::ascom_state::*;
use crate::telescope_control::connection::motor::MotorState;

use super::*;

pub struct ParkTask {
    park_pos: Degrees,
    motor_goto_task: Option<AbortableTask<MotorResult<()>, MotorResult<()>>>,
}

impl ParkTask {
    pub fn new(pos: Degrees) -> Self {
        Self {
            park_pos: pos,
            motor_goto_task: None,
        }
    }
}

#[async_trait]
impl LongTask for ParkTask {
    /// Slews to pos and enters park state when complete
    /// Returns a future which will complete when the park finishes or is aborted
    async fn start<L, T>(&mut self, locker: &L) -> MotorResult<ASCOMResult<WaitableTask<()>>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let mut lock = locker.write().await;
        let mut cs = HasCS::get_mut(&mut *lock)?;

        // State checks
        match &cs.ascom_state {
            // FIXME better parking logic
            AscomState::Parked => return Ok(Ok(WaitableTask::new_completed(()))),
            AscomState::Idle(GuideState::Idle) => {
                if cs.motor.get_pos().await? == self.park_pos {
                    cs.ascom_state = AscomState::Parked;
                    return Ok(Ok(WaitableTask::new_completed(())));
                }
            }
            AscomState::Tracking(GuideState::Idle) => {}
            AscomState::Slewing(SlewingState::MoveAxis(_, GuideState::Idle)) => {}
            AscomState::Idle(GuideState::Guiding) => unreachable!(),
            AscomState::Tracking(GuideState::Guiding) => unreachable!(),
            AscomState::Slewing(SlewingState::MoveAxis(_, GuideState::Guiding)) => unreachable!(),
            AscomState::Slewing(SlewingState::SlewTo) => unreachable!(),
        }

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

        let motor_goto_task = cs.motor.goto(locker.clone(), self.park_pos).await?;
        cs.ascom_state = AscomState::Slewing(SlewingState::SlewTo);

        self.motor_goto_task = Some(motor_goto_task.clone());

        let (park_task, finisher) = WaitableTask::new();
        task::spawn(async move {
            // ignore result for now
            let _result = motor_goto_task.await; // this is checked later
            finisher.finish(())
        });

        Ok(Ok(park_task))
    }

    async fn complete<L, T>(&mut self, locker: &L) -> MotorResult<()>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let task = mem::replace(&mut self.motor_goto_task, None);
        if task.is_some() {
            // None means we were already parked
            task.unwrap().await.unwrap()?; // Check if the slew failed
        }
        let mut lock = locker.write().await;
        HasCS::get_mut(&mut *lock)?.ascom_state = AscomState::Parked;
        Ok(())
    }

    async fn abort<L, T>(&mut self, locker: &L) -> MotorResult<()>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let task = mem::replace(&mut self.motor_goto_task, None);
        if task.is_some() {
            // None means we were already parked
            task.unwrap().abort().await.unwrap()?;
        }
        let mut lock = locker.write().await;
        HasCS::get_mut(&mut *lock)?.ascom_state = AscomState::Idle(GuideState::Idle);
        Ok(())
    }

    fn get_abortable_task(&self, task: LongRunningTask) -> AbortableTaskType {
        AbortableTaskType::Parking(task)
    }
}

pub struct UnparkTask {}

impl UnparkTask {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ShortTask for UnparkTask {
    async fn run<L, T>(&mut self, locker: &L) -> MotorResult<ASCOMResult<()>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let mut lock = locker.write().await;
        let cs = HasCS::get_mut(&mut *lock)?;
        if cs.ascom_state.is_parked() {
            cs.ascom_state = AscomState::Idle(GuideState::Idle);
        }
        Ok(Ok(()))
    }
}
