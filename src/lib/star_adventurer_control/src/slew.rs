use crate::astro_math::{Degrees, Hours};
use crate::enums::*;
use crate::errors::{AlpacaError, ErrorType, Result};
use crate::{astro_math, MotionState, SlewingState, StarAdventurer, State, RA_CHANNEL};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockWriteGuard};
use std::time::Duration;
use synscan::motors::{Direction, DriveMode};
use synscan::MotorController;
use tokio::task;
use tokio::time::sleep;

impl StarAdventurer {
    /// True if telescope is currently moving in response to one of the Slew methods or the MoveAxis(TelescopeAxes, Double) method
    /// False at all other times.
    pub fn is_slewing(&self) -> Result<bool> {
        Ok(match self.state.read().unwrap().motion_state {
            MotionState::Slewing(_) => true,
            _ => false,
        })
    }

    /// Returns the post-slew settling time (sec.)
    pub fn get_slew_settle_time(&self) -> Result<f64> {
        // TODO use this
        Ok(self.state.read().unwrap().post_slew_settle_time)
    }

    /// Sets the post-slew settling time (integer sec.).
    pub fn set_slew_settle_time(&mut self, time: f64) -> Result<()> {
        self.state.write().unwrap().post_slew_settle_time = time;
        Ok(())
    }

    /// Immediately Stops a slew in progress.
    pub fn abort_slew(&mut self) -> Result<()> {
        let mut state = self.state.write().unwrap();
        match &state.motion_state {
            MotionState::Tracking(_) => Err(AlpacaError::from_msg(
                ErrorType::InvalidOperation,
                "Not slewing".to_string(),
            )),
            MotionState::Slewing(slewing_state) => {
                match slewing_state {
                    SlewingState::GotoSlewing(_, _, goto_task) => {
                        goto_task.abort();
                    }
                    _ => (),
                };

                let mut driver = self.driver.lock().unwrap();
                driver.stop_motion(RA_CHANNEL, false)?;

                let prev_state = slewing_state.get_state_to_restore();

                Self::restore_tracking_state(&mut driver, &mut state, prev_state)?;
                Ok(())
            }
        }
    }

    /// The rates at which the telescope may be moved about the specified axis by the MoveAxis(TelescopeAxes, Double) method.
    pub fn get_axis_rates(&self, axis: Axis) -> Result<Vec<(f64, f64)>> {
        if axis != Axis::Primary {
            return Err(AlpacaError::from_msg(
                ErrorType::InvalidOperation,
                "Can only slew around primary axis".to_string(),
            ));
        }
        // experimentally, 1_103 to 16_000_000 for period
        Ok(vec![(0.000029, 0.418032)])
    }

    /// True if this telescope can move the requested axis.
    pub fn can_move_axis(&self, axis: Axis) -> Result<bool> {
        Ok(axis == Axis::Primary)
    }

    /// Predicts the pointing state that a German equatorial mount will be in if it slews to the given coordinates
    pub fn predict_destination_side_of_pier(&self, _ra: Hours, _dec: Degrees) -> Result<PierSide> {
        // TODO pier side stuff
        Ok(self.state.read().unwrap().pier_side)
    }

    /// True if this telescope is capable of programmed finding its home position (FindHome() method).
    pub fn can_find_home(&self) -> Result<bool> {
        Ok(false)
    }

    /// Locates the telescope's "home" position (synchronous)
    pub fn find_home(&self) -> Result<()> {
        Err(AlpacaError::from_msg(
            ErrorType::ActionNotImplemented,
            format!("Home is not implemented"),
        ))
    }

