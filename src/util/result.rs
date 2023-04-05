use crate::{Degrees, Hours};
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::{fmt, result};
use synscan::util::SynScanError;

pub fn convert_synscan_error(e: SynScanError) -> ASCOMError {
    ASCOMError::new(
        match e {
            SynScanError::UnknownCommand => ASCOMErrorCode::new_for_driver::<0>(),
            SynScanError::CommandLengthError => ASCOMErrorCode::new_for_driver::<1>(),
            SynScanError::MotorNotStopped => ASCOMErrorCode::new_for_driver::<2>(),
            SynScanError::InvalidCharacter => ASCOMErrorCode::new_for_driver::<3>(),
            SynScanError::NotInitialized => ASCOMErrorCode::new_for_driver::<4>(),
            SynScanError::DriverSleeping => ASCOMErrorCode::new_for_driver::<5>(),
            SynScanError::PECTrainingRunning => ASCOMErrorCode::new_for_driver::<6>(),
            SynScanError::NoValidPECData => ASCOMErrorCode::new_for_driver::<7>(),
            SynScanError::CommunicationError(_) => ASCOMErrorCode::new_for_driver::<8>(),
        },
        format!("{}", e),
    )
}

pub fn check_dec(dec: Degrees) -> ASCOMResult<()> {
    if (-90. ..=90.).contains(&dec) {
        Ok(())
    } else {
        return Err(ASCOMError::new(
            ASCOMErrorCode::INVALID_VALUE,
            format!("Declination of {} is not valid", dec),
        ));
    }
}

pub fn check_ra(ra: Hours) -> ASCOMResult<()> {
    if (0. ..24.).contains(&ra) {
        Ok(())
    } else {
        return Err(ASCOMError::new(
            ASCOMErrorCode::INVALID_VALUE,
            format!("Right Ascension of {} is not valid", ra),
        ));
    }
}

pub fn check_alt(alt: Degrees) -> ASCOMResult<()> {
    if (-90. ..=90.).contains(&alt) {
        Ok(())
    } else {
        return Err(ASCOMError::new(
            ASCOMErrorCode::INVALID_VALUE,
            format!("Altitude of {} is not valid", alt),
        ));
    }
}

pub fn check_az(az: Degrees) -> ASCOMResult<()> {
    if (0. ..360.).contains(&az) {
        Ok(())
    } else {
        return Err(ASCOMError::new(
            ASCOMErrorCode::INVALID_VALUE,
            format!("Azimuth {} is not valid", az),
        ));
    }
}
