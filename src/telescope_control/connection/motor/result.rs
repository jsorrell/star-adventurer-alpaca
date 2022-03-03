use core::fmt::{Debug, Display, Formatter};
use std::io;
use std::io::Error;

#[derive(Clone, Debug)]
pub enum MotorError {
    IOError(String),
    Disconnected,
}

pub type MotorResult<T> = Result<T, MotorError>;

impl From<io::Error> for MotorError {
    fn from(e: Error) -> Self {
        Self::IOError(e.to_string())
    }
}

impl From<String> for MotorError {
    fn from(s: String) -> Self {
        Self::IOError(s)
    }
}

impl From<&str> for MotorError {
    fn from(s: &str) -> Self {
        Self::IOError(s.to_string())
    }
}

impl Display for MotorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IOError(s) => Display::fmt(&s, f),
            Self::Disconnected => Display::fmt("Disconnected", f),
        }
    }
}
