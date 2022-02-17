use crate::astro_math::{deg_to_hours, hours_to_deg, modulo};
use crate::rotation_direction::RotationDirection;
use crate::telescope_control::driver::Driver;
use crate::telescope_control::{DeclinationSlew, StarAdventurer, State, StateArc};
use crate::tracking_direction::TrackingDirection;
use crate::util::*;
use crate::{astro_math, AxisRateRange};
use std::future::Future;
use std::time::Duration;
use tokio::sync::{oneshot, RwLockWriteGuard};
use tokio::task::JoinHandle;
use tokio::{join, sync, task, time};

type SlewTaskHandle = JoinHandle<()>;

impl StarAdventurer {
    /// True if telescope is currently moving in response to one of the Slew methods or the MoveAxis(TelescopeAxes, Double) method
    /// False at all other times.
    pub async fn is_slewing(&self) -> AscomResult<bool> {
        let state = self.state.read().await;
        Ok(state.is_slewing() || state.motor_state.is_manually_moving_axis())
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

    pub async fn get_pending_dec_change(&self) -> Degrees {
        if let DeclinationSlew::Waiting(d, _) = self.state.read().await.declination_slew {
            d
        } else {
            0.
        }
    }

    pub async fn complete_dec_slew(&self) {
        let mut state = self.state.write().await;
        if let DeclinationSlew::Waiting(d, _) = state.declination_slew {
            state.declination += d;
        }

        state.declination_slew = DeclinationSlew::Idle;
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
                        .using(state_lock.observation_location.get_rotation_direction_key())
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
                .using(state_lock.observation_location.get_rotation_direction_key())
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

        state.declination_slew = DeclinationSlew::Idle;

        // Nothing to do if not slewing
        if !(state.is_slewing() || state.motor_state.is_manually_moving_axis()) {
            return Ok(());
        }

        // Nothing to do if already stopping
        if state.motor_state.slew_is_stopping() {
            // TODO no great way to know when we've stopped -- can implement with signals if necessary
            // return immediately for now
            return Ok(());
        }

        let after_state = state
            .motor_state
            .get_after_state()
            .expect("Expected restorable state when aborting slew.");

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

    /// Ensures not parked or slewing and cancels guiding
    /// Returns a restorable state
    fn check_state_for_slew(state: &RwLockWriteGuard<State>) -> AscomResult<()> {
        if state.motor_state.is_parked() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidWhileParked,
                "Can't slew while parked".to_string(),
            ));
        }

