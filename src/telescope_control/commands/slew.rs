use std::future::Future;
use std::mem;
use std::time::Duration;

use tokio::{join, task, time};

use crate::astro_math;
use crate::rotation_direction::{RotationDirection, RotationDirectionKey};
use crate::telescope_control::connection::consts;
use crate::telescope_control::slew_def::Slew;
use crate::tracking_direction::TrackingDirection;
use crate::util::*;

use super::super::commands::target::Target;
use super::super::star_adventurer::{DeclinationSlew, StarAdventurer};
use ascom_alpaca::api::{Axis, AxisRate, SideOfPier};
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult};

impl StarAdventurer {
    /// True if telescope is currently moving in response to one of the Slew methods or the MoveAxis(TelescopeAxes, Double) method
    /// False at all other times.
    pub async fn is_slewing(&self) -> ASCOMResult<bool> {
        Ok(matches!(
            &*self.dec_slew.read().await,
            DeclinationSlew::Waiting { .. }
        ) || self.connection.is_slewing().await?)
    }

    /// Returns the post-slew settling time (sec.)
    pub async fn get_slew_settle_time(&self) -> ASCOMResult<u32> {
        Ok(*self.settings.post_slew_settle_time.read().await)
    }

    /// Sets the post-slew settling time (integer sec.).
    pub async fn set_slew_settle_time(&self, time: u32) -> ASCOMResult<()> {
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
    pub async fn abort_slew(&self) -> ASCOMResult<()> {
        // Spec wants this for some reason
        if self.connection.is_parked().await? {
            return Err(ASCOMError::new(
                ASCOMErrorCode::INVALID_WHILE_PARKED,
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

    pub(in crate::telescope_control) fn get_axis_rate_range() -> AxisRate {
        // experimentally, 1_103 to 16_000_000 for period
        AxisRate {
            // TODO are these accurate? testing needed
            minimum: consts::MIN_SPEED,
            maximum: consts::SLEW_SPEED_WITH_TRACKING.min(consts::SLEW_SPEED_AGAINST_TRACKING),
        }
    }

    /// The rates at which the telescope may be moved about the specified axis by the MoveAxis(TelescopeAxes, Double) method.
    pub async fn get_axis_rates(&self, axis: Axis) -> ASCOMResult<Vec<AxisRate>> {
        Ok(if axis == Axis::Primary {
            vec![Self::get_axis_rate_range()]
        } else {
            vec![AxisRate {
                minimum: 0.,
                maximum: 0.,
            }]
        })
    }

    /// True if this telescope can move the requested axis.
    pub async fn can_move_axis(&self, axis: Axis) -> ASCOMResult<bool> {
        Ok(axis == Axis::Primary)
    }

    /// True if this telescope is capable of programmed finding its home position (FindHome() method).
    pub async fn can_find_home(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Locates the telescope's "home" position (synchronous)
    pub async fn find_home(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Move the telescope in one axis at the given rate.
    /// Rate in deg/sec
    /// TODO Does this stop other slewing? Returning an error for now
    pub async fn move_axis(&self, axis: Axis, rate: Degrees) -> ASCOMResult<()> {
        if axis != Axis::Primary {
            return Err(ASCOMError::invalid_value("Can only slew on primary axis"));
        }

        // rate of 0 is just an alias for killing slews (i think) so we can redirect there
        if rate == 0. {
            tracing::info!("Redirecting moveaxis to abort");
            return self.abort_slew().await;
        }

        if !(self.connection.get_min_speed().await?..=self.connection.get_max_speed().await?)
            .contains(&rate.abs())
        {
            return Err(ASCOMError::invalid_value("Rate is invalid"));
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

    // // Positive if with tracking, negative if against
    // fn calculate_pos_change(ra_change: Hours, slew_speed: Degrees) -> (Hours, chrono::Duration) {
    //     const INSTANT_DISTANCE: Hours = 0.1;
    //
    //     if ra_change.abs() < INSTANT_DISTANCE {
    //         return (ra_change, chrono::Duration::zero());
    //     }
    //
    //     let slew_speed_hours_per_hour = deg_to_hours(slew_speed) * 3600.;
    //
    //     // ALG FOR SLEW TIME ESTIMATION
    //     // -----------------------------------
    //     // pos_change = ra_change + dt
    //     //
    //     // dt = abs(pos_change) / slew_speed
    //     //
    //     // assume sign(ra_change) == sign(pos_change) (it will be unless INSTANT_DISTANCE is way too low)
    //     // dt = (abs(ra_change) + dt) / slew_speed
    //     // dt = abs(ra_change) / (slew_speed - 1)
    //
    //     let dt_hours = ra_change.abs() / (slew_speed_hours_per_hour - 1.) as Hours;
    //     let pos_change = ra_change + dt_hours;
    //     let dt_seconds = dt_hours * 3600.;
    //
    //     (
    //         pos_change,
    //         chrono::Duration::seconds(dt_seconds.round() as i64),
    //     )
    // }

    // fn find_shortest_path_to_ha(
    //     current_ha: Hours,
    //     target_ha: Hours,
    //     allow_meridian_flip: bool,
    // ) -> (Hours, TrackingDirection, bool, chrono::Duration) {
    //     let dist_with_tracking = modulo(target_ha - current_ha, 24.);
    //     let dist_with_flip_with_tracking = modulo(target_ha - current_ha + 12., 24.);
    //
    //     let options = [
    //         {
    //             let dist = dist_with_tracking;
    //             (
    //                 // Positive slew, no meridian flip
    //                 dist,
    //                 TrackingDirection::WithTracking,
    //                 false,
    //                 chrono::Duration::seconds(
    //                     (hours_to_deg(dist) / consts::SLEW_SPEED_WITH_TRACKING).round() as i64,
    //                 ),
    //             )
    //         },
    //         {
    //             let dist = 24. - dist_with_tracking;
    //
    //             (
    //                 // Negative slew, no meridian flip
    //                 dist,
    //                 TrackingDirection::AgainstTracking,
    //                 false,
    //                 chrono::Duration::seconds(
    //                     (hours_to_deg(dist) / consts::SLEW_SPEED_AGAINST_TRACKING).round() as i64,
    //                 ),
    //             )
    //         },
    //         {
    //             let dist = dist_with_flip_with_tracking;
    //             (
    //                 // Positive slew, meridian flip
    //                 dist_with_flip_with_tracking,
    //                 TrackingDirection::WithTracking,
    //                 true,
    //                 chrono::Duration::seconds(
    //                     (hours_to_deg(dist) / consts::SLEW_SPEED_WITH_TRACKING).round() as i64,
    //                 ),
    //             )
    //         },
    //         {
    //             let dist = 24. - dist_with_flip_with_tracking;
    //             (
    //                 // Negative slew, meridian flip
    //                 dist,
    //                 TrackingDirection::AgainstTracking,
    //                 true,
    //                 chrono::Duration::seconds(
    //                     (hours_to_deg(dist) / consts::SLEW_SPEED_AGAINST_TRACKING).round() as i64,
    //                 ),
    //             )
    //         },
    //     ];
    //
    //     options
    //         .into_iter()
    //         .filter(|a| allow_meridian_flip || !a.2)
    //         .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
    //         .unwrap()
    // }

    // fn find_shortest_path_to_ra(
    //     current_ra: Hours,
    //     target_ra: Hours,
    // ) -> (Hours, TrackingDirection, bool, chrono::Duration) {
    //     let dist_with_tracking = modulo(current_ra - target_ra, 24.);
    //     let dist_with_flip_with_tracking = modulo(current_ra - target_ra + 12., 24.);
    //
    //     let options = [
    //         {
    //             // Positive slew, no meridian flip
    //             let (change, duration) = Self::calculate_pos_change(
    //                 dist_with_tracking,
    //                 consts::SLEW_SPEED_WITH_TRACKING,
    //             );
    //             (change, TrackingDirection::WithTracking, false, duration)
    //         },
    //         {
    //             // Negative slew, no meridian flip
    //             let (change, duration) = Self::calculate_pos_change(
    //                 dist_with_tracking - 24.,
    //                 consts::SLEW_SPEED_AGAINST_TRACKING,
    //             );
    //             (
    //                 change.abs(),
    //                 TrackingDirection::AgainstTracking,
    //                 false,
    //                 duration,
    //             )
    //         },
    //         {
    //             // Positive slew, meridian flip
    //             let (change, duration) = Self::calculate_pos_change(
    //                 dist_with_flip_with_tracking,
    //                 consts::SLEW_SPEED_WITH_TRACKING,
    //             );
    //             (change, TrackingDirection::WithTracking, true, duration)
    //         },
    //         {
    //             // Negative slew, meridian flip
    //             let (change, duration) = Self::calculate_pos_change(
    //                 dist_with_flip_with_tracking - 24.,
    //                 consts::SLEW_SPEED_AGAINST_TRACKING,
    //             );
    //             (
    //                 change.abs(),
    //                 TrackingDirection::AgainstTracking,
    //                 true,
    //                 duration,
    //             )
    //         },
    //     ];
    //
    //     options
    //         .into_iter()
    //         .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
    //         .unwrap()
    // }

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

    async fn slew(
        &self,
        slew: Slew,
        dec: Degrees,
        current_pos: Degrees,
        key: RotationDirectionKey,
    ) -> ASCOMResult<impl Future<Output = ASCOMResult<()>>> {
        /* RA */
        tracing::warn!(
            "Starting slew estimated to take {}s",
            slew.estimate_slew_time().as_secs()
        );

        let motor_direction = MotorEncodingDirection::from(slew.direction().using(key));
        let pos_change = astro_math::hours_to_deg(slew.distance()) * motor_direction.get_sign_f64();
        let dest_motor_pos = current_pos + pos_change;

        let motor_slew_task = self.connection.slew_to(dest_motor_pos).await?;
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

        let dec_slew_task = self.slew_dec(dec, slew.does_meridian_flip()).await;

        /* Join, discarding abort result because this isn't used by ASCOM */

        Ok(async {
            let (motor_result, _dec_result) = join!(ra_slew_task, dec_slew_task);
            motor_result.unwrap()
        })
    }

    async fn slew_to_ha(
        &self,
        ha: Hours,
        dec: Degrees,
    ) -> ASCOMResult<impl Future<Output = ASCOMResult<()>>> {
        /* RA */
        let current_pos = self.connection.get_pos().await?;
        let (observation_location, mech_ha_offset, pier_side, mount_limits) = join!(
            async { *self.settings.observation_location.read().await },
            async { *self.settings.mech_ha_offset.read().await },
            async { *self.settings.pier_side.read().await },
            async { *self.settings.mount_limits.read().await },
        );

        let key = observation_location.get_rotation_direction_key();
        let current_mech_ha = Self::calc_mech_ha(current_pos, mech_ha_offset, key);

        let slew = Slew::to_ha(current_mech_ha, ha, pier_side, mount_limits);

        self.slew(slew, dec, current_pos, key).await
    }

    async fn slew_to_ra(
        &self,
        ra: Hours,
        dec: Degrees,
    ) -> ASCOMResult<impl Future<Output = ASCOMResult<()>>> {
        /* RA */
        let current_pos = self.connection.get_pos().await?;
        let (observation_location, mech_ha_offset, date_offset, pier_side, mount_limits) = join!(
            async { *self.settings.observation_location.read().await },
            async { *self.settings.mech_ha_offset.read().await },
            async { *self.settings.date_offset.read().await },
            async { *self.settings.pier_side.read().await },
            async { *self.settings.mount_limits.read().await },
        );

        let key = observation_location.get_rotation_direction_key();
        let current_mech_ha = Self::calc_mech_ha(current_pos, mech_ha_offset, key);
        let current_ha = Self::calc_ha_from_mech_ha(current_mech_ha, pier_side);
        let current_ra = Self::calc_ra(current_ha, observation_location.longitude, date_offset);

        let slew = Slew::change_ra(current_mech_ha, ra - current_ra, mount_limits);

        self.slew(slew, dec, current_pos, key).await
    }

    /// Predicts the pointing state that a German equatorial mount will be in if it slews to the given coordinates
    pub async fn predict_destination_side_of_pier(
        &self,
        ra: Hours,
        _dec: Degrees,
    ) -> ASCOMResult<SideOfPier> {
        let current_pos = self.connection.get_pos().await?;
        let (observation_location, mech_ha_offset, date_offset, pier_side, mount_limits) = join!(
            async { *self.settings.observation_location.read().await },
            async { *self.settings.mech_ha_offset.read().await },
            async { *self.settings.date_offset.read().await },
            async { *self.settings.pier_side.read().await },
            async { *self.settings.mount_limits.read().await },
        );

        let key = observation_location.get_rotation_direction_key();
        let current_mech_ha = Self::calc_mech_ha(current_pos, mech_ha_offset, key);
        let current_ha = Self::calc_ha_from_mech_ha(current_mech_ha, pier_side);
        let current_ra = Self::calc_ra(current_ha, observation_location.longitude, date_offset);

        let slew = Slew::change_ra(current_mech_ha, ra - current_ra, mount_limits);

        Ok(if slew.does_meridian_flip() {
            pier_side.opposite()
        } else {
            pier_side
        })
    }

    /* Target */

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return immediately after the slew starts
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub async fn slew_to_target_async(&self) -> ASCOMResult<impl Future<Output = ASCOMResult<()>>> {
        // Ensure target is set
        let target = *self.settings.target.read().await;
        let ra = target.try_get_right_ascension()?;
        let dec = target.try_get_declination()?;

        self.slew_to_ra(ra, dec).await
    }

    /// Move the telescope to the TargetRightAscension and TargetDeclination equatorial coordinates, return when slew is complete
    pub async fn slew_to_target(&self) -> ASCOMResult<()> {
        self.slew_to_target_async().await?.await
    }

    /* Coordinates */
    /// True if this telescope is capable of programmed asynchronous slewing to equatorial coordinates.
    pub async fn can_slew_async(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given equatorial coordinates, return immediately after the slew starts
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub async fn slew_to_coordinates_async(
        &self,
        ra: Hours,
        dec: Degrees,
    ) -> ASCOMResult<impl Future<Output = ASCOMResult<()>>> {
        check_ra(ra)?;
        check_dec(dec)?;

        *self.settings.target.write().await = Target {
            right_ascension: Some(ra),
            declination: Some(dec),
        };

        self.slew_to_ra(ra, dec).await
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to equatorial coordinates
    pub async fn can_slew(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given equatorial coordinates, return when slew is complete
    pub async fn slew_to_coordinates(&self, ra: Hours, dec: Degrees) -> ASCOMResult<()> {
        self.slew_to_coordinates_async(ra, dec).await?.await
    }

    /* Alt Az */

    /// True if this telescope is capable of programmed asynchronous slewing to local horizontal coordinates
    pub async fn can_slew_alt_az_async(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given local horizontal coordinates, return immediately after the slew starts.
    /// The client can poll the Slewing method to determine when the mount reaches the intended coordinates.
    pub async fn slew_to_alt_az_async(
        &self,
        alt: Degrees,
        az: Degrees,
    ) -> ASCOMResult<impl Future<Output = ASCOMResult<()>>> {
        check_alt(alt)?;
        check_az(az)?;

        let (ha, dec) = astro_math::calculate_ha_dec_from_alt_az(
            alt,
            az,
            self.settings.observation_location.read().await.latitude,
        );

        self.slew_to_ha(ha, dec).await
    }

    /// True if this telescope is capable of programmed slewing (synchronous or asynchronous) to local horizontal coordinates
    pub async fn can_slew_alt_az(&self) -> ASCOMResult<bool> {
        Ok(true)
    }

    /// Move the telescope to the given local horizontal coordinates, return when slew is complete
    pub async fn slew_to_alt_az(&self, alt: Degrees, az: Degrees) -> ASCOMResult<()> {
        self.slew_to_alt_az_async(alt, az).await?.await
    }
}

#[cfg(test)]
mod tests {
    use crate::telescope_control::test_util;

    #[tokio::test]
    async fn test_slew() {
        let sa = test_util::create_sa(None).await;
        sa.sync_to_coordinates(0., 30.).await.unwrap();
        sa.slew_to_coordinates(-1., 14.).await.unwrap();
    }
}
