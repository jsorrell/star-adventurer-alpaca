use crate::telescope_control::connection::motor::locked::HasMotor;
use crate::telescope_control::connection::motor::Motor;

use super::*;

pub enum Con {
    Connected(ConnectedState),
    Disconnected,
}

impl Con {
    pub fn is_connected(&self) -> bool {
        match self {
            Con::Connected(_) => true,
            Con::Disconnected => false,
        }
    }

    pub fn get_con(&self) -> AscomResult<&ConnectedState> {
        match self {
            Self::Connected(c) => Ok(c),
            Self::Disconnected => Err(AscomError::from_msg(
                AscomErrorType::NotConnected,
                "Telescope Not Connected".to_string(),
            )),
        }
    }

    pub fn get_mut_con(&mut self) -> AscomResult<&mut ConnectedState> {
        match self {
            Self::Connected(c) => Ok(c),
            Self::Disconnected => Err(AscomError::from_msg(
                AscomErrorType::NotConnected,
                "Telescope Not Connected".to_string(),
            )),
        }
    }
}

#[async_trait]
impl RWLockable<Con> for Arc<RwLock<Con>> {
    async fn read(&self) -> RwLockReadGuard<'_, Con> {
        RwLock::read(self).await
    }

    async fn write(&self) -> RwLockWriteGuard<'_, Con> {
        RwLock::write(self).await
    }
}

impl HasMotor for Con {
    fn get(&self) -> MotorResult<&Motor> {
        match self.get_con() {
            Ok(c) => Ok(&c.motor),
            Err(_) => Err(MotorError::Disconnected),
        }
    }

    fn get_mut(&mut self) -> MotorResult<&mut Motor> {
        match self.get_mut_con() {
            Ok(c) => Ok(&mut c.motor),
            Err(_) => Err(MotorError::Disconnected),
        }
    }
}

pub struct ConnectedState {
    pub ascom_state: AscomState,
    pub motor: Motor,
}
