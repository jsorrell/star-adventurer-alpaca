use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;

use synscan::AutoGuideSpeed;
use tokio::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::{select, task};

use ascom_state::*;
pub use motor::consts;
use potential_connection::*;

use crate::telescope_control::connection::motor::{MotorBuilder, MotorError, MotorResult};
use crate::telescope_control::connection::tasks::*;
use crate::util::*;

mod ascom_state;
mod motor;
mod potential_connection;
mod tasks;

pub type ConnectionBuilder = MotorBuilder;

#[derive(Clone)]
pub struct Connection {
    c: Arc<RwLock<PotentialConnection>>,
    task_lock: Arc<Mutex<AbortableTaskType>>,
    cb: ConnectionBuilder,
}

pub struct CSReadLock<'a> {
    con_lock: RwLockReadGuard<'a, PotentialConnection>,
}

impl Deref for CSReadLock<'_> {
    type Target = ConnectedState;

    fn deref(&self) -> &Self::Target {
        if let PotentialConnection::Connected(cs) = &*self.con_lock {
            cs
        } else {
            unreachable!()
        }
    }
}

pub struct CSWriteLock<'a> {
    _con_lock: RwLockWriteGuard<'a, PotentialConnection>,
}

impl Deref for CSWriteLock<'_> {
    type Target = ConnectedState;

    fn deref(&self) -> &Self::Target {
        if let PotentialConnection::Connected(cs) = &*self._con_lock {
            cs
        } else {
            unreachable!()
        }
    }
}

impl DerefMut for CSWriteLock<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let PotentialConnection::Connected(cs) = &mut *self._con_lock {
            cs
        } else {
            unreachable!()
        }
    }
}

impl Connection {
    pub fn new(cb: ConnectionBuilder) -> Self {
        Connection {
            c: Arc::new(RwLock::new(PotentialConnection::Disconnected)),
            task_lock: Arc::new(Mutex::new(AbortableTaskType::None)),
            cb,
        }
    }

    pub async fn connect(&self, autoguide_speed: AutoGuideSpeed) -> AscomResult<()> {
        let mut con = self.c.write().await;
        if matches!(*con, PotentialConnection::Connected(_)) {
            return Ok(());
        }

        let motor_result = self.cb.create().await;
        if let Err(e) = motor_result {
            return AscomResult::Err(AscomError {
                error_number: 0x600,
                error_message: format!("Could not connect to motor controller: {}", e),
            });
        }

        let mut motor = motor_result.unwrap();
        if let Err(e) = motor.set_autoguide_speed(autoguide_speed).await {
            return AscomResult::Err(AscomError {
                error_number: 0x600,
                error_message: format!("Error setting autoguide speed: {}", e),
            });
        }

        // TODO currently stopping the motor on connection. We should restore the state maybe
        let result = motor.change_rate_open(MotionRate::ZERO).await;
        if let Err(e) = result {
            return AscomResult::Err(AscomError {
                error_number: 0x600,
                error_message: format!("Error stopping motor: {}", e),
            });
        };

        let state = AscomState::Idle(GuideState::Idle);

        let cs = ConnectedState {
            ascom_state: state,
            motor,
        };

        *con = PotentialConnection::Connected(cs);

        Ok(())
    }

    pub async fn disconnect(&self) {
        let mut con = self.c.write().await;
        *con = PotentialConnection::Disconnected;
    }

