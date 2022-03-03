use super::super::*;
use tokio::{select, task};

pub trait HasMotor {
    fn get(&self) -> MotorResult<&Motor>;
    fn get_mut(&mut self) -> MotorResult<&mut Motor>;
}

impl HasMotor for Motor {
    fn get(&self) -> MotorResult<&Motor> {
        Ok(self)
    }

    fn get_mut(&mut self) -> MotorResult<&mut Motor> {
        Ok(self)
    }
}

impl Motor {
    fn wait_for_stop<L, T>(locker: L) -> WaitableTask<MotorResult<()>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasMotor + Send + Sync,
    {
        let (task, finisher) = WaitableTask::new();

        task::spawn(async move {
            let result = StopWaiter.wait(locker.clone()).await;

            match result {
                Ok(_) => {
                    let mut ml = locker.write().await;
                    let result = ml.get_mut();
                    let motor = match result {
                        Ok(motor) => motor,
                        Err(e) => {
                            finisher.finish(Err(e));
                            return;
                        }
                    };

                    motor.state = MotorState::Stationary;
                    finisher.finish(Ok(()))
                }
                Err(e) => finisher.finish(Err(e)),
            }
        });

        task
    }

    fn wait_for_goto_end<L, T>(
        locker: L,
        abort_waiter: WaitableTask<()>,
    ) -> AbortableTask<MotorResult<()>, MotorResult<()>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasMotor + Send + Sync,
    {
        let task = AbortableTask::new_with_abort_waiter(abort_waiter);
        let finisher = task.get_finisher();
        let abort_waiter = task.get_abort_waiter();
        task::spawn(async move {
            select! {
                result = GotoEndWaiter.wait(locker.clone()) => {
                    if result.is_ok() {
                        let mut ml = locker.write().await;
                        let result = ml.get_mut();
                        let motor = match result {
                            Ok(motor) => motor,
                            Err(e) => {
                                finisher.finish(Err(e));
                                return;
                            }
                        };

                        motor.state = MotorState::Changing;
                    }
                    finisher.finish(result)
                }

                _ = abort_waiter => {
                    finisher.aborted(Ok(()))
                }
            }
        });

        task
    }

    fn wait_for_rate<L, T>(locker: L, target_rate: MotionRate) -> WaitableTask<MotorResult<()>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasMotor + Send + Sync,
    {
        let (task, finisher) = WaitableTask::new();
        task::spawn(async move {
            let result = RateWaiter(target_rate.rate()).wait(locker.clone()).await;
            match result {
                Ok(_) => {
                    let mut ml = locker.write().await;
                    let result = ml.get_mut();
                    let motor = match result {
                        Ok(motor) => motor,
                        Err(e) => {
                            finisher.finish(Err(e));
                            return;
                        }
                    };
                    motor.state = MotorState::Moving(target_rate);
                    finisher.finish(MotorResult::Ok(()))
                }
                Err(e) => finisher.finish(MotorResult::Err(e)),
            }
        });
        task
    }

    // Cannot be called while gotoing. Cancel the goto task instead
    pub async fn stop<L, T>(&mut self, locker: L) -> MotorResult<WaitableTask<MotorResult<()>>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasMotor + Send + Sync,
    {
        match self.state {
            MotorState::Stationary => Ok(WaitableTask::new_completed(Ok(()))),
            MotorState::Gotoing(_) => panic!("stop cannot be called while Gotoing"),
            MotorState::Changing => panic!("stop cannot while state Changing"),
            MotorState::Moving(_) => {
                self.mc.stop_motion().await?;
                Ok(Self::wait_for_stop(locker))
            }
        }
    }

    /// Motor must be stopped or this will panic
    async fn start_rotation<L, T>(
        &mut self,
        locker: L,
        mut motion_rate: MotionRate,
    ) -> MotorResult<WaitableTask<MotorResult<()>>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasMotor + Send + Sync,
    {
        if !matches!(self.state, MotorState::Stationary) {
            panic!("start_rotation called on a moving motor")
        }

        if motion_rate.rate() < self.get_min_speed() {
            warn!("Rotation speed of {} too low", motion_rate.rate());
            motion_rate.set_rate(self.get_min_speed())
        } else if self.get_max_speed() < motion_rate.rate() {
            warn!("Rotation speed of {} too high", motion_rate.rate());
            motion_rate.set_rate(self.get_max_speed())
        }

        self.mc.set_tracking_mode(motion_rate.direction()).await?;
        self.mc.set_motion_rate(motion_rate.rate()).await?;
        self.mc.start_motion().await?;

        self.state = MotorState::Changing;
        Ok(Self::wait_for_rate(locker, motion_rate))
    }

