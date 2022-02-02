use crate::Degrees;
use crate::TrackingRate::{King, Lunar, Sidereal, Solar};
use tokio::sync::watch;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum AlignmentMode {
    /// Altitude-Azimuth alignment
    AltAz,
    /// Polar (equatorial) mount other than German equatorial
    Polar,
    /// German equatorial mount
    GermanPolar,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum EquatorialCoordinateType {
    Other,
    Topocentric,
    J2000,
    J2050,
    B1950,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum PierSide {
    Unknown = -1,
    East,
    West,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum TrackingRate {
    Sidereal,
    Lunar,
    Solar,
    King,
}

impl TrackingRate {
    // const SIDEREAL_PERIOD: u32 = 110_359;
    // const LUNAR_PERIOD: u32 = 114_581;
    // const SOLAR_PERIOD: u32 = 110_662;
    // const KING_PERIOD: u32 = 110_390;

    pub fn determine_step_period(&self, frequency: u32, steps_per_rotation: u32) -> u32 {
        let exact_period =
            (frequency as f64 / steps_per_rotation as f64) * (360. / self.as_deg() as f64);
        exact_period.round() as u32
    }

    pub fn as_deg(&self) -> Degrees {
        match self {
            TrackingRate::Sidereal => 0.00417809,
            TrackingRate::Lunar => 0.004024138,
            TrackingRate::Solar => 0.00416665,
            TrackingRate::King => 0.00417692,
        }
    }

    pub fn determine_from_period(
        period: u32,
        frequency: u32,
        steps_per_rotation: u32,
    ) -> Option<Self> {
        if Sidereal.determine_step_period(frequency, steps_per_rotation) == period {
            Some(Sidereal)
        } else if Lunar.determine_step_period(frequency, steps_per_rotation) == period {
            Some(Lunar)
        } else if Solar.determine_step_period(frequency, steps_per_rotation) == period {
            Some(Solar)
        } else if King.determine_step_period(frequency, steps_per_rotation) == period {
            Some(King)
        } else {
            None
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Axis {
    /// RA or Az
    Primary,
    /// Dec or Alt
    Secondary,
    /// imager rotator/de-rotator
    Tertiary,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum GuideDirection {
    North,
    South,
    East,
    West,
}

pub(crate) type TaskCanceller = watch::Sender<bool>;

#[derive(Debug)]
pub enum MotionState {
    Tracking(TrackingState),
    Slewing(SlewingState),
}

#[derive(Debug)]
pub enum SlewingState {
    ManualSlewing(TrackingState),
    GotoSlewing(Degrees, TrackingState, TaskCanceller), // ra channel target
}

impl SlewingState {
    pub fn get_state_to_restore(&self) -> TrackingState {
        let ts = match self {
            SlewingState::ManualSlewing(ts) => ts,
            SlewingState::GotoSlewing(_, ts, _) => ts,
        };

        match ts {
            TrackingState::Stationary(p) => TrackingState::Stationary(*p),
            TrackingState::Tracking(None) => TrackingState::Tracking(None),
            TrackingState::Tracking(_) => panic!("Kept autoguiding while slewing"),
        }
    }
}

#[derive(Debug)]
pub enum TrackingState {
    Stationary(bool),                // bool for parked
    Tracking(Option<TaskCanceller>), // guiding task
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Target {
    pub right_ascension: f64,
    pub declination: f64,
}

impl Default for Target {
    fn default() -> Self {
        Target {
            right_ascension: 0.,
            declination: 0.,
        }
    }
}
