use super::*;
use crate::telescope_control::connection::motor::locked::HasMotor;
use async_trait::async_trait;
use std::time::Duration;
use synscan::DriveMode;
use tokio::time;
use tokio::time::Interval;

#[async_trait]
pub trait Waiter {
    fn get_check_interval(&self) -> Interval;
    async fn check(&self, c: &MC) -> MotorResult<bool>;

    async fn wait_sync(&self, motor: &Motor) -> MotorResult<()> {
        let mut check_interval = self.get_check_interval();

        loop {
            check_interval.tick().await;
            let result = self.check(&motor.mc).await;
            match result {
                Ok(true) => {
                    return Ok(());
                }
                Ok(false) => {
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    async fn wait<L, T>(&self, locker: L) -> MotorResult<()>
    where
        L: 'static + RWLockable<T> + Clone + Send + Sync,
        T: HasMotor + Sync,
    {
        let mut check_interval = self.get_check_interval();

        loop {
            check_interval.tick().await;
            let ml = locker.read().await;
            let motor = ml.get()?;
            let result = self.check(&motor.mc).await;
            match result {
                Ok(true) => {
                    return Ok(());
                }
                Ok(false) => {
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
}

pub struct StopWaiter;
#[async_trait]
impl Waiter for StopWaiter {
    fn get_check_interval(&self) -> Interval {
        time::interval(Duration::from_millis(250))
    }

    async fn check(&self, mc: &MC) -> MotorResult<bool> {
        Ok(!mc.inquire_status().await?.running)
    }
}

pub struct GotoEndWaiter;
#[async_trait]
impl Waiter for GotoEndWaiter {
    fn get_check_interval(&self) -> Interval {
        time::interval(Duration::from_millis(1000))
    }

    async fn check(&self, mc: &MC) -> MotorResult<bool> {
        Ok(mc.inquire_status().await?.mode != DriveMode::Goto)
    }
}

pub struct RateWaiter(pub Degrees);
#[async_trait]
impl Waiter for RateWaiter {
    fn get_check_interval(&self) -> Interval {
        time::interval(Duration::from_millis(100))
    }

    async fn check(&self, mc: &MC) -> MotorResult<bool> {
        Ok((mc.inquire_rate().await? - self.0).abs() < consts::ALLOWABLE_RATE_DIFFERENCE)
    }
}
