use synscan::{AutoGuideSpeed, DriveMode};

pub use builder::*;
use consts::*;
pub use mc::MC;
pub use motor_accessor_types::locked;
pub use motor_accessor_types::open;
pub use motor_state::*;
pub use result::*;
pub use waiters::*;

use crate::util::*;

pub mod consts;
mod mc;
mod motor_state;
mod motor_accessor_types {
    pub mod locked;
    pub mod open;
}
mod builder;
mod result;
mod waiters;

pub struct Motor {
    pub mc: MC,
    pub state: MotorState,
}

impl Motor {
    pub async fn get_pos(&self) -> MotorResult<f64> {
        self.mc.inquire_pos().await
    }

    pub async fn set_autoguide_speed(&mut self, speed: AutoGuideSpeed) -> MotorResult<()> {
        self.mc.set_autoguide_speed(speed).await
    }

    pub fn get_state(&self) -> &MotorState {
        &self.state
    }

    #[inline]
    pub fn get_min_speed(&self) -> Degrees {
        MIN_SPEED
    }

    #[inline]
    pub fn get_max_speed(&self) -> Degrees {
        // FIXME
        SLEW_SPEED_AGAINST_TRACKING.min(SLEW_SPEED_WITH_TRACKING)
    }

    pub(in crate::telescope_control::connection::motor) async fn determine_motor_state(
        &mut self,
    ) -> MotorResult<()> {
        let s = self.mc.inquire_status().await?;

        self.state = match (s.mode, s.running) {
            (_, false) => MotorState::Stationary,
            (DriveMode::Tracking, true) => {
                let rate = self.mc.inquire_rate().await?;
                MotorState::Moving(MotionRate::new(rate, s.direction))
            }
            (DriveMode::Goto, true) => {
                // TODO should we try to handle goto's another way? we would need an accessor passed
                self.mc.stop_motion().await?;
                self.wait_for_stop_open().await?;
                MotorState::Stationary
                // let target = self.mc.inquire_goto_target().await?;
                // MotorState::Gotoing(target)
            }
        };
        Ok(())
    }
}
