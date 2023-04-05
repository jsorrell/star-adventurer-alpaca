pub use star_adventurer::StarAdventurer;

use crate::util::*;
use ascom_alpaca::api::{
    AlignmentModeResponse, EquatorialSystemResponse, TelescopeSetSideOfPierRequestSideOfPier,
};
use ascom_alpaca::{ASCOMError, ASCOMErrorCode, ASCOMResult};

mod connection;
mod commands {
    pub mod guide;
    pub mod observing_pos;
    pub mod parking;
    pub mod pointing_pos;
    pub mod slew;
    pub mod sync;
    pub mod target;
    pub mod tracking;
}
pub mod mount_limits;
mod slew_def;
mod star_adventurer;
#[cfg(test)]
pub(in crate::telescope_control) mod test_util;

impl StarAdventurer {
    /// Returns the alignment mode of the mount (Alt/Az, Polar, German Polar)
    pub async fn get_alignment_mode(&self) -> ASCOMResult<AlignmentModeResponse> {
        Ok(AlignmentModeResponse::GermanPolar)
    }

    /// Returns the current equatorial coordinate system used by this telescope (e.g. Topocentric or J2000).
    pub async fn get_equatorial_system(&self) -> ASCOMResult<EquatorialSystemResponse> {
        Ok(EquatorialSystemResponse::Topocentric)
    }

    /// The telescope's effective aperture diameter (meters)
    pub async fn get_aperture(&self) -> ASCOMResult<f64> {
        self.settings.telescope_details.aperture.ok_or_else(|| {
            ASCOMError::new(
                ASCOMErrorCode::VALUE_NOT_SET,
                "Aperture not defined".to_string(),
            )
        })
    }

    /// The area of the telescope's aperture, taking into account any obstructions (square meters)
    pub async fn get_aperture_area(&self) -> ASCOMResult<f64> {
        self.settings
            .telescope_details
            .aperture_area
            .ok_or_else(|| {
                ASCOMError::new(
                    ASCOMErrorCode::VALUE_NOT_SET,
                    "Aperture area not defined".to_string(),
                )
            })
    }

    /// The telescope's focal length in meters
    pub async fn get_focal_length(&self) -> ASCOMResult<f64> {
        self.settings.telescope_details.focal_length.ok_or_else(|| {
            ASCOMError::new(
                ASCOMErrorCode::VALUE_NOT_SET,
                "Focal length not defined".to_string(),
            )
        })
    }

    /// True if the mount is stopped in the Home position. Set only following a FindHome() operation, and reset with any slew operation.
    /// This property must be False if the telescope does not support homing.
    pub async fn is_home(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if the telescope or driver applies atmospheric refraction to coordinates.
    pub async fn does_refraction(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Tell the telescope or driver whether to apply atmospheric refraction to coordinates.
    pub async fn set_does_refraction(&self, _does_refraction: bool) -> ASCOMResult<()> {
        // TODO implement this?
        Err(ASCOMError::new(
            ASCOMErrorCode::NOT_IMPLEMENTED,
            "Refraction calculations not available".to_string(),
        ))
    }

    /// Indicates the pointing state of the mount
    pub async fn get_side_of_pier(&self) -> ASCOMResult<PierSide> {
        Ok(*self.settings.pier_side.read().await)
    }

    /// True if the SideOfPier property can be set, meaning that the mount can be forced to flip.
    pub async fn can_set_side_of_pier(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Sets the pointing state of the mount
    pub async fn set_side_of_pier(
        &self,
        _side: TelescopeSetSideOfPierRequestSideOfPier,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::new(
            ASCOMErrorCode::NOT_IMPLEMENTED,
            "Side of pier control not implemented".to_string(),
        ))
    }
}
