use crate::astro_math::{Degrees, Hours};
use crate::telescope_control::{StarAdventurer, State, RA_CHANNEL};
use crate::util::enums::*;
use crate::util::result::*;
use crate::{astro_math, AxisRate};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use synscan::motors::{Direction, DriveMode};
use synscan::MotorController;
use tokio::sync::{watch, RwLock, RwLockWriteGuard};
use tokio::{task, time};

impl StarAdventurer {
    /// True if telescope is currently moving in response to one of the Slew methods or the MoveAxis(TelescopeAxes, Double) method
    /// False at all other times.
    pub async fn is_slewing(&self) -> AscomResult<bool> {
        Ok(match self.state.read().await.motion_state {
            MotionState::Slewing(_) => true,
            _ => false,
        })
    }

    /// Returns the post-slew settling time (sec.)
    pub async fn get_slew_settle_time(&self) -> AscomResult<f64> {
        // TODO use this
        Ok(self.state.read().await.post_slew_settle_time)
    }

    /// Sets the post-slew settling time (integer sec.).
    pub async fn set_slew_settle_time(&self, time: f64) -> AscomResult<()> {
        self.state.write().await.post_slew_settle_time = time;
        Ok(())
    }

    pub(in crate::telescope_control) async fn restore_state<'a, F>(
        state_to_restore: TrackingState,
        state: &mut RwLockWriteGuard<'a, State>,
        driver: &Arc<Mutex<MotorController<'static>>>,
        determinant: F,
    ) -> AscomResult<bool>
    where
        F: FnOnce(&mut MutexGuard<MotorController>) -> AscomResult<bool> + Send + 'static,
    {
        let start_motion = match state_to_restore {
            TrackingState::Tracking(_) => true,
            TrackingState::Stationary(_) => false,
        };
        let tracking_rate = state.tracking_rate.as_deg();
        let tracking_direction = Self::get_tracking_direction(state.observation_location.latitude);

        let driver_clone = driver.clone();

        let did_restore = task::spawn_blocking(move || {
            let mut driver = driver_clone.lock().unwrap();
            if determinant(&mut driver)? {
                driver.set_motion_mode(
                    RA_CHANNEL,
                    DriveMode::Tracking,
                    false,
                    tracking_direction,
                )?;
                driver.set_motion_rate_degrees(RA_CHANNEL, tracking_rate, false)?;
                if start_motion {
                    driver.start_motion(RA_CHANNEL)?;
                } else {
                    driver.stop_motion(RA_CHANNEL, false)?;
                }
                return AscomResult::Ok(true);
            }
            Ok(false)
        })
        .await
        .unwrap()?;

        if did_restore {
            state.motion_state = MotionState::Tracking(state_to_restore);
        }

        Ok(did_restore)
    }

    async fn abort_slew_with_locks(
        state: &mut RwLockWriteGuard<'_, State>,
        driver: &Arc<Mutex<MotorController<'static>>>,
    ) -> AscomResult<()> {
        match &state.motion_state {
            MotionState::Tracking(_) => Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Not slewing".to_string(),
            )),
            MotionState::Slewing(slewing_state) => {
                match slewing_state {
                    SlewingState::GotoSlewing(_, _, task_canceller) => {
                        task_canceller.send(true).unwrap()
                    }
                    _ => (),
                };

                let state_to_restore = slewing_state.get_state_to_restore();
                Self::restore_state(state_to_restore, state, driver, |_| Ok(true)).await?;
                Ok(())
            }
        }
    }

    /// Immediately Stops a slew in progress.
    pub async fn abort_slew(&self) -> AscomResult<()> {
        let mut state = self.state.write().await;
        Self::abort_slew_with_locks(&mut state, &self.driver).await
    }

    /// The rates at which the telescope may be moved about the specified axis by the MoveAxis(TelescopeAxes, Double) method.
    pub async fn get_axis_rates(&self, axis: Axis) -> AscomResult<Vec<AxisRate>> {
        if axis != Axis::Primary {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Can only slew around primary axis".to_string(),
            ));
        }
        // experimentally, 1_103 to 16_000_000 for period
        Ok(vec![AxisRate {
            minimum: 0.000029,
            maximum: 0.418032,
        }])
    }

    /// True if this telescope can move the requested axis.
    pub async fn can_move_axis(&self, axis: Axis) -> AscomResult<bool> {
        Ok(axis == Axis::Primary)
    }

    /// Predicts the pointing state that a German equatorial mount will be in if it slews to the given coordinates
    pub async fn predict_destination_side_of_pier(
        &self,
        _ra: Hours,
        _dec: Degrees,
    ) -> AscomResult<PierSide> {
        // TODO pier side stuff
        Ok(self.state.read().await.pier_side)
    }

    /// True if this telescope is capable of programmed finding its home position (FindHome() method).
    pub async fn can_find_home(&self) -> AscomResult<bool> {
        Ok(false)
    }

    /// Locates the telescope's "home" position (synchronous)
    pub async fn find_home(&self) -> AscomResult<()> {
        Err(AscomError::from_msg(
            AscomErrorType::ActionNotImplemented,
            format!("Home is not implemented"),
        ))
    }

    /// Move the telescope in one axis at the given rate.
    /// Rate in deg/sec
    /// TODO Does this stop other slewing? Returning an error for now
    pub async fn move_axis(&self, axis: Axis, rate: Degrees) -> AscomResult<()> {
        if axis != Axis::Primary {
            return Err(AscomError::from_msg(
                AscomErrorType::ActionNotImplemented,
                format!("Can only slew on primary axis"),
            ));
        }

        if rate != 0.
            && (rate < self.get_axis_rates(axis).await.unwrap()[0].minimum
                || self.get_axis_rates(axis).await.unwrap()[0].maximum < rate)
        {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidValue,
                format!("Rate is invalid"),
            ));
        }

        let mut state = self.state.write().await;
        match &state.motion_state {
            MotionState::Slewing(slewing_state) => match (slewing_state, rate == 0.) {
                (SlewingState::ManualSlewing(_), true) => {
                    Self::abort_slew_with_locks(&mut state, &self.driver).await
                }
                _ => Err(AscomError::from_msg(
                    AscomErrorType::InvalidOperation,
                    "Already slewing".to_string(),
                )),
            },
            MotionState::Tracking(ts) => {
                let prev_state = match ts {
                    TrackingState::Tracking(Some(task_canceller)) => {
                        task_canceller.send(true).unwrap();
                        TrackingState::Tracking(None)
                    }
                    TrackingState::Tracking(None) => TrackingState::Tracking(None),
                    TrackingState::Stationary(true) => {
                        return Err(AscomError::from_msg(
                            AscomErrorType::InvalidWhileParked,
                            "Can't slew while parked".to_string(),
                        ))
                    }
                    TrackingState::Stationary(false) => TrackingState::Stationary(false),
                };
                state.motion_state = MotionState::Slewing(SlewingState::ManualSlewing(prev_state));
                let mut direction =
                    Self::get_tracking_direction(state.observation_location.latitude);
                direction = if rate < 0. {
                    direction.opposite()
                } else {
                    direction
                };

                let mut driver = self.driver.lock().unwrap();
                driver.set_motion_mode(RA_CHANNEL, DriveMode::Tracking, false, direction)?;
                driver.set_motion_rate_degrees(RA_CHANNEL, rate.abs(), false)?;
                driver.start_motion(RA_CHANNEL)?;
                Ok(())
            }
        }
    }

    /// Ensures we are not slewing or parked and stops pulse guiding
    /// Returns the state that should be restored when done
    pub(crate) fn check_current_state_for_slewing(
        motion_state: &MotionState,
    ) -> AscomResult<TrackingState> {
        match &motion_state {
            MotionState::Slewing(_) => Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Already slewing".to_string(),
            )),
            MotionState::Tracking(TrackingState::Stationary(true)) => Err(AscomError::from_msg(
                AscomErrorType::InvalidWhileParked,
                "Can't slew while parked".to_string(),
            )),
            MotionState::Tracking(TrackingState::Tracking(Some(task_canceller))) => {
                task_canceller.send(true).unwrap();
                Ok(TrackingState::Tracking(None))
            }
            MotionState::Tracking(TrackingState::Tracking(None)) => {
                Ok(TrackingState::Tracking(None))
            }
            MotionState::Tracking(TrackingState::Stationary(false)) => {
                Ok(TrackingState::Stationary(false))
            }
        }
    }

    pub(in crate::telescope_control) async fn goto_task(
        state_arc: Arc<RwLock<State>>,
        driver_arc: Arc<Mutex<MotorController<'static>>>,
        cancel_rx: watch::Receiver<bool>,
    ) -> AscomResult<()> {
        let mut interval = time::interval(Duration::from_millis(1000));

        loop {
            let mut cancel_rx = cancel_rx.clone();
            // Check every second
            tokio::select! {
                // FIXME fix this error
                _ = cancel_rx.changed() => return Err(AscomError::from_msg(AscomErrorType::InvalidOperation, "Cancelled".to_string())),
                _ = interval.tick() => {
                    let mut state = state_arc.write().await;
                    let slewing_state = match &state.motion_state {
                        MotionState::Slewing(slewing_state) => slewing_state,
                        _ => unreachable!(),
                    };
                    let state_to_restore = slewing_state.get_state_to_restore();

                    let did_restore =
                        Self::restore_state(state_to_restore, &mut state, &driver_arc, |driver| {
                            let status = driver.get_status(RA_CHANNEL)?;
                            Ok(status.mode == DriveMode::Tracking)
                        })
                        .await?;

                    if did_restore {
                        return Ok(());
                    }
                }
            }
        }
    }

    /// pos in degrees relative to turning on mount
    /// pos can be negative or positive or past 360 deg
    fn slew_motor_to_pos(
        state_arc: &Arc<RwLock<State>>,
        state_lock: &mut RwLockWriteGuard<State>,
        driver_arc: &Arc<Mutex<MotorController<'static>>>,
        driver_lock: &mut MutexGuard<MotorController>,
        pos: Degrees,
        after_state: TrackingState,
    ) -> AscomResult<task::JoinHandle<AscomResult<()>>> {
        // Tell Driver to start slew
        driver_lock.stop_motion(RA_CHANNEL, false)?;

        // Set driver to max speed
        driver_lock.set_step_period(RA_CHANNEL, 1)?;
        driver_lock.set_goto_target(RA_CHANNEL, pos)?;

        // Put in GOTO mode (2nd two params are ignored here)
        driver_lock.set_motion_mode(
            RA_CHANNEL,
            synscan::motors::DriveMode::Goto,
            false,
            Direction::Clockwise,
        )?;

        // GO!
        driver_lock.start_motion(RA_CHANNEL)?;

        let state_arc_clone = Arc::clone(&state_arc);
        let driver_arc_clone = Arc::clone(&driver_arc);

        // Init goto task
        let (canceller, cancel_rx) = watch::channel::<bool>(false);
        let goto_task = task::spawn(Self::goto_task(
            state_arc_clone,
            driver_arc_clone,
            cancel_rx,
        ));

        state_lock.motion_state =
            MotionState::Slewing(SlewingState::GotoSlewing(pos, after_state, canceller));
        Ok(goto_task)
    }

    /// Slews to closest version of given angle relative to where it started
    pub(in crate::telescope_control) fn slew_motor_to_angle(
        state_arc: &Arc<RwLock<State>>,
        state_lock: &mut RwLockWriteGuard<State>,
        driver_arc: &Arc<Mutex<MotorController<'static>>>,
        driver_lock: &mut MutexGuard<MotorController>,
        target_angle: Degrees,
        after_state: TrackingState,
    ) -> AscomResult<task::JoinHandle<AscomResult<()>>> {
        let cur_pos = driver_lock.get_pos(RA_CHANNEL)?;
        let cur_angle = astro_math::modulo(cur_pos, 360.);

        let target_angle = astro_math::modulo(target_angle, 360.);
        println!("Cur Angle: {}, Target Angle: {}", cur_angle, target_angle);

        let no_overflow_distance = (target_angle - cur_angle).abs();
        let overflow_distance = 360. - no_overflow_distance;

        let change = if overflow_distance < no_overflow_distance {
            // go the overflow way
            if cur_angle < target_angle {
                -overflow_distance
            } else {
                overflow_distance
            }
        } else {
            if cur_angle < target_angle {
                no_overflow_distance
            } else {
                -no_overflow_distance
            }
        };

        println!("Angle Change: {}", change);

        Self::slew_motor_to_pos(
            state_arc,
            state_lock,
            driver_arc,
            driver_lock,
            cur_pos + change,
            after_state,
        )
    }

    /// Slews to closest version of given hour angle
    fn slew_motor_to_hour_angle(
        state_arc: &Arc<RwLock<State>>,
        state_lock: &mut RwLockWriteGuard<State>,
        driver_arc: &Arc<Mutex<MotorController<'static>>>,
        driver_lock: &mut MutexGuard<MotorController>,
        hour_angle: Hours,
        after_state: TrackingState,
    ) -> AscomResult<task::JoinHandle<AscomResult<()>>> {
        let target_angle = astro_math::hours_to_deg(hour_angle - state_lock.hour_angle_offset);
        Self::slew_motor_to_angle(
            state_arc,
            state_lock,
            driver_arc,
            driver_lock,
            target_angle,
            after_state,
        )
    }

    fn alert_user_to_change_declination(cur_declination: Degrees, target_declination: Degrees) {
        // Handle declination stuff
        // FIXME Better notification
        // FIXME Side of pier change?
        let dec_change = target_declination - cur_declination;
        if dec_change != 0. {
            let dec_change_turns = dec_change / 2.957;
            // TODO List this in clockwise or ccw (depending on pier side)
            println!(
                "TURN DECLINATION KNOB {:.2} TURNS TO THE {}",
                dec_change_turns.abs(),
                if dec_change_turns < 0. {
                    "SOUTH"
                } else {
                    "NORTH"
                }
            );
        }
    }

    fn slew_to_hour_angle_and_dec(
        state_arc: &Arc<RwLock<State>>,
        state_lock: &mut RwLockWriteGuard<State>,
        driver_arc: &Arc<Mutex<MotorController<'static>>>,
        driver_lock: &mut MutexGuard<MotorController>,
        target_hour_angle: Hours,
        target_declination: Degrees,
        after_state: TrackingState,
    ) -> AscomResult<task::JoinHandle<AscomResult<()>>> {
        Self::alert_user_to_change_declination(state_lock.declination, target_declination);
        state_lock.declination = target_declination;

        Self::slew_motor_to_hour_angle(
            state_arc,
            state_lock,
            driver_arc,
            driver_lock,
            target_hour_angle,
            after_state,
        )
    }

    fn slew_to_coordinates_with_locks(
        state_arc: &Arc<RwLock<State>>,
        state_lock: &mut RwLockWriteGuard<State>,
        driver_arc: &Arc<Mutex<MotorController<'static>>>,
        driver_lock: &mut MutexGuard<MotorController>,
        ra: Hours,
        dec: Degrees,
        after_state: TrackingState,
    ) -> AscomResult<task::JoinHandle<AscomResult<()>>> {
        let hour_angle = astro_math::calculate_local_sidereal_time(
            Self::calculate_utc_date(state_lock.date_offset),
            state_lock.observation_location.longitude,
        ) - ra;
        Self::slew_to_hour_angle_and_dec(
            state_arc,
            state_lock,
            driver_arc,
            driver_lock,
            hour_angle,
            dec,
            after_state,
        )
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to equatorial coordinates
    pub async fn can_slew(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given equatorial coordinates, return when slew is complete
    pub async fn slew_to_coordinates(&self, ra: Hours, dec: Degrees) -> AscomResult<()> {
        let waiter = {
            let mut state = self.state.write().await;
            let mut driver = self.driver.lock().unwrap();
            let restore_state = Self::check_current_state_for_slewing(&state.motion_state)?;

            Self::slew_to_coordinates_with_locks(
                &self.state,
                &mut state,
                &self.driver,
                &mut driver,
                ra,
                dec,
                restore_state,
            )?
        };
        waiter.await.unwrap()
    }

    /// True if this telescope is capable of programmed asynchronous slewing to equatorial coordinates.
    pub async fn can_slew_async(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given equatorial coordinates, return immediately after the slew starts
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub async fn slew_to_coordinates_async(&self, ra: Hours, dec: Degrees) -> AscomResult<()> {
        let mut state = self.state.write().await;
        let mut driver = self.driver.lock().unwrap();
        let restore_state = Self::check_current_state_for_slewing(&state.motion_state)?;

        let _join_handle = Self::slew_to_coordinates_with_locks(
            &self.state,
            &mut state,
            &self.driver,
            &mut driver,
            ra,
            dec,
            restore_state,
        )?;
        Ok(())
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to local horizontal coordinates
    pub async fn can_slew_alt_az(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given local horizontal coordinates, return when slew is complete
    pub async fn slew_to_alt_az(&self, alt: Degrees, az: Degrees) -> AscomResult<()> {
        let waiter = {
            let mut state = self.state.write().await;
            let mut driver = self.driver.lock().unwrap();
            let restore_state = Self::check_current_state_for_slewing(&state.motion_state)?;

            let (ha, dec) = astro_math::calculate_ha_dec_from_alt_az(
                alt,
                az,
                state.observation_location.latitude,
            );

            Self::slew_to_hour_angle_and_dec(
                &self.state,
                &mut state,
                &self.driver,
                &mut driver,
                ha,
                dec,
                restore_state,
            )?
        };
        waiter.await.unwrap()
    }

    /// True if this telescope is capable of programmed asynchronous slewing to local horizontal coordinates
    pub async fn can_slew_alt_az_async(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given local horizontal coordinates, return immediately after the slew starts.
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub async fn slew_to_alt_az_async(&self, alt: Degrees, az: Degrees) -> AscomResult<()> {
        let mut state = self.state.write().await;
        let mut driver = self.driver.lock().unwrap();
        let restore_state = Self::check_current_state_for_slewing(&state.motion_state)?;

        let (ha, dec) =
            astro_math::calculate_ha_dec_from_alt_az(alt, az, state.observation_location.latitude);

        let _join_handle = Self::slew_to_hour_angle_and_dec(
            &self.state,
            &mut state,
            &self.driver,
            &mut driver,
            ha,
            dec,
            restore_state,
        )?;
        Ok(())
    }

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return when slew is complete
    pub async fn slew_to_target(&self) -> AscomResult<()> {
        let waiter = {
            let mut state = self.state.write().await;
            let mut driver = self.driver.lock().unwrap();
            let restore_state = Self::check_current_state_for_slewing(&state.motion_state)?;
            let ra = state.target.right_ascension;
            let dec = state.target.declination;
            Self::slew_to_coordinates_with_locks(
                &self.state,
                &mut state,
                &self.driver,
                &mut driver,
                ra,
                dec,
                restore_state,
            )?
        };
        waiter.await.unwrap()
    }

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return immediatley after the slew starts
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub async fn slew_to_target_async(&self) -> AscomResult<()> {
        let mut state = self.state.write().await;
        let mut driver = self.driver.lock().unwrap();
        let restore_state = Self::check_current_state_for_slewing(&state.motion_state)?;
        let ra = state.target.right_ascension;
        let dec = state.target.declination;
        Self::slew_to_coordinates_with_locks(
            &self.state,
            &mut state,
            &self.driver,
            &mut driver,
            ra,
            dec,
            restore_state,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_util;
    #[tokio::test]
    async fn test_slew() {
        let sa = test_util::create_sa(None).await;
        sa.sync_to_coordinates(0., 30.).await.unwrap();
        sa.slew_to_coordinates(-1., 14.).await.unwrap();
    }
}
