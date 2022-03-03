use std::future::Future;
use std::mem;
use std::time::Duration;

use tokio::{join, task, time};

use crate::astro_math;
use crate::astro_math::{deg_to_hours, hours_to_deg, modulo};
use crate::rotation_direction::RotationDirection;
use crate::telescope_control::connection::consts;
use crate::tracking_direction::TrackingDirection;
use crate::util::*;

use super::super::commands::target::Target;
use super::super::star_adventurer::{DeclinationSlew, StarAdventurer};

impl StarAdventurer {
    /// True if telescope is currently moving in response to one of the Slew methods or the MoveAxis(TelescopeAxes, Double) method
    /// False at all other times.
    pub async fn is_slewing(&self) -> AscomResult<bool> {
        Ok(matches!(
            &*self.dec_slew.read().await,
            DeclinationSlew::Waiting { .. }
        ) || self.connection.is_slewing().await?)
    }

    /// Returns the post-slew settling time (sec.)
    pub async fn get_slew_settle_time(&self) -> AscomResult<u32> {
        Ok(*self.settings.post_slew_settle_time.read().await)
    }

    /// Sets the post-slew settling time (integer sec.).
    pub async fn set_slew_settle_time(&self, time: u32) -> AscomResult<()> {
        *self.settings.post_slew_settle_time.write().await = time;
        Ok(())
    }

    pub async fn get_pending_dec_change(&self) -> Degrees {
        if let DeclinationSlew::Waiting { dec_change, .. } = &*self.dec_slew.read().await {
            *dec_change
        } else {
            0.
        }
    }

    pub async fn complete_dec_slew(&self) {
        let mut dec_slew_lock = self.dec_slew.write().await;
        let dec_slew = mem::take(&mut *dec_slew_lock);
        if let DeclinationSlew::Waiting {
            dec_change,
            meridian_flip,
            finisher,
        } = dec_slew
        {
            let mut declination_lock = self.settings.declination.write().await;
            *declination_lock += dec_change;
            if meridian_flip {
                self.settings.pier_side.write().await.flip();
            }
            finisher.finish(AbortResult::Completed(()))
        }
    }

