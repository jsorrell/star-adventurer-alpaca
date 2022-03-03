#![allow(unused)]
use synscan::SingleChannel;

pub const NUM_TRIES: u64 = 3;
pub const RETRY_MILLIS: u64 = 10;

pub const BAUD_RATE: u32 = 115_200;
pub const DEFAULT_TIMEOUT_MILLIS: u64 = 50;

pub const SIDEREAL_PERIOD: u32 = 110_359;
pub const LUNAR_PERIOD: u32 = 114_581;
pub const SOLAR_PERIOD: u32 = 110_662;
pub const KING_PERIOD: u32 = 110_390;

pub const MIN_SPEED: f64 = 0.000029;
pub const SLOW_GOTO_SPEED: f64 = 0.133727;
pub const SLEW_SPEED_WITH_TRACKING: f64 = 0.2817; // deg/sec empirically determined
pub const SLEW_SPEED_AGAINST_TRACKING: f64 = 0.3072; // deg/sec empirically determined

pub(in crate::telescope_control::connection::motor) const RA_CHANNEL: SingleChannel =
    SingleChannel::Channel1;

/// Used when waiting for rate change
pub(in crate::telescope_control::connection::motor) const ALLOWABLE_RATE_DIFFERENCE: f64 = 0.0001;
