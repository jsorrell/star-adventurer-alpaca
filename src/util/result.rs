use crate::{Degrees, Hours};
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::{fmt, result};
use synscan::util::SynScanError;

pub fn convert_synscan_error(e: SynScanError) -> ASCOMError {
    ASCOMError::new(
        ASCOMErrorCode::new_for_driver(
            100 + match e {
                SynScanError::UnknownCommand => 1,
                SynScanError::CommandLengthError => 2,
                SynScanError::MotorNotStopped => 3,
                SynScanError::InvalidCharacter => 4,
                SynScanError::NotInitialized => 5,
                SynScanError::DriverSleeping => 6,
                SynScanError::PECTrainingRunning => 7,
                SynScanError::NoValidPECData => 8,
                SynScanError::CommunicationError(_) => 9,
            },
        ),
        e,
    )
}

pub fn check_dec(dec: Degrees) -> ASCOMResult<()> {
    if (-90. ..=90.).contains(&dec) {
        Ok(())
    } else {
        return Err(ASCOMError::invalid_value(format_args!(
            "Declination of {} is not valid",
            dec
        )));
    }
}

pub fn check_ra(ra: Hours) -> ASCOMResult<()> {
    if (0. ..24.).contains(&ra) {
        Ok(())
    } else {
        return Err(ASCOMError::invalid_value(format_args!(
            "Right Ascension of {} is not valid",
            ra
        )));
    }
}

pub fn check_alt(alt: Degrees) -> ASCOMResult<()> {
    if (-90. ..=90.).contains(&alt) {
        Ok(())
    } else {
        return Err(ASCOMError::invalid_value(format_args!(
            "Altitude of {} is not valid",
            alt
        )));
    }
}

pub fn check_az(az: Degrees) -> ASCOMResult<()> {
    if (0. ..360.).contains(&az) {
        Ok(())
    } else {
        return Err(ASCOMError::invalid_value(format_args!(
            "Azimuth {} is not valid",
            az
        )));
    }
}
