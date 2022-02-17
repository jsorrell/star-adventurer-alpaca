use crate::alternate::Alternate;
use crate::util::*;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use synscan::serialport::SPSerialPort;
use synscan::util::SynScanResult;
use synscan::{AutoGuideSpeed, Direction, DriveMode, MotorController, MotorStatus, SingleChannel};
use tokio::{task, time};

const RA_CHANNEL: SingleChannel = SingleChannel::Channel1;
type MCAccess<'a> = MutexGuard<'a, MotorController<SPSerialPort>>;
type MCArc = Arc<Mutex<MotorController<SPSerialPort>>>;

#[derive(Clone)]
pub struct Driver(MCArc);

#[derive(Copy, Clone)]
pub enum Status {
    Stationary,
    Moving(Direction),
    Slewing(Degrees),
}

const NUM_TRIES: u64 = 3;

const BAUD_RATE: u32 = 115_200;
const DEFAULT_TIMEOUT_MILLIS: u64 = 50;

const SIDEREAL_PERIOD: u32 = 110_359;
const LUNAR_PERIOD: u32 = 114_581;
const SOLAR_PERIOD: u32 = 110_662;
const KING_PERIOD: u32 = 110_390;

#[derive(Clone, Default, Debug)]
pub struct DriverBuilder {
    path: Option<String>,
    timeout: Option<Duration>,
}

impl DriverBuilder {
    fn determine_driver_port() -> Result<String, String> {
        let available_ports = serialport::available_ports();
        if let Err(e) = available_ports {
            return Err(e.description);
        }

        let port = available_ports.unwrap().into_iter().find(|p| {
            let t = &p.port_type;

            if let serialport::SerialPortType::UsbPort(i) = t {
                if i.vid == 0x67b && // Prolific
                    i.pid == 0x2303
                // PL2303 usb to serial
                {
                    let mc = MotorController::new_serialport(
                        &p.port_name,
                        BAUD_RATE,
                        Duration::from_millis(DEFAULT_TIMEOUT_MILLIS),
                    );

                    if let Err(_e) = mc {
                        return false;
                    }

                    let mut mc = mc.unwrap();

                    if let Err(_e) = mc.test() {
                        return false;
                    }

                    return true;
                }
            }

            false
        });

        if port.is_some() {
            Ok(port.unwrap().port_name)
        } else {
            Err("StarAdventurer Port not found".to_string())
        }
    }

    pub fn new() -> Self {
        DriverBuilder::default()
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn create(self) -> Result<Driver, String> {
        let path = if self.path.is_some() {
            self.path.unwrap()
        } else {
            let port = Self::determine_driver_port()?;
            log::warn!("Found StarAdventurer COM port at {}", port);
            port
        };
        let timeout = self
            .timeout
            .unwrap_or_else(|| Duration::from_millis(DEFAULT_TIMEOUT_MILLIS as u64));
        let mc = MotorController::new_serialport(path, BAUD_RATE, timeout);
        if let Err(_e) = mc {
            return Err("Couldn't connect to StarAdventurer".to_string());
        }
        Ok(Driver(Arc::new(Mutex::new(mc.unwrap()))))
    }
}

impl Driver {
    pub const SLEW_SPEED_WITH_TRACKING: f64 = 0.2817; // deg/sec empirically determined
    pub const SLEW_SPEED_AGAINST_TRACKING: f64 = 0.3072; // deg/sec empirically determined

    pub fn test_connection(&self) -> SynScanResult<()> {
        let mut mc = self.0.lock().unwrap();
        mc.test()
    }

    /// The command should be idempotent
    async fn do_command_with_retries<F, T>(&self, f: F) -> SynScanResult<T>
    where
        F: Fn(&mut MCAccess) -> SynScanResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let driver_clone = self.0.clone();
        task::spawn_blocking(move || {
            let mut mc = driver_clone.lock().unwrap();
            let result = retry::retry_with_index(
                retry::delay::Exponential::from_millis(10).take(NUM_TRIES as usize),
                |try_no| {
                    if 1 < try_no {}

                    let r = f(&mut mc);

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
                log::error!(
                    "Operation did not succeed within {} tries: {}",
                    NUM_TRIES,
                    e
                );
                return Err(e);
            }

            SynScanResult::Ok(result.unwrap())
        })
        .await
        .unwrap()
    }

    #[inline]
    pub fn get_min_speed() -> Degrees {
        0.000029
    }

    #[inline]
    pub fn get_max_speed() -> Degrees {
        Self::SLEW_SPEED_AGAINST_TRACKING.min(Self::SLEW_SPEED_WITH_TRACKING)
        // 0.418032 // Max Reported Speed but it doesn't go this fast
    }

    #[inline]
    pub fn get_goto_zeroing_speed() -> Degrees {
        0.133727
    }

    /// Normally keep track of rate locally -- only used for initing
    pub async fn determine_tracking_rate(&self) -> SynScanResult<Alternate<TrackingRate, Degrees>> {
        let rate = self
            .do_command_with_retries(|mc| mc.inquire_step_period(RA_CHANNEL))
            .await?;
        Ok(Alternate::A(match rate {
            SIDEREAL_PERIOD => TrackingRate::Sidereal,
            LUNAR_PERIOD => TrackingRate::Lunar,
            SOLAR_PERIOD => TrackingRate::Solar,
            KING_PERIOD => TrackingRate::King,
            _ => {
                let rate_deg = self
                    .do_command_with_retries(|mc| mc.inquire_motion_rate_degrees(RA_CHANNEL))
                    .await?;
                return Ok(Alternate::B(rate_deg));
            }
        }))
    }

