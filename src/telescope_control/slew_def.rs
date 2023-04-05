use crate::astro_math::{deg_to_hours, hours_to_deg, modulo};
use crate::telescope_control::connection::consts::{
    SLEW_SPEED_AGAINST_TRACKING, SLEW_SPEED_WITH_TRACKING,
};
use crate::tracking_direction::TrackingDirection;
use crate::util::*;
use crate::StarAdventurer;
use ascom_alpaca::api::SideOfPier;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct Slew {
    distance: Hours,
    direction: TrackingDirection,
    meridian_flip: bool,
}

impl Slew {
    fn find_best(start_mech_ha: Hours, mount_limits: MountLimits, options: Vec<Self>) -> Self {
        options
            .into_iter()
            .filter(|s| mount_limits.is_valid_slew(start_mech_ha, s))
            .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap())
            .expect("No valid slew -- this shouldn't happen")
    }

    pub fn to_mech_ha(
        current_mech_ha: Hours,
        target_mech_ha: Hours,
        mount_limits: MountLimits,
    ) -> Self {
        let dist_with_tracking = modulo(target_mech_ha - current_mech_ha, 24.);
        Self::find_best(
            current_mech_ha,
            mount_limits,
            vec![
                Self {
                    distance: dist_with_tracking,
                    direction: TrackingDirection::WithTracking,
                    meridian_flip: false,
                },
                Self {
                    distance: 24. - dist_with_tracking,
                    direction: TrackingDirection::AgainstTracking,
                    meridian_flip: false,
                },
            ],
        )
    }

    pub fn to_ha(
        current_mech_ha: Hours,
        target_ha: Hours,
        current_pier_side: SideOfPier,
        mount_limits: MountLimits,
    ) -> Self {
        let east_ha = StarAdventurer::calc_ha_from_mech_ha(current_mech_ha, SideOfPier::East);
        let west_ha = StarAdventurer::calc_ha_from_mech_ha(current_mech_ha, SideOfPier::West);

        Self::find_best(
            current_mech_ha,
            mount_limits,
            vec![
                Self {
                    distance: modulo(target_ha - east_ha, 24.),
                    direction: TrackingDirection::WithTracking,
                    meridian_flip: SideOfPier::East != current_pier_side,
                },
                Self {
                    distance: 24. - modulo(target_ha - east_ha, 24.),
                    direction: TrackingDirection::AgainstTracking,
                    meridian_flip: SideOfPier::East != current_pier_side,
                },
                Self {
                    distance: modulo(target_ha - west_ha, 24.),
                    direction: TrackingDirection::WithTracking,
                    meridian_flip: SideOfPier::West != current_pier_side,
                },
                Self {
                    distance: 24. - modulo(target_ha - west_ha, 24.),
                    direction: TrackingDirection::AgainstTracking,
                    meridian_flip: SideOfPier::West != current_pier_side,
                },
            ],
        )
    }

    pub fn change_ra(current_mech_ha: Hours, ra_change: Hours, mount_limits: MountLimits) -> Self {
        let ra_change = modulo(ra_change, 24.);
        let ra_change_with_flip = modulo(ra_change + 12., 24.);
        let pos_ha_change = Self::ha_change_from_ra_change(ra_change - 24.);
        let neg_ha_change = Self::ha_change_from_ra_change(ra_change);
        let pos_ha_change_with_flip = Self::ha_change_from_ra_change(ra_change_with_flip - 24.);
        let neg_ha_change_with_flip = Self::ha_change_from_ra_change(ra_change_with_flip);

        Self::find_best(
            current_mech_ha,
            mount_limits,
            vec![
                Self {
                    distance: pos_ha_change,
                    direction: TrackingDirection::WithTracking,
                    meridian_flip: false,
                },
                Self {
                    distance: -neg_ha_change,
                    direction: TrackingDirection::AgainstTracking,
                    meridian_flip: false,
                },
                Self {
                    distance: pos_ha_change_with_flip,
                    direction: TrackingDirection::WithTracking,
                    meridian_flip: true,
                },
                Self {
                    distance: -neg_ha_change_with_flip,
                    direction: TrackingDirection::AgainstTracking,
                    meridian_flip: true,
                },
            ],
        )
    }

    pub fn distance(&self) -> Hours {
        self.distance
    }

    pub fn direction(&self) -> TrackingDirection {
        self.direction
    }

    pub fn does_meridian_flip(&self) -> bool {
        self.meridian_flip
    }

    pub fn estimate_slew_time(&self) -> Duration {
        Duration::from_secs_f64(match self.direction {
            TrackingDirection::WithTracking => {
                hours_to_deg(self.distance) / SLEW_SPEED_WITH_TRACKING
            }
            TrackingDirection::AgainstTracking => {
                hours_to_deg(self.distance) / SLEW_SPEED_AGAINST_TRACKING
            }
        })
    }

    /// Negative if with tracking, Positive if against
    fn ha_change_from_ra_change(ra_change: Hours) -> Hours {
        const INSTANT_DISTANCE: Hours = 0.1;
        if ra_change.abs() < INSTANT_DISTANCE {
            return -ra_change;
        }

        let slew_speed = if ra_change < 0. {
            SLEW_SPEED_WITH_TRACKING
        } else {
            -SLEW_SPEED_AGAINST_TRACKING
        };

        let slew_speed_hours_per_hour = deg_to_hours(slew_speed) * 3600.;

        // ALG FOR SLEW TIME ESTIMATION
        // -----------------------------------
        // ha_change = -ra_change + slew_time
        //
        // slew_time = ha_change / slew_speed
        //
        // slew_time = (-ra_change + slew_time) / slew_speed
        //
        // slew_time - slew_time/slew_speed = -ra_change / slew_speed
        // slew_time(1/slew_speed - 1) = ra_change/slew_speed
        // slew_time = ra_change/(slew_speed (1/slew_speed - 1))
        // slew_time = ra_change/(1 - slew_speed)

        let slew_time_hours = ra_change / (1. - slew_speed_hours_per_hour) as Hours;
        -ra_change + slew_time_hours
    }
}
