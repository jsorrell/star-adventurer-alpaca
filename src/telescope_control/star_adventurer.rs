use std::time::Duration;

use synscan::AutoGuideSpeed;
use tokio::sync::RwLock;

use super::commands::target::Target;
use crate::config::TelescopeDetails;
use crate::telescope_control::connection::*;
use crate::util::*;
use crate::{config, Config};

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
        let mut cb = ConnectionBuilder::new().with_timeout(Duration::from_millis(
            config.com_settings.timeout_millis as u64,
        ));

        if config.com_settings.path.is_some() {
            cb = cb.with_path(config.com_settings.path.clone().unwrap());
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
}

pub(in crate::telescope_control) struct Settings {
    // Not affected by motor state, Only changed by specific requests
    pub observation_location: RwLock<config::ObservingLocation>,
    pub date_offset: RwLock<chrono::Duration>,
    pub instant_dec_slew: RwLock<bool>,

    pub park_pos: RwLock<Degrees>,
    pub target: RwLock<Target>,

    pub post_slew_settle_time: RwLock<u32>,
    pub autoguide_speed: RwLock<AutoGuideSpeed>, // Set to motor on connection

    pub tracking_rate: RwLock<TrackingRate>, // Read from motor on connection

    // Pos
    pub hour_angle_offset: RwLock<Hours>, // Hour angle at pos=0
    pub declination: RwLock<Degrees>,
    pub pier_side: RwLock<PierSide>,

    pub telescope_details: TelescopeDetails,
}

impl Settings {
    pub fn new(config: &Config) -> Self {
        Settings {
            observation_location: RwLock::new(config.observation_location),
            park_pos: RwLock::new(0.0), // TODO: Put an angular park pos in config (that only works after alignment)
            declination: RwLock::new(0.0), // Set only by sync or goto
            hour_angle_offset: RwLock::new(0.0), // Set only by sync or goto
            autoguide_speed: RwLock::new(AutoGuideSpeed::Half), // Write only, so default to half b/c most standard
            pier_side: RwLock::new(PierSide::East), // TODO Should this always be East to start?
            date_offset: RwLock::new(chrono::Duration::zero()), // Assume using computer time
            post_slew_settle_time: RwLock::new(config.other_settings.slew_settle_time),
            target: RwLock::new(Target::default()), // No target initially
            tracking_rate: RwLock::new(TrackingRate::Sidereal),
            instant_dec_slew: RwLock::new(config.other_settings.instant_dec_slew),
            telescope_details: config.telescope_details,
        }
    }
}