    /// Immediately Stops a slew in progress.
    pub async fn abort_slew(&self) -> AscomResult<()> {
        // Spec wants this for some reason
        if self.connection.is_parked().await? {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidWhileParked,
                "Can't abort slew while parked".to_string(),
            ));
        }

        let mut dec_slew_lock = self.dec_slew.write().await;
        let dec_slew = mem::take(&mut *dec_slew_lock);
        if let DeclinationSlew::Waiting { finisher, .. } = dec_slew {
            finisher.finish(AbortResult::Aborted(()))
        }

        self.connection.abort_slew().await?;
        Ok(())
    }

    pub(in crate::telescope_control) fn get_axis_rate_range() -> AxisRateRange {
        // experimentally, 1_103 to 16_000_000 for period
        AxisRateRange {
            // TODO are these accurate? testing needed
            minimum: consts::MIN_SPEED,
            maximum: consts::SLEW_SPEED_WITH_TRACKING.min(consts::SLEW_SPEED_AGAINST_TRACKING),
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

        if !(self.connection.get_min_speed().await?..=self.connection.get_max_speed().await?)
            .contains(&rate.abs())
        {
            return Err(AscomError::from_msg(
                AscomErrorType::InvalidValue,
                "Rate is invalid".to_string(),
            ));
        }

        let target_direction = if rate < 0. {
            TrackingDirection::AgainstTracking
        } else {
            TrackingDirection::WithTracking
        };

        let target_rate = MotionRate::new(
            rate.abs(),
            target_direction
                .using(
                    self.settings
                        .observation_location
                        .read()
                        .await
                        .get_rotation_direction_key(),
                )
                .into(),
        );

        self.connection.move_motor(target_rate).await?;
        Ok(())
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
                        (hours_to_deg(dist) / consts::SLEW_SPEED_WITH_TRACKING).round() as i64,
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
                        (hours_to_deg(dist) / consts::SLEW_SPEED_AGAINST_TRACKING).round() as i64,
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
                        (hours_to_deg(dist) / consts::SLEW_SPEED_WITH_TRACKING).round() as i64,
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
                        (hours_to_deg(dist) / consts::SLEW_SPEED_AGAINST_TRACKING).round() as i64,
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
                    consts::SLEW_SPEED_WITH_TRACKING,
                );
                (change, TrackingDirection::WithTracking, false, duration)
            },
            {
                // Negative slew, no meridian flip
                let (change, duration) = Self::calculate_pos_change(
                    dist_with_tracking - 24.,
                    consts::SLEW_SPEED_AGAINST_TRACKING,
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
                    consts::SLEW_SPEED_WITH_TRACKING,
                );
                (change, TrackingDirection::WithTracking, true, duration)
            },
            {
                // Negative slew, meridian flip
                let (change, duration) = Self::calculate_pos_change(
                    dist_with_flip_with_tracking - 24.,
                    consts::SLEW_SPEED_AGAINST_TRACKING,
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
        if dec_change != 0. {
            let dec_change_turns = dec_change / 2.957;
            // TODO Remove the turns after blocking app is implemented
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

    async fn slew_dec(
        &self,
        target_dec: Degrees,
        meridian_flip: bool,
    ) -> WaitableTask<AbortResult<(), ()>> {
        if !*self.settings.instant_dec_slew.read().await {
            // Lock the slew bookkeeper
            let mut lock = self.dec_slew.write().await;
            // Make a new task
            let current_dec = *self.settings.declination.read().await;
            if target_dec != current_dec || meridian_flip {
                let (slew_task, finisher) = WaitableTask::new();
                let dec_change = Self::calculate_dec_change(current_dec, target_dec, meridian_flip);
                *lock = DeclinationSlew::Waiting {
                    meridian_flip,
                    dec_change,
                    finisher,
                };
                return slew_task;
            }
        } else {
            // Instant return
            let mut dec_lock = self.settings.declination.write().await;
            let current_dec = *dec_lock;
            if target_dec != current_dec || meridian_flip {
                let dec_change = Self::calculate_dec_change(current_dec, target_dec, meridian_flip);
                Self::alert_user_to_change_declination(dec_change);
            }
            *dec_lock = target_dec;
            if meridian_flip {
                self.settings.pier_side.write().await.flip();
            }
        }
        WaitableTask::new_completed(AbortResult::Completed(()))
    }

    async fn slew_to_motor_pos_and_dec(
        &self,
        pos: Degrees,
        dec: Degrees,
        meridian_flip: bool,
    ) -> AscomResult<impl Future<Output = AscomResult<()>>> {
        /* RA */

        let motor_slew_task = self.connection.slew_to(pos).await?;
        let (ra_slew_task, finisher) = WaitableTask::new();
        let settle_time = *self.settings.post_slew_settle_time.read().await;
        task::spawn(async move {
            let result = motor_slew_task.await;
            if matches!(&result, AbortResult::Completed(Ok(_))) {
                time::sleep(Duration::from_secs(settle_time as u64)).await;
            }
            finisher.finish(result)
        });

        /* Dec */

        let dec_slew_task = self.slew_dec(dec, meridian_flip).await;

        /* Join, discarding abort result because this isn't used by ASCOM */

        Ok(async {
            let (motor_result, _dec_result) = join!(ra_slew_task, dec_slew_task);
            motor_result.unwrap()
        })
    }

    async fn slew_to_ha_and_dec(
        &self,
        ha: Hours,
        dec: Degrees,
    ) -> AscomResult<impl Future<Output = AscomResult<()>>> {
        /* RA */
        let current_pos = self.connection.get_pos().await?;
        let (observation_location, hour_angle_offset) = join!(
            async { *self.settings.observation_location.read().await },
            async { *self.settings.hour_angle_offset.read().await },
        );

        let key = observation_location.get_rotation_direction_key();
        let current_ha = Self::calc_ha(current_pos, hour_angle_offset, key);

        // Find shortest path
        let (distance, direction, meridian_flip, est_slew_time) =
            Self::find_shortest_path_to_ha(current_ha, ha, true);
        log::warn!(
            "Starting slew estimated to take {}s",
            est_slew_time.num_seconds()
        );

        let med: MotorEncodingDirection = direction.using(key).into();
        let pos_offset = med.get_sign_f64() * hours_to_deg(distance);

        self.slew_to_motor_pos_and_dec(current_pos + pos_offset, dec, meridian_flip)
            .await
    }

    async fn slew_to_ra_and_dec(
        &self,
        ra: Hours,
        dec: Degrees,
    ) -> AscomResult<impl Future<Output = AscomResult<()>>> {
        /* RA */
        let current_pos = self.connection.get_pos().await?;
        let (observation_location, hour_angle_offset, date_offset) = join!(
            async { *self.settings.observation_location.read().await },
            async { *self.settings.hour_angle_offset.read().await },
            async { *self.settings.date_offset.read().await },
        );

        let key = observation_location.get_rotation_direction_key();
        let current_ha = Self::calc_ha(current_pos, hour_angle_offset, key);
        /* RA */
        let current_ra = Self::calc_ra(current_ha, observation_location.longitude, date_offset);

        // Find shortest path
        let (distance, direction, meridian_flip, est_slew_time) =
            Self::find_shortest_path_to_ra(current_ra, ra);
        log::warn!(
            "Starting slew estimated to take {}s",
            est_slew_time.num_seconds()
        );

        let med: MotorEncodingDirection = direction.using(key).into();
        let pos_offset = med.get_sign_f64() * hours_to_deg(distance);

        self.slew_to_motor_pos_and_dec(current_pos + pos_offset, dec, meridian_flip)
            .await
    }

    /// Predicts the pointing state that a German equatorial mount will be in if it slews to the given coordinates
    pub async fn predict_destination_side_of_pier(
        &self,
        ra: Hours,
        _dec: Degrees,
    ) -> AscomResult<PierSide> {
        let current_ra = self.get_ra().await?;

        // Find shortest path
        let (_, _, meridian_flip, _) = Self::find_shortest_path_to_ra(current_ra, ra);

        Ok(if meridian_flip {
            self.settings.pier_side.read().await.opposite()
        } else {
            *self.settings.pier_side.read().await
        })
    }

    /* Target */

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return immediately after the slew starts
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub async fn slew_to_target_async(&self) -> AscomResult<impl Future<Output = AscomResult<()>>> {
        // Ensure target is set
        let target = *self.settings.target.read().await;
        let ra = target.try_get_right_ascension()?;
        let dec = target.try_get_declination()?;

        self.slew_to_ra_and_dec(ra, dec).await
    }

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return when slew is complete
    pub async fn slew_to_target(&self) -> AscomResult<()> {
        self.slew_to_target_async().await?.await
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
    ) -> AscomResult<impl Future<Output = AscomResult<()>>> {
        check_ra(ra)?;
        check_dec(dec)?;

        *self.settings.target.write().await = Target {
            right_ascension: Some(ra),
            declination: Some(dec),
        };

        self.slew_to_ra_and_dec(ra, dec).await
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to equatorial coordinates
    pub async fn can_slew(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given equatorial coordinates, return when slew is complete
    pub async fn slew_to_coordinates(&self, ra: Hours, dec: Degrees) -> AscomResult<()> {
        self.slew_to_coordinates_async(ra, dec).await?.await
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
    ) -> AscomResult<impl Future<Output = AscomResult<()>>> {
        check_alt(alt)?;
        check_az(az)?;

        let (ha, dec) = astro_math::calculate_ha_dec_from_alt_az(
            alt,
            az,
            self.settings.observation_location.read().await.latitude,
        );

        self.slew_to_ha_and_dec(ha, dec).await
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to local horizontal coordinates
    pub async fn can_slew_alt_az(&self) -> AscomResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given local horizontal coordinates, return when slew is complete
    pub async fn slew_to_alt_az(&self, alt: Degrees, az: Degrees) -> AscomResult<()> {
        self.slew_to_alt_az_async(alt, az).await?.await
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
