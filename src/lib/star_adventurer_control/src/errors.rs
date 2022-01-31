use std::error::Error;
use std::fmt::{Display, Formatter};
use std::{fmt, result};
use synscan::result::SynScanError;

pub type Result<T> = result::Result<T, AlpacaError>;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum ErrorType {
    PropertyOrMethodNotImplemented = 0x400,
    InvalidValue = 0x401,
    ValueNotSet = 0x402,
    NotConnected = 0x407,
    InvalidWhileParked = 0x408,
    InvalidWhileSlaved = 0x409,
    InvalidOperation = 0x40B,
    ActionNotImplemented = 0x40C,
}

#[derive(Debug)]
pub struct AlpacaError {
    error_number: i32,
    error_message: String,
}

impl AlpacaError {
    pub fn from_msg(e_type: ErrorType, message: String) -> AlpacaError {
        AlpacaError {
            error_number: e_type as i32,
            error_message: message,
        }
    }
}

impl From<SynScanError> for AlpacaError {
    fn from(e: SynScanError) -> Self {
        Self {
            error_number: match e {
                SynScanError::UnknownCommand => 0x500,
                SynScanError::CommandLengthError => 0x501,
                SynScanError::MotorNotStopped => 0x502,
                SynScanError::InvalidCharacter => 0x503,
                SynScanError::NotInitialized => 0x504,
                SynScanError::DriverSleeping => 0x505,
                SynScanError::PECTrainingRunning => 0x506,
                SynScanError::NoValidPECData => 0x507,
                SynScanError::CommunicationError(_) => 0x508,
            },
            error_message: format!("{}", e),
        }
    }
}

impl Display for AlpacaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error_number, self.error_message)
    }
}

impl Error for AlpacaError {}