    /// Move the telescope in one axis at the given rate.
    /// Rate in deg/sec
    /// TODO Does this stop other slewing? Returning an error for now
    pub fn move_axis(&mut self, axis: Axis, rate: Degrees) -> Result<()> {
        if axis != Axis::Primary {
            return Err(AlpacaError::from_msg(
                ErrorType::ActionNotImplemented,
                format!("Can only slew on primary axis"),
            ));
        }

        if rate == 0.
            || rate < self.get_axis_rates(axis).unwrap()[0].0
            || self.get_axis_rates(axis).unwrap()[0].1 < rate
        {
            return Err(AlpacaError::from_msg(
                ErrorType::InvalidValue,
                format!("Rate is invalid"),
            ));
        }

        let mut state = self.state.write().unwrap();
        match &state.motion_state {
            MotionState::Slewing(slewing_state) => match (slewing_state, rate == 0.) {
                (SlewingState::ManualSlewing(_), true) => {
                    std::mem::drop(state);
                    self.abort_slew()?;
                    Ok(())
                }
                _ => Err(AlpacaError::from_msg(
                    ErrorType::InvalidOperation,
                    "Already slewing".to_string(),
                )),
            },
            MotionState::Tracking(ts) => {
                let prev_state = match ts {
                    TrackingState::Tracking(Some(guiding_task)) => {
                        guiding_task.abort();
                        TrackingState::Tracking(None)
                    }
                    TrackingState::Tracking(None) => TrackingState::Tracking(None),
                    TrackingState::Stationary(true) => {
                        return Err(AlpacaError::from_msg(
                            ErrorType::InvalidWhileParked,
                            "Can't slew while parked".to_string(),
                        ))
                    }
                    TrackingState::Stationary(false) => TrackingState::Stationary(false),
                };
                state.motion_state = MotionState::Slewing(SlewingState::ManualSlewing(prev_state));
                let mut direction = Self::get_tracking_direction(state.latitude);
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
    ) -> Result<TrackingState> {
        match &motion_state {
            MotionState::Slewing(_) => Err(AlpacaError::from_msg(
                ErrorType::InvalidOperation,
                "Already slewing".to_string(),
            )),
            MotionState::Tracking(TrackingState::Stationary(true)) => Err(AlpacaError::from_msg(
                ErrorType::InvalidWhileParked,
                "Can't slew while parked".to_string(),
            )),
            MotionState::Tracking(TrackingState::Tracking(Some(guiding_task))) => {
                guiding_task.abort();
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

    async fn goto_task(
        state_arc: Arc<RwLock<State>>,
        driver_arc: Arc<Mutex<MotorController<'static>>>,
    ) -> Result<()> {
        loop {
            {
                let mut state = state_arc.write().unwrap();
                let mut driver = driver_arc.lock().unwrap();
                let status = driver.get_status(RA_CHANNEL)?;
                println!("Checking");
                if status.mode == DriveMode::Tracking {
                    println!("done");
                    let slewing_state = match &state.motion_state {
                        MotionState::Slewing(slewing_state) => slewing_state,
                        _ => unreachable!(),
                    };
                    let old_ts = slewing_state.get_state_to_restore();
                    StarAdventurer::restore_tracking_state(&mut driver, &mut state, old_ts)?;
                    return Ok(());
                }
            }
            println!("Still going");

            sleep(Duration::from_millis(1000)).await;
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
    ) -> Result<()> {
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
        let goto_task = task::spawn(Self::goto_task(state_arc_clone, driver_arc_clone));
        // let goto_task = task::spawn(async move {
        //     loop {
        //         {
        //             let mut state = state_arc_clone.write().unwrap();
        //             let mut driver = driver_arc_clone.lock().unwrap();
        //             let status = driver.get_status(RA_CHANNEL)?;
        //             println!("Checking");
        //             if status.mode == DriveMode::Tracking {
        //                 println!("done");
        //                 let slewing_state = match &state.motion_state {
        //                     MotionState::Slewing(slewing_state) => slewing_state,
        //                     _ => unreachable!(),
        //                 };
        //                 let old_ts = slewing_state.get_state_to_restore();
        //                 StarAdventurer::restore_tracking_state(&mut driver, &mut state, old_ts)?;
        //                 return Ok(())
        //             }
        //         }
        //         println!("Still going");
        //
        //         sleep(Duration::from_millis(1000)).await;
        //     }
        // });

        state_lock.motion_state =
            MotionState::Slewing(SlewingState::GotoSlewing(pos, after_state, goto_task));

        Ok(())
    }

    /// Slews to closest version of given angle relative to where it started
    pub(crate) fn slew_motor_to_angle(
        state_arc: &Arc<RwLock<State>>,
        state_lock: &mut RwLockWriteGuard<State>,
        driver_arc: &Arc<Mutex<MotorController<'static>>>,
        driver_lock: &mut MutexGuard<MotorController>,
        target_angle: Degrees,
        after_state: TrackingState,
    ) -> Result<()> {
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
    ) -> Result<()> {
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
    ) -> Result<()> {
        Self::slew_motor_to_hour_angle(
            state_arc,
            state_lock,
            driver_arc,
            driver_lock,
            target_hour_angle,
            after_state,
        )?;
        Self::alert_user_to_change_declination(state_lock.declination, target_declination);
        state_lock.declination = target_declination;
        Ok(())
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to local horizontal coordinates
    pub fn can_slew_alt_az(&self) -> Result<bool> {
        Ok(true)
    }

    /// Move the telescope to the given local horizontal coordinates, return when slew is complete
    pub fn slew_to_alt_az(&self, az: Degrees, alt: Degrees) {
        todo!()
    }

    /// True if this telescope is capable of programmed asynchronous slewing to local horizontal coordinates
    pub fn can_slew_alt_az_async(&self) -> Result<bool> {
        Ok(true)
    }

    /// Move the telescope to the given local horizontal coordinates, return immediately after the slew starts.
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub fn slew_to_alt_az_async(&self, az: Degrees, alt: Degrees) {
        todo!()
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to equatorial coordinates
    pub fn can_slew(&self) -> Result<bool> {
        Ok(true)
    }

    /// Move the telescope to the given equatorial coordinates, return when slew is complete
    pub fn slew_to_coordinates(&self, ra: Hours, dec: Degrees) {
        todo!()
    }

    fn slew_to_coordinates_with_locks(
        state_arc: &Arc<RwLock<State>>,
        state_lock: &mut RwLockWriteGuard<State>,
        driver_arc: &Arc<Mutex<MotorController<'static>>>,
        driver_lock: &mut MutexGuard<MotorController>,
        ra: Hours,
        dec: Degrees,
        after_state: TrackingState,
    ) -> Result<()> {
        let hour_angle = astro_math::calculate_local_sidereal_time(
            Self::calculate_utc_date(state_lock.date_offset),
            state_lock.longitude,
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

    /// True if this telescope is capable of programmed asynchronous slewing to equatorial coordinates.
    pub fn can_slew_async(&self) -> Result<bool> {
        Ok(true)
    }

    /// Move the telescope to the given equatorial coordinates, return immediately after the slew starts
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub fn slew_to_coordinates_async(&mut self, ra: Hours, dec: Degrees) -> Result<()> {
        let mut state = self.state.write().unwrap();
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
        )
    }

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return when slew is complete
    pub fn slew_to_target(&self) {
        todo!()
    }

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return immediatley after the slew starts
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub fn slew_to_target_async(&mut self) -> Result<()> {
        let mut state = self.state.write().unwrap();
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
        )
    }
}
