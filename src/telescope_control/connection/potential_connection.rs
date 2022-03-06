use crate::telescope_control::connection::motor::locked::HasMotor;
use crate::telescope_control::connection::motor::Motor;

use super::*;

pub enum PotentialConnection {
    Connected(ConnectedState),
    Disconnected,
}

impl PotentialConnection {
    pub fn is_connected(&self) -> bool {
        match self {
            PotentialConnection::Connected(_) => true,
            PotentialConnection::Disconnected => false,
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
impl RWLockable<PotentialConnection> for Arc<RwLock<PotentialConnection>> {
    async fn read(&self) -> RwLockReadGuard<'_, PotentialConnection> {
        RwLock::read(self).await
    }

    async fn write(&self) -> RwLockWriteGuard<'_, PotentialConnection> {
        RwLock::write(self).await
    }
}

impl HasMotor for PotentialConnection {
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