    pub async fn get_status(&self) -> SynScanResult<Status> {
        let s = self
            .do_command_with_retries(|mc| mc.inquire_status(RA_CHANNEL))
            .await?;

        Ok(match (s.mode, s.running) {
            (DriveMode::Tracking, true) => Status::Moving(s.direction),
            (DriveMode::Goto, true) => {
                let t = self
                    .do_command_with_retries(|mc| mc.inquire_goto_target_degrees(RA_CHANNEL))
                    .await?;
                Status::Slewing(t)
            }
            (_, false) => Status::Stationary,
        })
    }

    pub async fn get_pos(&self) -> SynScanResult<f64> {
        self.do_command_with_retries(|mc| mc.inquire_pos_degrees(RA_CHANNEL))
            .await
    }

    /// Motor must be stopped
    pub async fn start_rotation(&self, motion_rate: MotionRate) -> SynScanResult<()> {
        let mut rate = motion_rate.rate();
        let direction = motion_rate.direction();

        if rate < Self::get_min_speed() {
            warn!("Rotation speed of {} too low", rate);
            rate = Self::get_min_speed();
        } else if Self::get_max_speed() < rate {
            warn!("Rotation speed of {} too high", rate);
            rate = Self::get_max_speed();
        }

        self.do_command_with_retries(move |mc| {
            mc.set_tracking_motion_mode(RA_CHANNEL, false, direction)
        })
        .await?;

        self.do_command_with_retries(move |mc| mc.set_motion_rate_degrees(RA_CHANNEL, rate.abs()))
            .await?;

        self.do_command_with_retries(move |mc| mc.start_motion(RA_CHANNEL))
            .await
    }

    /// Motor must be moving at a slow speed
    pub async fn change_rotation_speed(&self, speed: Degrees) -> SynScanResult<()> {
        if speed < Self::get_min_speed() {
            warn!("Rotation speed of {} too low", speed);
        } else if Self::get_max_speed() < speed {
            warn!("Rotation speed of {} too high", speed);
        }

        self.do_command_with_retries(move |mc| mc.set_motion_rate_degrees(RA_CHANNEL, speed))
            .await
    }

    async fn wait_for_status<F>(self, f: F)
    where
        F: Fn(MotorStatus) -> bool, // + Send + 'static,
    {
        let mut check_interval = time::interval(Duration::from_millis(250));
        loop {
            check_interval.tick().await;
            let st = self
                .do_command_with_retries(|mc| mc.inquire_status(RA_CHANNEL))
                .await;

            if let Err(e) = st {
                log::warn!("Error getting status from driver: {}", e);
                continue; // just keep waiting
            }

            if f(st.unwrap()) {
                return;
            }
        }
    }

    pub async fn wait_for_stop(self) {
        self.wait_for_status(move |st| !st.running).await
    }

    pub async fn stop_async(self) -> SynScanResult<impl Future<Output = ()>> {
        self.do_command_with_retries(|mc| mc.stop_motion(RA_CHANNEL))
            .await?;

        Ok(self.wait_for_stop())
    }

    pub async fn stop(self) -> SynScanResult<()> {
        Ok(self.stop_async().await?.await)
    }

    async fn wait_for_goto_end(self) {
        self.wait_for_status(move |st| st.mode != DriveMode::Goto)
            .await
    }

    pub async fn track_goto(self) -> impl Future<Output = ()> {
        self.clone().wait_for_goto_end().await;
        self.wait_for_stop()
    }

    pub async fn goto_async(
        self,
        deg: Degrees,
    ) -> SynScanResult<impl Future<Output = impl Future<Output = ()>>> {
        self.do_command_with_retries(|mc| mc.stop_motion(RA_CHANNEL))
            .await?;

        self.do_command_with_retries(|mc| mc.set_goto_motion_mode(RA_CHANNEL, true))
            .await?;

        self.do_command_with_retries(move |mc| mc.set_goto_target_degrees(RA_CHANNEL, deg))
            .await?;

        self.do_command_with_retries(|mc| mc.start_motion(RA_CHANNEL))
            .await?;

        Ok(self.track_goto())
    }

    pub async fn set_autoguide_speed(&self, speed: AutoGuideSpeed) -> SynScanResult<()> {
        self.do_command_with_retries(move |mc| mc.set_autoguide_speed(RA_CHANNEL, speed))
            .await?;
        Ok(())
    }

    /// Atomic in that it does nothing if it fails
    pub async fn change_motor_rate(&self, from: MotionRate, to: MotionRate) -> SynScanResult<()> {
        if from == to {
            Ok(())
        } else if from.is_zero() {
            self.start_rotation(to).await
        } else if to.is_zero() {
            self.clone().stop().await
        } else if to.direction() == from.direction() {
            self.change_rotation_speed(to.rate()).await
        } else {
            self.clone().stop().await?;
            if let Err(_e) = self.start_rotation(to).await {
                // Todo this is potentially recoverable, but it would be awkward to do
                panic!("Fatal: Failed mid command and entered invalid state");
            }
            Ok(())
        }
    }
}