    pub async fn read_con(&self) -> AscomResult<CSReadLock<'_>> {
        let lock = self.c.read().await;
        match &*lock {
            PotentialConnection::Connected(_) => Ok(CSReadLock { con_lock: lock }),
            PotentialConnection::Disconnected => Err(AscomError::not_connected()),
        }
    }

    pub async fn write_con(&self) -> AscomResult<CSWriteLock<'_>> {
        let lock = self.c.write().await;
        match &*lock {
            PotentialConnection::Connected(_) => Ok(CSWriteLock { _con_lock: lock }),
            PotentialConnection::Disconnected => Err(AscomError::not_connected()),
        }
    }

    /* GET/Read */

    pub async fn get_min_speed(&self) -> AscomResult<Degrees> {
        let lock = self.read_con().await?;
        Ok(lock.motor.get_min_speed())
    }

    pub async fn get_max_speed(&self) -> AscomResult<Degrees> {
        let lock = self.read_con().await?;
        Ok(lock.motor.get_max_speed())
    }

    pub async fn get_pos(&self) -> AscomResult<Degrees> {
        let lock = self.read_con().await?;
        self.check_motor_result(lock.motor.get_pos().await).await
    }

    pub async fn is_guiding(&self) -> AscomResult<bool> {
        let lock = self.read_con().await?;
        Ok(lock.ascom_state.is_guiding())
    }

    pub async fn is_slewing(&self) -> AscomResult<bool> {
        let lock = self.read_con().await?;
        Ok(lock.ascom_state.is_slewing())
    }

    pub async fn is_parked(&self) -> AscomResult<bool> {
        let lock = self.read_con().await?;
        Ok(lock.ascom_state.is_parked())
    }

    pub async fn is_tracking(&self) -> AscomResult<bool> {
        let lock = self.read_con().await?;
        Ok(lock.ascom_state.is_tracking())
    }

    /* PUT/Write */

    /// Convenience function that internally locks and unlocks the connection
    pub async fn set_autoguide_speed(&self, speed: AutoGuideSpeed) -> AscomResult<()> {
        let mut lock = self.write_con().await?;
        self.check_motor_result(lock.motor.set_autoguide_speed(speed).await)
            .await
    }

    async fn check_motor_result<T>(&self, res: MotorResult<T>) -> AscomResult<T> {
        match res {
            Ok(r) => return Ok(r),
            Err(MotorError::Disconnected) => {}
            Err(motor_error) => {
                // Error means we disconnect
                log::error!("Disconnecting due to motor error {}", motor_error);
                self.disconnect().await;
            }
        }
        Err(AscomError::not_connected())
    }

    async fn run_short_task(&self, mut short_task: impl ShortTask) -> AscomResult<()> {
        // Ensure we're connected
        self.read_con().await?;
        self.check_motor_result(short_task.run(&self.c.clone()).await)
            .await?
    }

    async fn run_long_task(
        &self,
        mut long_task: impl LongTask + Send + 'static,
        mut task_lock: MutexGuard<'_, AbortableTaskType>,
    ) -> AscomResult<WaitableTask<AbortResult<AscomResult<()>, AscomResult<()>>>> {
        // Ensure we're connected
        self.read_con().await?;

        let locker = self.c.clone();
        let waiter = self
            .check_motor_result(long_task.start(&locker).await)
            .await??;

        let (task, finisher) = AbortableTask::new();
        let abort_waiter = task.get_abort_waiter();

        *task_lock = long_task.get_abortable_task(task.clone());

        let connection = self.clone();

        task::spawn(async move {
            let lock_task = connection.task_lock.lock();
            let completion = async move {
                waiter.await;
                lock_task.await
            };

            select! {
                mut task_lock = completion => {
                    *task_lock = AbortableTaskType::None;
                    let result = connection.check_motor_result(long_task.complete(&locker).await).await;
                    finisher.finish(result);
                }
                _ = abort_waiter => {
                    let result = connection.check_motor_result(long_task.abort(&locker).await).await;
                    finisher.aborted(result);
                }
            }
        });

        Ok(task.into())
    }

    pub async fn start_tracking(&self, rate: MotionRate) -> AscomResult<()> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidValue,
                    "Can't start tracking while slewing".to_string(),
                ));
            }
            AbortableTaskType::Parking(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidValue,
                    "Can't start tracking while parking".to_string(),
                ));
            }
            AbortableTaskType::Guiding(guide_task) => {
                guide_task.abort().await.unwrap()?;
            }
            AbortableTaskType::None => {}
        }
        *task_lock = AbortableTaskType::None;

        let start_tracking_task = StartTrackingTask::new(rate);

        self.run_short_task(start_tracking_task).await
    }

    pub async fn stop_tracking(&self) -> AscomResult<()> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidValue,
                    "Can't stop tracking while slewing".to_string(),
                ));
            }
            AbortableTaskType::Parking(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidValue,
                    "Can't stop tracking while parking".to_string(),
                ));
            }
            AbortableTaskType::Guiding(guide_task) => {
                guide_task.abort().await.unwrap()?;
            }
            AbortableTaskType::None => {}
        }
        *task_lock = AbortableTaskType::None;

        let stop_tracking_task = StopTrackingTask::new();

        self.run_short_task(stop_tracking_task).await
    }

    pub async fn update_tracking_rate(&self, rate: MotionRate) -> AscomResult<()> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => return Ok(()),
            AbortableTaskType::Parking(_) => return Ok(()),
            AbortableTaskType::Guiding(guide_task) => {
                if self.is_tracking().await? {
                    guide_task.abort().await.unwrap()?;
                } else {
                    return Ok(());
                }
            }
            AbortableTaskType::None => {}
        }
        *task_lock = AbortableTaskType::None;

        let update_tracking_rate_task = UpdateTrackingRateTask::new(rate);

        self.run_short_task(update_tracking_rate_task).await
    }

    pub async fn move_motor(&self, rate: MotionRate) -> AscomResult<()> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidOperation,
                    "Can't move motor while slewing".to_string(),
                ));
            }
            AbortableTaskType::Parking(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidOperation,
                    "Can't move motor while parking".to_string(),
                ));
            }
            AbortableTaskType::Guiding(guide_task) => {
                guide_task.abort().await.unwrap()?;
            }
            AbortableTaskType::None => {}
        }
        *task_lock = AbortableTaskType::None;

        let move_motor_task = MoveMotorTask::new(rate);

        self.run_short_task(move_motor_task).await
    }

    pub async fn pulse_guide(
        &self,
        guide_rate: MotionRate,
        duration: Duration,
    ) -> AscomResult<WaitableTask<AbortResult<AscomResult<()>, AscomResult<()>>>> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidOperation,
                    "Can't guide while slewing".to_string(),
                ));
            }
            AbortableTaskType::Parking(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidOperation,
                    "Can't guide while parking".to_string(),
                ));
            }
            AbortableTaskType::Guiding(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidOperation,
                    "Already guiding".to_string(),
                ));
            }
            AbortableTaskType::None => {}
        }
        *task_lock = AbortableTaskType::None;

        let pulse_guide_task = PulseGuideTask::new(guide_rate, duration);

        self.run_long_task(pulse_guide_task, task_lock).await
    }

    /// pos in degrees relative to turning on mount
    /// pos can be negative or positive or past 360 deg
    pub async fn slew_to(
        &self,
        target_pos: Degrees,
    ) -> AscomResult<WaitableTask<AbortResult<AscomResult<()>, AscomResult<()>>>> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidValue,
                    "Already slewing".to_string(),
                ));
            }
            AbortableTaskType::Parking(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidValue,
                    "Can't slew while parking".to_string(),
                ));
            }
            AbortableTaskType::Guiding(guide_task) => {
                guide_task.abort().await.unwrap()?;
            }
            AbortableTaskType::None => {}
        }

        let slew_task = SlewToTask::new(target_pos);

        self.run_long_task(slew_task, task_lock).await
    }

    pub async fn park(
        &self,
        park_pos: Degrees,
    ) -> AscomResult<WaitableTask<AbortResult<AscomResult<()>, AscomResult<()>>>> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidValue,
                    "Can't park while slewing".to_string(),
                ));
            }
            AbortableTaskType::Parking(_) => {
                return Err(AscomError::from_msg(
                    AscomErrorType::InvalidValue,
                    "Already parking".to_string(),
                ));
            }
            AbortableTaskType::Guiding(guide_task) => {
                guide_task.abort().await.unwrap()?;
            }
            AbortableTaskType::None => {}
        }

        let park_task = ParkTask::new(park_pos);

        self.run_long_task(park_task, task_lock).await
    }

    pub async fn unpark(&self) -> AscomResult<()> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Ok(()); // Nothing to do
            }
            AbortableTaskType::Parking(park_task) => {
                park_task.abort().await.unwrap()?; // TODO do we abort this or not?
            }
            AbortableTaskType::Guiding(_) => {
                return Ok(()); // Nothing to do
            }
            AbortableTaskType::None => {}
        }
        *task_lock = AbortableTaskType::None;

        let unpark_task = UnparkTask::new();

        self.run_short_task(unpark_task).await
    }

    pub async fn abort_slew(&self) -> AscomResult<()> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(slew_task) => {
                slew_task.abort().await.unwrap()?;
            }
            AbortableTaskType::Parking(park_task) => {
                park_task.abort().await.unwrap()?;
            }
            AbortableTaskType::Guiding(guide_task) => {
                guide_task.abort().await.unwrap()?;
            }
            AbortableTaskType::None => {}
        }
        *task_lock = AbortableTaskType::None;

        let abort_slew_task = AbortSlewTask::new();
        self.run_short_task(abort_slew_task).await
    }
}

impl Deref for Connection {
    type Target = Arc<RwLock<PotentialConnection>>;

    fn deref(&self) -> &Self::Target {
        &self.c
    }
}
