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
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult};

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

    pub async fn connect(&self, autoguide_speed: AutoGuideSpeed) -> ASCOMResult<()> {
        let mut con = self.c.write().await;
        if matches!(*con, PotentialConnection::Connected(_)) {
            return Ok(());
        }

        let mut motor = self.cb.create().await.map_err(|e| {
            ASCOMError::new(
                ASCOMErrorCode::new_for_driver(0),
                format_args!("Could not connect to motor controller: {}", e),
            )
        })?;

        motor
            .set_autoguide_speed(autoguide_speed)
            .await
            .map_err(|e| {
                ASCOMError::new(
                    ASCOMErrorCode::new_for_driver(1),
                    format_args!("Error setting autoguide speed: {}", e),
                )
            })?;

        // TODO currently stopping the motor on connection. We should restore the state maybe
        motor
            .change_rate_open(MotionRate::ZERO)
            .await
            .map_err(|e| {
                ASCOMError::new(
                    ASCOMErrorCode::new_for_driver(2),
                    format_args!("Error stopping motor: {}", e),
                )
            })?;

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

    pub async fn read_con(&self) -> ASCOMResult<CSReadLock<'_>> {
        let lock = self.c.read().await;
        match &*lock {
            PotentialConnection::Connected(_) => Ok(CSReadLock { con_lock: lock }),
            PotentialConnection::Disconnected => Err(ASCOMError::NOT_CONNECTED),
        }
    }

    pub async fn write_con(&self) -> ASCOMResult<CSWriteLock<'_>> {
        let lock = self.c.write().await;
        match &*lock {
            PotentialConnection::Connected(_) => Ok(CSWriteLock { _con_lock: lock }),
            PotentialConnection::Disconnected => Err(ASCOMError::NOT_CONNECTED),
        }
    }

    /* GET/Read */

    pub async fn get_min_speed(&self) -> ASCOMResult<Degrees> {
        let lock = self.read_con().await?;
        Ok(lock.motor.get_min_speed())
    }

    pub async fn get_max_speed(&self) -> ASCOMResult<Degrees> {
        let lock = self.read_con().await?;
        Ok(lock.motor.get_max_speed())
    }

    pub async fn get_pos(&self) -> ASCOMResult<Degrees> {
        let lock = self.read_con().await?;
        self.check_motor_result(lock.motor.get_pos().await).await
    }

    pub async fn is_guiding(&self) -> ASCOMResult<bool> {
        let lock = self.read_con().await?;
        Ok(lock.ascom_state.is_guiding())
    }

    pub async fn is_slewing(&self) -> ASCOMResult<bool> {
        let lock = self.read_con().await?;
        Ok(lock.ascom_state.is_slewing())
    }

    pub async fn is_parked(&self) -> ASCOMResult<bool> {
        let lock = self.read_con().await?;
        Ok(lock.ascom_state.is_parked())
    }

    pub async fn is_tracking(&self) -> ASCOMResult<bool> {
        let lock = self.read_con().await?;
        Ok(lock.ascom_state.is_tracking())
    }

    /* PUT/Write */

    /// Convenience function that internally locks and unlocks the connection
    pub async fn set_autoguide_speed(&self, speed: AutoGuideSpeed) -> ASCOMResult<()> {
        let mut lock = self.write_con().await?;
        self.check_motor_result(lock.motor.set_autoguide_speed(speed).await)
            .await
    }

    async fn check_motor_result<T>(&self, res: MotorResult<T>) -> ASCOMResult<T> {
        match res {
            Ok(r) => return Ok(r),
            Err(MotorError::Disconnected) => {}
            Err(motor_error) => {
                // Error means we disconnect
                tracing::error!("Disconnecting due to motor error {}", motor_error);
                self.disconnect().await;
            }
        }
        Err(ASCOMError::NOT_CONNECTED)
    }

    async fn run_short_task(&self, mut short_task: impl ShortTask) -> ASCOMResult<()> {
        // Ensure we're connected
        self.read_con().await?;
        self.check_motor_result(short_task.run(&self.c.clone()).await)
            .await?
    }

    async fn run_long_task(
        &self,
        mut long_task: impl LongTask + Send + 'static,
        mut task_lock: MutexGuard<'_, AbortableTaskType>,
    ) -> ASCOMResult<WaitableTask<AbortResult<ASCOMResult<()>, ASCOMResult<()>>>> {
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

    pub async fn start_tracking(&self, rate: MotionRate) -> ASCOMResult<()> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Err(ASCOMError::invalid_value(
                    "Can't start tracking while slewing",
                ));
            }
            AbortableTaskType::Parking(_) => {
                return Err(ASCOMError::invalid_value(
                    "Can't start tracking while parking",
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

    pub async fn stop_tracking(&self) -> ASCOMResult<()> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Err(ASCOMError::invalid_value(
                    "Can't stop tracking while slewing",
                ));
            }
            AbortableTaskType::Parking(_) => {
                return Err(ASCOMError::invalid_value(
                    "Can't stop tracking while parking",
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

    pub async fn update_tracking_rate(&self, rate: MotionRate) -> ASCOMResult<()> {
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

    pub async fn move_motor(&self, rate: MotionRate) -> ASCOMResult<()> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Err(ASCOMError::invalid_operation(
                    "Can't move motor while slewing",
                ));
            }
            AbortableTaskType::Parking(_) => {
                return Err(ASCOMError::invalid_operation(
                    "Can't move motor while parking",
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
    ) -> ASCOMResult<WaitableTask<AbortResult<ASCOMResult<()>, ASCOMResult<()>>>> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Err(ASCOMError::invalid_operation("Can't guide while slewing"));
            }
            AbortableTaskType::Parking(_) => {
                return Err(ASCOMError::invalid_operation("Can't guide while parking"));
            }
            AbortableTaskType::Guiding(_) => {
                return Err(ASCOMError::invalid_operation("Already guiding"));
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
    ) -> ASCOMResult<WaitableTask<AbortResult<ASCOMResult<()>, ASCOMResult<()>>>> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Err(ASCOMError::invalid_value("Already slewing"));
            }
            AbortableTaskType::Parking(_) => {
                return Err(ASCOMError::invalid_value("Can't slew while parking"));
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
    ) -> ASCOMResult<WaitableTask<AbortResult<ASCOMResult<()>, ASCOMResult<()>>>> {
        let mut task_lock = self.task_lock.lock().await;

        match &mut *task_lock {
            AbortableTaskType::Slewing(_) => {
                return Err(ASCOMError::invalid_value("Can't park while slewing"));
            }
            AbortableTaskType::Parking(_) => {
                return Err(ASCOMError::invalid_value("Already parking"));
            }
            AbortableTaskType::Guiding(guide_task) => {
                guide_task.abort().await.unwrap()?;
            }
            AbortableTaskType::None => {}
        }

        let park_task = ParkTask::new(park_pos);

        self.run_long_task(park_task, task_lock).await
    }

    pub async fn unpark(&self) -> ASCOMResult<()> {
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

    pub async fn abort_slew(&self) -> ASCOMResult<()> {
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
