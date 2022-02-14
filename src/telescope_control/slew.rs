use crate::rotation_direction::RotationDirection;
use crate::telescope_control::driver::Driver;
use crate::telescope_control::{StarAdventurer, State, StateArc};
use crate::tracking_direction::TrackingDirection;
use crate::util::*;
use crate::{astro_math, AxisRateRange};
use std::future::Future;
use std::time::Duration;
use tokio::sync::{oneshot, RwLockWriteGuard};
use tokio::task::JoinHandle;
use tokio::{task, time};

type SlewTaskHandle = JoinHandle<AscomResult<()>>;

#[derive(Debug)]
pub(in crate::telescope_control) enum CompletionResult<T> {
    Completed(T),
    Cancelled,
}

impl StarAdventurer {
    /// True if telescope is currently moving in response to one of the Slew methods or the MoveAxis(TelescopeAxes, Double) method
    /// False at all other times.
    pub async fn is_slewing(&self) -> AscomResult<bool> {
        let state = self.state.read().await;
        Ok(state.motor_state.is_slewing() || state.motor_state.is_manually_moving_axis())
    }

    /// Returns the post-slew settling time (sec.)
    pub async fn get_slew_settle_time(&self) -> AscomResult<u32> {
        Ok(self.state.read().await.post_slew_settle_time)
    }

    /// Sets the post-slew settling time (integer sec.).
    pub async fn set_slew_settle_time(&self, time: u32) -> AscomResult<()> {
        self.state.write().await.post_slew_settle_time = time;
        Ok(())
    }

    async fn restore_state_after_slew(
        after_state: AfterSlewState,
        state_lock: &mut RwLockWriteGuard<'_, State>,
        driver: Driver,
    ) {
        if after_state == AfterSlewState::Tracking {
            if let Err(_e) = driver
                .start_rotation(MotionRate::new(
                    state_lock.tracking_rate.into(),
                    TrackingDirection::WithTracking
                        .using(state_lock.rotation_direction_key)
                        .into(),
                ))
                .await
            {
                panic!("Fatal: Entered unknown state trying to restore state after slew");
            }
        }

        state_lock.motor_state = MotorState::from_after_state(
            after_state,
            state_lock.tracking_rate,
            TrackingDirection::WithTracking
                .using(state_lock.rotation_direction_key)
                .into(),
        );
    }

