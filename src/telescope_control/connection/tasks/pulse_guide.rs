use std::time::Duration;

use tokio::task;
use tokio::time::{sleep_until, Instant};

use crate::telescope_control::connection::ascom_state::*;

use super::*;

const EARLY_RETURN_MILLIS: u64 = 5;

pub struct PulseGuideTask {
    guide_rate: MotionRate,
    duration: Duration,
    finish_time: Instant,
}

impl PulseGuideTask {
    pub fn new(guide_rate: MotionRate, duration: Duration) -> Self {
        Self {
            guide_rate,
            duration,
            finish_time: Instant::now(), // temporary, unused value
        }
    }
}

#[async_trait]
impl LongTask for PulseGuideTask {
    /// Pulse Guides in the given direction for the given time
    /// Restores when complete
    /// Pulse guide has the lowest priority and can be cancelled by calling other methods
    async fn start<L, T>(&mut self, locker: &L) -> MotorResult<AscomResult<WaitableTask<()>>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let mut lock = locker.write().await;
        let cs = HasCS::get_mut(&mut *lock)?;

        match &cs.ascom_state {
            AscomState::Parked => {
                return Ok(Err(AscomError::from_msg(
                    AscomErrorType::InvalidWhileParked,
                    "Can't pulse guide while parked".to_string(),
                )));
            }
            AscomState::Slewing(SlewingState::SlewTo) => {
                unreachable!()
            }
            st => {
                assert!(!st.is_guiding())
            }
        };

        let current_rate = cs.motor.get_state().get_rate();

        let finish_time = Instant::now() + self.duration;
        self.finish_time = finish_time;
        let rate_change_task = cs
            .motor
            .change_rate(locker.clone(), current_rate + self.guide_rate)
            .await?;
        *cs.ascom_state.guide_ref_mut() = GuideState::Guiding;
        drop(lock);
        rate_change_task.await?;

        let (guide_task, finisher) = WaitableTask::new();
        task::spawn(async move {
            sleep_until(finish_time - Duration::from_millis(EARLY_RETURN_MILLIS)).await; // Come back early so we can spin sleep the rest
            finisher.finish(())
        });

        Ok(Ok(guide_task))
    }

    async fn complete<L, T>(&mut self, locker: &L) -> MotorResult<()>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let mut lock = locker.write().await;
        let cs = HasCS::get_mut(&mut *lock)?;
        spin_sleep::sleep(self.finish_time - Instant::now());
        let current_rate = cs.motor.get_state().get_rate();
        let rate_change_task = cs
            .motor
            .change_rate(locker.clone(), current_rate - self.guide_rate)
            .await?;
        *cs.ascom_state.guide_ref_mut() = GuideState::Idle;
        drop(lock);
        rate_change_task.await
    }

    async fn abort<L, T>(&mut self, locker: &L) -> MotorResult<()>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync,
    {
        let mut lock = locker.write().await;
        let cs = HasCS::get_mut(&mut *lock)?;
        *cs.ascom_state.guide_ref_mut() = GuideState::Idle; // marker to show we've remembered to abort
        Ok(())
    }

    fn get_abortable_task(&self, task: LongRunningTask) -> AbortableTaskType {
        AbortableTaskType::Guiding(task)
    }
}
