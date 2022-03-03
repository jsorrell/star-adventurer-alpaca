use super::consts::*;
use super::*;
use synscan::serialport::SPSerialPort;
use synscan::util::{SynScanError, SynScanResult};
use synscan::{AutoGuideSpeed, Direction, MotorController, MotorStatus};

pub struct MC(pub(in crate::telescope_control::connection::motor) MotorController<SPSerialPort>);

impl MC {
    /// Run a command on the motor.
    /// On failure, the command will be retried up to a set number of tries.
    /// As such, the command should be idempotent.
    async fn do_command_with_retries<F, T>(f: F) -> MotorResult<T>
    where
        F: Fn() -> SynScanResult<T> + Send,
        T: 'static + Send,
    {
        // TODO make this async by making the synscan library async
        let result = retry::retry_with_index(
            retry::delay::Exponential::from_millis(RETRY_MILLIS).take(NUM_TRIES as usize),
            |try_no| {
                let r = f();

                if let Err(e) = &r {
                    if try_no < NUM_TRIES {
                        log::warn!("Error sending command to driver: {} -- Retrying", e);
                    }
                }
                r
            },
        );

        if let Err(e) = result {
            let e = match e {
                retry::Error::Operation { error: e, .. } => e,
                _ => unreachable!(),
            };
            match e {
                SynScanError::CommunicationError(e) => {
                    // Cable unplugged or something like that
                    return Err(e.into());
                }
                _ => {
                    // We did something wrong
                    eprintln!("Misused motor: {:?}", e);
                    panic!("Misuse of motor")
                }
            }
        }

        Ok(result.unwrap())
    }

    pub async fn set_tracking_mode(&self, direction: Direction) -> MotorResult<()> {
        Self::do_command_with_retries(|| {
            self.0
                .set_tracking_motion_mode(RA_CHANNEL, false, direction)
        })
        .await
    }

    pub async fn set_motion_rate(&self, rate: Degrees) -> MotorResult<()> {
        Self::do_command_with_retries(|| self.0.set_motion_rate_degrees(RA_CHANNEL, rate)).await
    }

    pub async fn start_motion(&self) -> MotorResult<()> {
        Self::do_command_with_retries(|| self.0.start_motion(RA_CHANNEL)).await
    }

    pub async fn stop_motion(&self) -> MotorResult<()> {
        Self::do_command_with_retries(|| self.0.stop_motion(RA_CHANNEL)).await
    }

    pub async fn inquire_pos(&self) -> MotorResult<Degrees> {
        Self::do_command_with_retries(|| self.0.inquire_pos_degrees(RA_CHANNEL)).await
    }

    pub async fn set_autoguide_speed(&self, speed: AutoGuideSpeed) -> MotorResult<()> {
        Self::do_command_with_retries(|| self.0.set_autoguide_speed(RA_CHANNEL, speed)).await
    }

    pub async fn set_goto_mode(&self) -> MotorResult<()> {
        Self::do_command_with_retries(|| self.0.set_goto_motion_mode(RA_CHANNEL, true)).await
    }

    pub async fn set_goto_target(&self, target: Degrees) -> MotorResult<()> {
        Self::do_command_with_retries(|| self.0.set_goto_target_degrees(RA_CHANNEL, target)).await
    }

    pub async fn inquire_rate(&self) -> MotorResult<Degrees> {
        Self::do_command_with_retries(|| self.0.inquire_motion_rate_degrees(RA_CHANNEL)).await
    }

    pub async fn inquire_status(&self) -> MotorResult<MotorStatus> {
        Self::do_command_with_retries(|| self.0.inquire_status(RA_CHANNEL)).await
    }

    #[allow(unused)] // unused for now
    pub async fn inquire_goto_target(&self) -> MotorResult<Degrees> {
        Self::do_command_with_retries(|| self.0.inquire_goto_target_degrees(RA_CHANNEL)).await
    }
}