    /// Immediately Stops a slew in progress.
    pub async fn abort_slew(&self) -> AscomResult<()> {
        let mut state = self.state.write().await;

        // Spec wants this for some reason
        if state.motor_state.is_parked() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidWhileParked,
                "Can't abort slew while parked".to_string(),
            ));
        }

        // Nothing to do if not slewing
        if !(state.motor_state.is_slewing() || state.motor_state.is_manually_moving_axis()) {
            return Ok(());
        }

        // Nothing to do if already stopping
        if state.motor_state.slew_is_stopping() {
            // // wait for stop
            // driver.clone().wait_for_stop().await;
            // the slew task handles restoring the state

            // TODO no great way to know when we've stopped -- can implement with signals if necessary
            // return immediately for now
            return Ok(());
        }

        let after_state = state.motor_state.as_after_slew_state();

        if state.motor_state.is_moving() {
            // Stop slew
            state.motor_state = MotorState::Moving(MovingState::Slewing(SlewingState::Stopping));
            let stop_complete = self.driver.clone().stop_async().await?;
            std::mem::drop(state);

            // Synchronous
            stop_complete.await;
            state = self.state.write().await;
        }

        Self::restore_state_after_slew(after_state, &mut state, self.driver.clone()).await;

        Ok(())
    }

    pub(in crate::telescope_control) fn get_axis_rate_range() -> AxisRateRange {
        // experimentally, 1_103 to 16_000_000 for period
        AxisRateRange {
            minimum: Driver::get_min_speed(),
            maximum: Driver::get_max_speed(),
        }
    }

    /// The rates at which the telescope may be moved about the specified axis by the MoveAxis(TelescopeAxes, Double) method.
    pub async fn get_axis_rates(&self, axis: Axis) -> AscomResult<Vec<AxisRateRange>> {
        Ok(if axis == Axis::Primary {
            vec![Self::get_axis_rate_range()]
        } else {
            vec![AxisRateRange {
                minimum: 0.,
                maximum: 0.,
            }]
        })
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
            AscomErrorType::PropertyOrMethodNotImplemented,
            "Home is not implemented".to_string(),
        ))
    }

    /// Move the telescope in one axis at the given rate.
    /// Rate in deg/sec
    /// TODO Does this stop other slewing? Returning an error for now
    pub async fn move_axis(&self, axis: Axis, rate: Degrees) -> AscomResult<()> {
        if axis != Axis::Primary {
            return Err(AscomError::from_msg(
                AscomErrorType::PropertyOrMethodNotImplemented,
                "Can only slew on primary axis".to_string(),
            ));
        }

        // rate of 0 is just an alias for killing slews (i think) so we can redirect there
        if rate == 0. {
            log::info!("Redirecting moveaxis to abort");
            return self.abort_slew().await;
        }

        if !(Driver::get_min_speed()..=Driver::get_max_speed()).contains(&rate.abs()) {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidValue,
                "Rate is invalid".to_string(),
            ));
        }

        let mut state = self.state.write().await;
        Self::check_state_for_slew(&state.motor_state)?;

        let current_rate = state.motor_state.determine_motion_rate();
        let target_direction = if rate < 0. {
            TrackingDirection::AgainstTracking
        } else {
            TrackingDirection::WithTracking
        };
        let target_rate = MotionRate::new(
            rate.abs(),
            target_direction.using(state.rotation_direction_key).into(),
        );

        let target_state = MotorState::Moving(MovingState::Constant {
            state: ConstantMotionState::MoveAxis {
                after_state: state.motor_state.as_after_slew_state(),
            },
            guiding_state: GuidingState::Idle,
            motion_rate: target_rate,
        });

        self.driver
            .change_motor_rate(current_rate, target_rate)
            .await?;

        state.motor_state = target_state;
        Ok(())
    }

    pub(in crate::telescope_control) async fn complete_slew(
        state_arc: StateArc,
        driver: Driver,
        cancel_rx: oneshot::Receiver<()>,
        goto_complete: impl Future<Output = impl Future<Output = ()>>,
    ) -> CompletionResult<()> {
        tokio::select! {
            _ = cancel_rx => {
                log::info!("Slew task cancelled");
                CompletionResult::Cancelled
            },
            stop_complete = goto_complete => {
                // Stop
                let mut state = state_arc.write().await;
                let after_state = state.motor_state.get_after_state().unwrap();
                state.motor_state = MotorState::Moving(MovingState::Slewing(SlewingState::Stopping));
                std::mem::drop(state);
                stop_complete.await;

                // Settle
                let mut state = state_arc.write().await;
                if 0 < state.post_slew_settle_time && after_state != AfterSlewState::Parked {
                    let (canceller, cancel_rx) = oneshot::channel();
                    state.motor_state = MotorState::Moving(MovingState::Slewing(SlewingState::Settling { canceller }));
                    let settled = time::sleep(Duration::from_secs(state.post_slew_settle_time as u64));
                    std::mem::drop(state);
                    tokio::select! {
                        _ = cancel_rx => {
                            log::info!("Settle task cancelled");
                            return CompletionResult::Cancelled
                        }
                        _ = settled => {}
                    };
                    // re-lock the state we dropped
                    state = state_arc.write().await;
                }

                // Restore
                Self::restore_state_after_slew(after_state, &mut state, driver).await;
                CompletionResult::Completed(())
            }
        }
    }

    /// pos in degrees relative to turning on mount
    /// pos can be negative or positive or past 360 deg
    async fn slew_motor_to_pos(
        state_arc: StateArc,
        state_lock: &mut RwLockWriteGuard<'_, State>,
        driver: Driver,
        pos: Degrees,
        after_state: AfterSlewState,
    ) -> AscomResult<impl Future<Output = CompletionResult<()>>> {
        let goto_complete = driver.clone().goto_async(pos).await?;

        let (canceller, cancel_rx) = oneshot::channel();
        let complete_slew_future =
            Self::complete_slew(state_arc.clone(), driver.clone(), cancel_rx, goto_complete);

        state_lock.motor_state = MotorState::Moving(MovingState::Slewing(SlewingState::Gotoing {
            destination: pos,
            canceller,
            after_state,
        }));

        Ok(complete_slew_future)
    }

    /// Slews to closest version of given angle relative to where it started
    pub(in crate::telescope_control) async fn slew_motor_to_angle(
        state_arc: StateArc,
        state_lock: &mut RwLockWriteGuard<'_, State>,
        driver: Driver,
        target_angle: Degrees,
        after_state: AfterSlewState,
    ) -> AscomResult<impl Future<Output = CompletionResult<()>>> {
        let cur_pos = driver.get_pos().await?;
        let cur_angle = astro_math::modulo(cur_pos, 360.);

        let target_angle = astro_math::modulo(target_angle, 360.);

        let no_overflow_distance = (target_angle - cur_angle).abs();
        let overflow_distance = 360. - no_overflow_distance;

        let change = if overflow_distance < no_overflow_distance {
            // go the overflow way
            if cur_angle < target_angle {
                -overflow_distance
            } else {
                overflow_distance
            }
        } else if cur_angle < target_angle {
            no_overflow_distance
        } else {
            -no_overflow_distance
        };

        Self::slew_motor_to_pos(state_arc, state_lock, driver, cur_pos + change, after_state).await
    }

    /// Slews to closest version of given hour angle
    async fn slew_motor_to_hour_angle(
        state_arc: StateArc,
        state_lock: &mut RwLockWriteGuard<'_, State>,
        driver: Driver,
        hour_angle: Hours,
        after_state: AfterSlewState,
    ) -> AscomResult<impl Future<Output = CompletionResult<()>>> {
        let target_angle = astro_math::hours_to_deg(hour_angle - state_lock.hour_angle_offset);
        Self::slew_motor_to_angle(state_arc, state_lock, driver, target_angle, after_state).await
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

    async fn slew_to_hour_angle_and_dec(
        state_arc: StateArc,
        state_lock: &mut RwLockWriteGuard<'_, State>,
        driver: Driver,
        target_hour_angle: Hours,
        target_declination: Degrees,
        after_state: AfterSlewState,
    ) -> AscomResult<impl Future<Output = CompletionResult<()>>> {
        Self::alert_user_to_change_declination(state_lock.declination, target_declination);
        state_lock.declination = target_declination;

        Self::slew_motor_to_hour_angle(
            state_arc,
            state_lock,
            driver,
            target_hour_angle,
            after_state,
        )
        .await
    }

    // TODO include estimated slew time in time calculations
    async fn slew_to_coordinates_with_locks(
        state_arc: StateArc,
        state_lock: &mut RwLockWriteGuard<'_, State>,
        driver: Driver,
        ra: Hours,
        dec: Degrees,
        after_state: AfterSlewState,
    ) -> AscomResult<impl Future<Output = CompletionResult<()>>> {
        let hour_angle = astro_math::calculate_local_sidereal_time(
            Self::calculate_utc_date(state_lock.date_offset),
            state_lock.observation_location.longitude,
        ) - ra;
        Self::slew_to_hour_angle_and_dec(
            state_arc,
            state_lock,
            driver,
            hour_angle,
            dec,
            after_state,
        )
        .await
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to equatorial coordinates
    pub async fn can_slew(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given equatorial coordinates, return when slew is complete
    pub async fn slew_to_coordinates(&self, ra: Hours, dec: Degrees) -> AscomResult<()> {
        self.slew_to_coordinates_async(ra, dec)
            .await?
            .await
            .unwrap()
    }

    /// True if this telescope is capable of programmed asynchronous slewing to equatorial coordinates.
    pub async fn can_slew_async(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given equatorial coordinates, return immediately after the slew starts
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub async fn slew_to_coordinates_async(
        &self,
        ra: Hours,
        dec: Degrees,
    ) -> AscomResult<SlewTaskHandle> {
        check_ra(ra)?;
        check_dec(dec)?;

        let mut state = self.state.write().await;
        state.target.right_ascension = Some(ra);
        state.target.declination = Some(dec);
        Self::slew_to_target_with_lock(self.state.clone(), &mut state, self.driver.clone()).await
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to local horizontal coordinates
    pub async fn can_slew_alt_az(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given local horizontal coordinates, return when slew is complete
    pub async fn slew_to_alt_az(&self, alt: Degrees, az: Degrees) -> AscomResult<()> {
        log::warn!("Starting synchronous alt az slew");
        self.slew_to_alt_az_async(alt, az).await?.await.unwrap()?;
        log::warn!("Ending synchrnonous");
        Ok(())
    }

    /// True if this telescope is capable of programmed asynchronous slewing to local horizontal coordinates
    pub async fn can_slew_alt_az_async(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given local horizontal coordinates, return immediately after the slew starts.
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub async fn slew_to_alt_az_async(
        &self,
        alt: Degrees,
        az: Degrees,
    ) -> AscomResult<SlewTaskHandle> {
        check_alt(alt)?;
        check_az(az)?;

        let mut state = self.state.write().await;
        Self::check_state_for_slew(&state.motor_state)?;
        let after_state = state.motor_state.as_after_slew_state();

        let (ha, dec) =
            astro_math::calculate_ha_dec_from_alt_az(alt, az, state.observation_location.latitude);

        let slew_task = Self::slew_to_hour_angle_and_dec(
            self.state.clone(),
            &mut state,
            self.driver.clone(),
            ha,
            dec,
            after_state,
        )
        .await?;

        Ok(task::spawn(async move {
            let _cancelled = slew_task.await;
            Ok(())
        }))
    }

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return when slew is complete
    pub async fn slew_to_target(&self) -> AscomResult<()> {
        self.slew_to_target_async().await?.await.unwrap()
    }

    async fn slew_to_target_with_lock(
        state_arc: StateArc,
        state_lock: &mut RwLockWriteGuard<'_, State>,
        driver: Driver,
    ) -> AscomResult<SlewTaskHandle> {
        Self::check_state_for_slew(&state_lock.motor_state)?;

        // Ensure target is set
        let ra = state_lock.target.try_get_right_ascension()?;
        let dec = state_lock.target.try_get_declination()?;

        let after_state = state_lock.motor_state.as_after_slew_state();

        let slew_task = Self::slew_to_coordinates_with_locks(
            state_arc.clone(),
            state_lock,
            driver.clone(),
            ra,
            dec,
            after_state,
        )
        .await?;

        Ok(task::spawn(async {
            let _cancelled = slew_task.await;
            Ok(())
        }))
    }

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return immediatley after the slew starts
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub async fn slew_to_target_async(&self) -> AscomResult<SlewTaskHandle> {
        let mut state = self.state.write().await;
        Self::slew_to_target_with_lock(self.state.clone(), &mut state, self.driver.clone()).await
    }

    /// Ensures not parked or slewing and cancels guiding
    /// Returns a restorable state
    fn check_state_for_slew(motor_state: &MotorState) -> AscomResult<()> {
        if motor_state.is_parked() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidWhileParked,
                "Can't slew while parked".to_string(),
            ));
        }

        if motor_state.is_slewing() && !motor_state.is_settling() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Already Slewing".to_string(),
            ));
        }

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
