use std::time::Duration;

use synscan::AutoGuideSpeed;
use tokio::join;
use tokio::sync::RwLock;

use crate::config::TelescopeDetails;
use crate::rotation_direction::{RotationDirection, RotationDirectionKey};
use crate::telescope_control::connection::*;
use crate::tracking_direction::TrackingDirection;
use crate::util::*;
use crate::{astro_math, config, Config};

use super::commands::target::Target;

pub enum DeclinationSlew {
    Waiting {
        dec_change: Degrees,
        meridian_flip: bool,
        finisher: WaitableTaskFinisher<AbortResult<(), ()>>,
    },
    Idle,
}

impl Default for DeclinationSlew {
    fn default() -> Self {
        Self::Idle
    }
}

pub struct StarAdventurer {
    pub(in crate::telescope_control) settings: Settings,
    pub(in crate::telescope_control) connection: Connection,
    pub(in crate::telescope_control) dec_slew: RwLock<DeclinationSlew>,
}

impl StarAdventurer {
    pub async fn new(config: &Config) -> Self {
        let mut cb = ConnectionBuilder::new()
            .with_timeout(Duration::from_millis(config.com.timeout_millis as u64));

        if config.com.path.is_some() {
            cb = cb.with_path(config.com.path.clone().unwrap());
        }

        let settings = Settings::new(config);

        StarAdventurer {
            settings,
            connection: Connection::new(cb),
            dec_slew: RwLock::new(DeclinationSlew::Idle),
        }
    }

    pub async fn is_connected(&self) -> bool {
        self.connection.read().await.is_connected()
    }

    pub async fn connect(&self) -> AscomResult<()> {
        self.connection
            .connect(*self.settings.autoguide_speed.read().await)
            .await
    }

    pub async fn disconnect(&self) -> AscomResult<()> {
        self.connection.disconnect().await;
        Ok(())
    }

    // With the telescope pointing at the meridian, this is zero
    pub fn calc_mech_ha(
        motor_pos: Degrees,
        mech_ha_offset: Hours,
        key: RotationDirectionKey,
    ) -> Hours {
        let tracking_direction: MotorEncodingDirection =
            TrackingDirection::WithTracking.using(key).into();
        let unmoduloed_angle = mech_ha_offset
            + tracking_direction.get_sign_f64() * astro_math::deg_to_hours(motor_pos);
        astro_math::modulo(unmoduloed_angle, 24.)
    }

    pub(in crate::telescope_control) async fn get_mech_ha(&self) -> AscomResult<Hours> {
        let pos = self.connection.get_pos().await?;
        let (mech_ha_offset, obs_loc) = join!(
            async { *self.settings.mech_ha_offset.read().await },
            async { *self.settings.observation_location.read().await },
        );

        Ok(Self::calc_mech_ha(
            pos,
            mech_ha_offset,
            obs_loc.get_rotation_direction_key(),
        ))
    }
}

pub(in crate::telescope_control) struct Settings {
    // Not affected by motor state, Only changed by specific requests
    pub observation_location: RwLock<config::ObservingLocation>,
    pub date_offset: RwLock<chrono::Duration>,
    pub instant_dec_slew: RwLock<bool>,

    pub park_ha: RwLock<Hours>, // Mechanical HA, 0..24
    pub mount_limits: RwLock<MountLimits>,
    pub target: RwLock<Target>,

    pub post_slew_settle_time: RwLock<u32>,
    pub autoguide_speed: RwLock<AutoGuideSpeed>, // Set to motor on connection

    pub tracking_rate: RwLock<TrackingRate>, // Read from motor on connection

    // Pos
    pub mech_ha_offset: RwLock<Hours>, // Mechanical HA, 0..24
    pub declination: RwLock<Degrees>,
    pub pier_side: RwLock<PierSide>,

    pub telescope_details: TelescopeDetails,
}

impl Settings {
    pub fn new(config: &Config) -> Self {
        Settings {
            observation_location: RwLock::new(config.observation_location),
            park_ha: RwLock::new(astro_math::modulo(config.other.park_hour_angle, 24.)), // Mechanical hour angle
            mount_limits: RwLock::new(MountLimits::new(
                config.other.mount_limit_east,
                config.other.mount_limit_west,
            )),
            declination: RwLock::new(config.initialization.declination), // Set only by sync or goto
            // hour_angle_offset: RwLock::new(StarAdventurer::calc_ha_from_mech_ha(
            //     config.initialization.hour_angle,
            //     config.initialization.pier_side,
            // )),
            mech_ha_offset: RwLock::new(config.initialization.hour_angle),
            autoguide_speed: RwLock::new(config.other.auto_guide_speed), // Write only
            pier_side: RwLock::new(config.initialization.pier_side),
            date_offset: RwLock::new(chrono::Duration::zero()), // Assume using computer time
            post_slew_settle_time: RwLock::new(config.other.slew_settle_time),
            target: RwLock::new(Target::default()), // No target initially
            tracking_rate: RwLock::new(TrackingRate::Sidereal),
            instant_dec_slew: RwLock::new(config.other.instant_dec_slew),
            telescope_details: config.telescope_details,
        }
    }
}
