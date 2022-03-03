// Pub use Tasks
pub use abort_slew::AbortSlewTask;
pub use move_motor::MoveMotorTask;
pub use park::{ParkTask, UnparkTask};
pub use pulse_guide::PulseGuideTask;
pub use set_tracking::{StartTrackingTask, StopTrackingTask, UpdateTrackingRateTask};
pub use slew_to::SlewToTask;

use crate::telescope_control::connection::con::Con;
use crate::telescope_control::connection::con::ConnectedState;
pub use crate::telescope_control::connection::motor::locked::HasMotor;
pub use crate::telescope_control::connection::motor::MotorResult;
use crate::util::*;

mod abort_slew;
mod move_motor;
mod park;
mod pulse_guide;
mod set_tracking;
mod slew_to;

pub type LongRunningTask = AbortableTask<AscomResult<()>, AscomResult<()>>;

pub enum AbortableTaskType {
    Parking(LongRunningTask),
    Slewing(LongRunningTask),
    Guiding(LongRunningTask),
    None,
}

pub trait HasCS {
    fn get(&self) -> MotorResult<&ConnectedState>;
    fn get_mut(&mut self) -> MotorResult<&mut ConnectedState>;
}

impl HasCS for Con {
    fn get(&self) -> MotorResult<&ConnectedState> {
        match self.get_con() {
            Ok(c) => Ok(c),
            Err(_) => Err("Motor Disconnected".into()),
        }
    }

    fn get_mut(&mut self) -> MotorResult<&mut ConnectedState> {
        match self.get_mut_con() {
            Ok(c) => Ok(c),
            Err(_) => Err("Motor Disconnected".into()),
        }
    }
}

#[async_trait]
pub trait LongTask {
    async fn start<L, T>(&mut self, locker: &L) -> MotorResult<AscomResult<WaitableTask<()>>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync;

    async fn complete<L, T>(&mut self, locker: &L) -> MotorResult<()>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync;

    async fn abort<L, T>(&mut self, locker: &L) -> MotorResult<()>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync;

    // TODO do this better?
    fn get_abortable_task(&self, task: LongRunningTask) -> AbortableTaskType;
}

#[async_trait]
pub trait ShortTask {
    async fn run<L, T>(&mut self, locker: &L) -> MotorResult<AscomResult<()>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasCS + HasMotor + Send + Sync;
}
