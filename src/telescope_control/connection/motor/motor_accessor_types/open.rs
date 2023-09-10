#![allow(unused)]
use super::super::*;
use crate::util::*;
use tracing::warn;

impl Motor {
    pub(in crate::telescope_control::connection::motor) async fn wait_for_stop_open(
        &mut self,
    ) -> MotorResult<()> {
        StopWaiter.wait_sync(self).await?;
        self.state = MotorState::Stationary;
        Ok(())
    }

    async fn wait_for_goto_end_open(&mut self) -> MotorResult<()> {
        GotoEndWaiter.wait_sync(self).await?;
        self.state = MotorState::Changing;
        Ok(())
    }

    async fn wait_for_rate_open(&mut self, target_rate: MotionRate) -> MotorResult<()> {
        RateWaiter(target_rate.rate()).wait_sync(self).await?;
        self.state = MotorState::Moving(target_rate);
        Ok(())
    }

    /// Moving -> Stopped
    async fn stop_open(&mut self) -> MotorResult<()> {
        match self.state {
            MotorState::Stationary => Ok(()),
            MotorState::Gotoing(_) => panic!("stop cannot be called while Gotoing"),
            MotorState::Changing => panic!("stop cannot while state Changing"),
            MotorState::Moving(_) => {
                self.mc.stop_motion().await?;
                self.wait_for_stop_open().await
            }
        }
    }

    /// From Stopped -> Moving
    async fn start_rotation_open(&mut self, mut motion_rate: MotionRate) -> MotorResult<()> {
        if !matches!(self.state, MotorState::Stationary) {
            panic!("start_rotation called on a moving motor")
        }

        if motion_rate.rate() < self.get_min_speed() {
            warn!("Rotation speed of {} too low", motion_rate.rate());
            motion_rate.set_rate(self.get_min_speed());
        } else if self.get_max_speed() < motion_rate.rate() {
            warn!("Rotation speed of {} too high", motion_rate.rate());
            motion_rate.set_rate(self.get_max_speed());
        }

        self.mc.set_tracking_mode(motion_rate.direction()).await?;
        self.mc.set_motion_rate(motion_rate.rate().abs()).await?;
        self.mc.start_motion().await?;
        self.state = MotorState::Changing;
        self.wait_for_rate_open(motion_rate).await
    }

    /// Moving -> Moving (same direction)
    async fn change_rotation_speed_open(&mut self, mut rate: Degrees) -> MotorResult<()> {
        if !matches!(self.state, MotorState::Moving(_)) {
            panic!("change_rotation_speed called when the motor state was not Moving")
        }
        if rate < self.get_min_speed() {
            warn!("Rotation speed of {} too low", rate);
            rate = self.get_min_speed()
        } else if self.get_max_speed() < rate {
            warn!("Rotation speed of {} too high", rate);
            rate = self.get_max_speed()
        }

        self.mc.set_motion_rate(rate).await?;

        let direction = self.state.get_rate().direction();
        self.state = MotorState::Changing;

        self.wait_for_rate_open(MotionRate::new(rate, direction))
            .await
    }

    /// Cannot be called while gotoing. Cancel the goto task first
    pub async fn change_rate_open(&mut self, to: MotionRate) -> MotorResult<()> {
        match self.state {
            MotorState::Gotoing(_) => panic!("change_rate cannot be called while Gotoing"),
            MotorState::Changing => panic!("change_rate cannot be called while state Changing"),
            _ => {}
        }

        let from = self.state.get_rate();

        if from == to {
            Ok(())
        } else if from.is_zero() {
            self.start_rotation_open(to).await
        } else if to.is_zero() {
            self.stop_open().await
        } else if to.direction() == from.direction() {
            self.change_rotation_speed_open(to.rate()).await
        } else {
            self.stop_open().await?;
            self.start_rotation_open(to).await
        }
    }

    /// Must be stopped
    pub async fn goto_open(&mut self, deg: Degrees) -> MotorResult<()> {
        if !matches!(self.state, MotorState::Stationary) {
            panic!("goto called on motor not stopped")
        }
        self.mc.set_goto_mode().await?;
        self.mc.set_goto_target(deg).await?;
        self.mc.start_motion().await?;
        self.state = MotorState::Gotoing(deg);
        self.wait_for_goto_end_open().await?;
        self.wait_for_stop_open().await?;
        Ok(())
    }
}