    async fn change_rotation_speed<L, T>(
        &mut self,
        locker: L,
        mut rate: Degrees,
    ) -> MotorResult<WaitableTask<MotorResult<()>>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasMotor + Send + Sync,
    {
        if !matches!(self.state, MotorState::Moving(_)) {
            panic!("change_rotation_speed called when the motor state was not Moving")
        }
        if rate < self.get_min_speed() {
            warn!("Rotation speed of {} too low", rate);
            rate = self.get_min_speed()
        } else if self.get_max_speed() < rate {
            warn!("Rotation speed of {} too high", rate);
            rate = self.get_max_speed()
        }

        self.mc.set_motion_rate(rate).await?;

        let direction = self.state.get_rate().direction();
        self.state = MotorState::Changing;

        Ok(Self::wait_for_rate(
            locker,
            MotionRate::new(rate, direction),
        ))
    }

    // Cannot be called while gotoing. Cancel the goto task first
    pub async fn change_rate<L, T>(
        &mut self,
        locker: L,
        to: MotionRate,
    ) -> MotorResult<WaitableTask<MotorResult<()>>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasMotor + Send + Sync,
    {
        match self.state {
            MotorState::Gotoing(_) => panic!("change_rate cannot be called while Gotoing"),
            MotorState::Changing => panic!("change_rate cannot be called while state Changing"),
            _ => {}
        }

        let from = self.state.get_rate();

        if from == to {
            Ok(WaitableTask::new_completed(Ok(())))
        } else if from.is_zero() {
            self.start_rotation(locker, to).await
        } else if to.is_zero() {
            self.stop(locker).await
        } else if to.direction() == from.direction() {
            self.change_rotation_speed(locker, to.rate()).await
        } else {
            let (task, finisher) = WaitableTask::new();
            let stop_task = self.stop(locker.clone()).await?;
            task::spawn(async move {
                let stop_result = stop_task.await;
                if stop_result.is_err() {
                    finisher.finish(stop_result);
                    return;
                }

                let result = {
                    let mut ml = locker.write().await;
                    let result = ml.get_mut();
                    let motor = match result {
                        Ok(motor) => motor,
                        Err(e) => {
                            finisher.finish(Err(e));
                            return;
                        }
                    };
                    motor.start_rotation(locker.clone(), to).await
                };

                if let Err(e) = result {
                    finisher.finish(Err(e));
                    return;
                }

                let finish_rotation = result.unwrap();

                finisher.finish(finish_rotation.await);
            });
            Ok(task)
        }
    }

    /// Must be stopped
    pub(crate) async fn goto<L, T>(
        &mut self,
        locker: L,
        deg: Degrees,
    ) -> MotorResult<AbortableTask<MotorResult<()>, MotorResult<()>>>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasMotor + Send + Sync,
    {
        if !matches!(self.state, MotorState::Stationary) {
            panic!("goto called on motor not stopped")
        }
        self.mc.set_goto_mode().await?;
        self.mc.set_goto_target(deg).await?;
        self.mc.start_motion().await?;
        self.state = MotorState::Gotoing(deg);

        let (abortable_task, finisher) = AbortableTask::new();
        let abort_waiter = abortable_task.get_abort_waiter();

        task::spawn(async move {
            let goto_result = Self::wait_for_goto_end(locker.clone(), abort_waiter).await;

            let aborted = match goto_result {
                AbortResult::Completed(result) => {
                    if result.is_err() {
                        finisher.finish(result);
                        return;
                    }
                    false
                }
                AbortResult::Aborted(result) => {
                    if result.is_err() {
                        finisher.aborted(result);
                        return;
                    }
                    // Stop the motor
                    let mut ml = locker.write().await;
                    let result = ml.get_mut();
                    let motor = match result {
                        Ok(motor) => motor,
                        Err(e) => {
                            finisher.aborted(Err(e));
                            return;
                        }
                    };
                    let result = motor.mc.stop_motion().await;
                    if result.is_err() {
                        finisher.aborted(result);
                        return;
                    }
                    true
                }
            };

            let stop_result = Self::wait_for_stop(locker).await;
            if stop_result.is_err() {
                if aborted {
                    finisher.aborted(stop_result);
                } else {
                    finisher.finish(stop_result);
                }

                return;
            }

            if aborted {
                finisher.aborted(Ok(()))
            } else {
                finisher.finish(Ok(()))
            }
        });

        Ok(abortable_task)
    }
}
