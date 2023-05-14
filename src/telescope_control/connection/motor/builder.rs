use super::*;
use std::time::Duration;
use synscan::MotorController;

#[derive(Clone, Default, Debug)]
pub struct MotorBuilder {
    path: Option<String>,
    timeout: Option<Duration>,
}

impl MotorBuilder {
    fn determine_serial_port() -> Result<String, String> {
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
                        consts::BAUD_RATE,
                        Duration::from_millis(consts::DEFAULT_TIMEOUT_MILLIS),
                    );

                    if let Err(_e) = mc {
                        return false;
                    }

                    let mc = mc.unwrap();

                    if let Err(_e) = mc.test() {
                        return false;
                    }

                    return true;
                }
            }

            false
        });

        if let Some(port) = port {
            Ok(port.port_name)
        } else {
            Err("StarAdventurer Port not found".to_string())
        }
    }

    pub fn new() -> Self {
        MotorBuilder::default()
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub async fn create(&self) -> Result<Motor, String> {
        let path = if self.path.is_some() {
            self.path.clone().unwrap()
        } else {
            let port = Self::determine_serial_port()?;
            tracing::warn!("Found StarAdventurer COM port at {}", port);
            port
        };
        let timeout = self
            .timeout
            .unwrap_or_else(|| Duration::from_millis(consts::DEFAULT_TIMEOUT_MILLIS));
        let mc = MotorController::new_serialport(path, consts::BAUD_RATE, timeout);
        if let Err(_e) = mc {
            return Err("Couldn't connect to StarAdventurer".to_string());
        }

        let mc = MC(mc.unwrap());

        let mut motor = Motor {
            mc,
            state: MotorState::Stationary, // Temporary
        };

        if motor.determine_motor_state().await.is_err() {
            return Err("Couldn't determine motor state".to_string());
        }

        Ok(motor)
    }
}