        if state.is_slewing() && !state.motor_state.is_settling() {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidOperation,
                "Already Slewing".to_string(),
            ));
        }

        Ok(())
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
        Self::check_state_for_slew(&state)?;

        let current_rate = state.motor_state.determine_motion_rate();
        let target_direction = if rate < 0. {
            TrackingDirection::AgainstTracking
        } else {
            TrackingDirection::WithTracking
        };
        let target_rate = MotionRate::new(
            rate.abs(),
            target_direction
                .using(state.observation_location.get_rotation_direction_key())
                .into(),
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
    ) -> () {
        tokio::select! {
            _ = cancel_rx => {
                log::info!("Slew task cancelled");
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
                            return
                        }
                        _ = settled => {}
                    };
                    // re-lock the state we dropped
                    state = state_arc.write().await;
                }

                // Restore
                Self::restore_state_after_slew(after_state, &mut state, driver).await;
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
    ) -> AscomResult<impl Future<Output = ()>> {
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

    async fn slew_motor_in_direction(
        state_arc: StateArc,
        state_lock: &mut RwLockWriteGuard<'_, State>,
        driver: Driver,
        distance: Hours,
        direction: impl RotationDirection,
        after_state: AfterSlewState,
    ) -> AscomResult<impl Future<Output = ()>> {
        let med: MotorEncodingDirection = direction
            .using(state_lock.observation_location.get_rotation_direction_key())
            .into();
        let pos_offset = med.get_sign_f64() * hours_to_deg(distance);
        let cur_pos = driver.get_pos().await?;
        Self::slew_motor_to_pos(
            state_arc,
            state_lock,
            driver,
            cur_pos + pos_offset,
            after_state,
        )
        .await
    }

    // Positive if with tracking, negative if against
    fn calculate_pos_change(ra_change: Hours, slew_speed: Degrees) -> (Hours, chrono::Duration) {
        const INSTANT_DISTANCE: Hours = 0.1;

        if ra_change.abs() < INSTANT_DISTANCE {
            return (ra_change, chrono::Duration::zero());
        }

        let slew_speed_hours_per_hour = deg_to_hours(slew_speed) * 3600.;

        // ALG FOR SLEW TIME ESTIMATION
        // -----------------------------------
        // pos_change = ra_change + dt
        //
        // dt = abs(pos_change) / slew_speed
        //
        // assume sign(ra_change) == sign(pos_change) (it will be unless INSTANT_DISTANCE is way too low)
        // dt = (abs(ra_change) + dt) / slew_speed
        // dt = abs(ra_change) / (slew_speed - 1)

        let dt_hours = ra_change.abs() / (slew_speed_hours_per_hour - 1.) as Hours;
        let pos_change = ra_change + dt_hours;
        let dt_seconds = dt_hours * 3600.;

        (
            pos_change,
            chrono::Duration::seconds(dt_seconds.round() as i64),
        )
    }

    fn find_shortest_path_to_ha(
        current_ha: Hours,
        target_ha: Hours,
        allow_meridian_flip: bool,
    ) -> (Hours, TrackingDirection, bool, chrono::Duration) {
        let dist_with_tracking = modulo(target_ha - current_ha, 24.);
        let dist_with_flip_with_tracking = modulo(target_ha - current_ha + 12., 24.);

        let options = [
            {
                let dist = dist_with_tracking;
                (
                    // Positive slew, no meridian flip
                    dist,
                    TrackingDirection::WithTracking,
                    false,
                    chrono::Duration::seconds(
                        (hours_to_deg(dist) / Driver::SLEW_SPEED_WITH_TRACKING).round() as i64,
                    ),
                )
            },
            {
                let dist = 24. - dist_with_tracking;

                (
                    // Negative slew, no meridian flip
                    dist,
                    TrackingDirection::AgainstTracking,
                    false,
                    chrono::Duration::seconds(
                        (hours_to_deg(dist) / Driver::SLEW_SPEED_AGAINST_TRACKING).round() as i64,
                    ),
                )
            },
            {
                let dist = dist_with_flip_with_tracking;
                (
                    // Positive slew, meridian flip
                    dist_with_flip_with_tracking,
                    TrackingDirection::WithTracking,
                    true,
                    chrono::Duration::seconds(
                        (hours_to_deg(dist) / Driver::SLEW_SPEED_WITH_TRACKING).round() as i64,
                    ),
                )
            },
            {
                let dist = 24. - dist_with_flip_with_tracking;
                (
                    // Negative slew, meridian flip
                    dist,
                    TrackingDirection::AgainstTracking,
                    true,
                    chrono::Duration::seconds(
                        (hours_to_deg(dist) / Driver::SLEW_SPEED_AGAINST_TRACKING).round() as i64,
                    ),
                )
            },
        ];

        options
            .into_iter()
            .filter(|a| allow_meridian_flip || !a.2)
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .unwrap()
    }

    fn find_shortest_path_to_ra(
        current_ra: Hours,
        target_ra: Hours,
    ) -> (Hours, TrackingDirection, bool, chrono::Duration) {
        let dist_with_tracking = modulo(current_ra - target_ra, 24.);
        let dist_with_flip_with_tracking = modulo(current_ra - target_ra + 12., 24.);

        let options = [
            {
                // Positive slew, no meridian flip
                let (change, duration) = Self::calculate_pos_change(
                    dist_with_tracking,
                    Driver::SLEW_SPEED_WITH_TRACKING,
                );
                (change, TrackingDirection::WithTracking, false, duration)
            },
            {
                // Negative slew, no meridian flip
                let (change, duration) = Self::calculate_pos_change(
                    dist_with_tracking - 24.,
                    Driver::SLEW_SPEED_AGAINST_TRACKING,
                );
                (
                    change.abs(),
                    TrackingDirection::AgainstTracking,
                    false,
                    duration,
                )
            },
            {
                // Positive slew, meridian flip
                let (change, duration) = Self::calculate_pos_change(
                    dist_with_flip_with_tracking,
                    Driver::SLEW_SPEED_WITH_TRACKING,
                );
                (change, TrackingDirection::WithTracking, true, duration)
            },
            {
                // Negative slew, meridian flip
                let (change, duration) = Self::calculate_pos_change(
                    dist_with_flip_with_tracking - 24.,
                    Driver::SLEW_SPEED_AGAINST_TRACKING,
                );
                (
                    change.abs(),
                    TrackingDirection::AgainstTracking,
                    true,
                    duration,
                )
            },
        ];

        options
            .into_iter()
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .unwrap()
    }

    fn calculate_dec_change(
        current_dec: Degrees,
        target_dec: Degrees,
        meridian_flip: bool,
    ) -> Degrees {
        if meridian_flip {
            let through_north = 180. - (current_dec + target_dec);
            let through_south = through_north - 360.;

            if through_north.abs() < through_south.abs() {
                through_north
            } else {
                through_south
            }
        } else {
            target_dec - current_dec
        }
    }

    fn alert_user_to_change_declination(dec_change: Degrees) {
        // Handle declination stuff
        // FIXME Better notification
        // FIXME Side of pier change?
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

    fn slew_dec_to_pos(
        state_lock: &mut RwLockWriteGuard<'_, State>,
        target_dec: Degrees,
        meridian_flip: bool,
        dec_slew_block: bool,
    ) -> impl Future<Output = ()> {
        let (tx, rx) = sync::oneshot::channel();

        if target_dec != state_lock.declination || meridian_flip {
            let dec_change =
                Self::calculate_dec_change(state_lock.declination, target_dec, meridian_flip);
            if dec_slew_block {
                state_lock.declination_slew = DeclinationSlew::Waiting(dec_change, tx);
            } else {
                // Instant return
                Self::alert_user_to_change_declination(dec_change);
                state_lock.declination = target_dec;
            }
        }

        if meridian_flip {
            //TODO implement meridian flip logic
        }

        async move {
            let _ = rx.await;
        }
    }

    /// Same as slew to pos, but modulos and calculates shortest path
    pub(in crate::telescope_control) async fn slew_motor_to_angle(
        state_arc: StateArc,
        state_lock: &mut RwLockWriteGuard<'_, State>,
        driver: Driver,
        target_angle: Degrees,
        after_state: AfterSlewState,
    ) -> AscomResult<impl Future<Output = ()>> {
        let cur_pos = deg_to_hours(driver.get_pos().await?);
        let target_pos = deg_to_hours(target_angle);

        let (distance, direction, _, est_slew_time) =
            Self::find_shortest_path_to_ha(cur_pos, target_pos, false);

        Self::slew_motor_in_direction(
            state_arc,
            state_lock,
            driver,
            distance,
            direction,
            after_state,
        )
        .await
    }

    async fn slew_to_ha_and_dec(
        state_arc: StateArc,
        state_lock: &mut RwLockWriteGuard<'_, State>,
        driver: Driver,
        ha: Hours,
        dec: Degrees,
        after_state: AfterSlewState,
        dec_slew_block: bool,
    ) -> AscomResult<impl Future<Output = ()>> {
        /* RA */
        let current_motor_pos = driver.get_pos().await?;
        let current_ha = Self::get_hour_angle(
            current_motor_pos,
            state_lock.hour_angle_offset,
            state_lock.observation_location.get_rotation_direction_key(),
        );

        // Find shortest path
        let (distance, direction, meridian_flip, est_slew_time) =
            Self::find_shortest_path_to_ha(current_ha, ha, true);

        let motor_slew_future = Self::slew_motor_in_direction(
            state_arc,
            state_lock,
            driver,
            distance,
            direction,
            after_state,
        )
        .await?;

        /* Dec */

        let dec_slew_future = Self::slew_dec_to_pos(state_lock, dec, meridian_flip, dec_slew_block);

        /* Join */

        return Ok(async {
            let _ = join!(motor_slew_future, dec_slew_future);
        });
    }

    async fn slew_to_ra_and_dec(
        state_arc: StateArc,
        state_lock: &mut RwLockWriteGuard<'_, State>,
        driver: Driver,
        ra: Hours,
        dec: Degrees,
        after_state: AfterSlewState,
        dec_slew_block: bool,
    ) -> AscomResult<impl Future<Output = ()>> {
        /* RA */
        let current_motor_pos = driver.get_pos().await?;
        let current_ha = Self::get_hour_angle(
            current_motor_pos,
            state_lock.hour_angle_offset,
            state_lock.observation_location.get_rotation_direction_key(),
        );

        let lst = astro_math::calculate_local_sidereal_time(
            Self::calculate_utc_date(state_lock.date_offset),
            state_lock.observation_location.longitude,
        );
        let current_ra = Self::calculate_ra(lst, current_ha);

        // Find shortest path
        let (distance, direction, meridian_flip, est_slew_time) =
            Self::find_shortest_path_to_ra(current_ra, ra);

        let motor_slew_future = Self::slew_motor_in_direction(
            state_arc,
            state_lock,
            driver,
            distance,
            direction,
            after_state,
        )
        .await?;

        /* Dec */

        let dec_slew_future = Self::slew_dec_to_pos(state_lock, dec, meridian_flip, dec_slew_block);

        /* Join */

        return Ok(async {
            let _ = join!(motor_slew_future, dec_slew_future);
        });
    }

    async fn slew_to_target_with_lock(
        state_arc: StateArc,
        state_lock: &mut RwLockWriteGuard<'_, State>,
        driver: Driver,
        dec_slew_block: bool,
    ) -> AscomResult<SlewTaskHandle> {
        Self::check_state_for_slew(&state_lock)?;

        // Ensure target is set
        let ra = state_lock.target.try_get_right_ascension()?;
        let dec = state_lock.target.try_get_declination()?;

        let after_state = state_lock.motor_state.as_after_slew_state();

        let slew_task = Self::slew_to_ra_and_dec(
            state_arc.clone(),
            state_lock,
            driver.clone(),
            ra,
            dec,
            after_state,
            dec_slew_block,
        )
        .await?;

        Ok(task::spawn(slew_task))
    }

    /* Target */

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return immediatley after the slew starts
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub async fn slew_to_target_async(&self) -> AscomResult<SlewTaskHandle> {
        let mut state = self.state.write().await;
        Self::slew_to_target_with_lock(
            self.state.clone(),
            &mut state,
            self.driver.clone(),
            self.dec_slew_block,
        )
        .await
    }

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return when slew is complete
    pub async fn slew_to_target(&self) -> AscomResult<()> {
        Ok(self.slew_to_target_async().await?.await.unwrap())
    }

    /* Coordinates */
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
        Self::slew_to_target_with_lock(
            self.state.clone(),
            &mut state,
            self.driver.clone(),
            self.dec_slew_block,
        )
        .await
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to equatorial coordinates
    pub async fn can_slew(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given equatorial coordinates, return when slew is complete
    pub async fn slew_to_coordinates(&self, ra: Hours, dec: Degrees) -> AscomResult<()> {
        Ok(self
            .slew_to_coordinates_async(ra, dec)
            .await?
            .await
            .unwrap())
    }

    /* Alt Az */

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
        Self::check_state_for_slew(&state)?;
        let after_state = state.motor_state.as_after_slew_state();

        let (ha, dec) =
            astro_math::calculate_ha_dec_from_alt_az(alt, az, state.observation_location.latitude);

        let slew_task = Self::slew_to_ha_and_dec(
            self.state.clone(),
            &mut state,
            self.driver.clone(),
            ha,
            dec,
            after_state,
            self.dec_slew_block,
        )
        .await?;

        Ok(task::spawn(slew_task))
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to local horizontal coordinates
    pub async fn can_slew_alt_az(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given local horizontal coordinates, return when slew is complete
    pub async fn slew_to_alt_az(&self, alt: Degrees, az: Degrees) -> AscomResult<()> {
        Ok(self.slew_to_alt_az_async(alt, az).await?.await.unwrap())
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
