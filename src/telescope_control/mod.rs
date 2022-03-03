pub use star_adventurer::StarAdventurer;

use crate::util::*;

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
mod star_adventurer;
#[cfg(test)]
pub(in crate::telescope_control) mod test_util;

impl StarAdventurer {
    /// Returns the alignment mode of the mount (Alt/Az, Polar, German Polar)
    pub async fn get_alignment_mode(&self) -> AscomResult<AlignmentMode> {
        Ok(AlignmentMode::GermanPolar)
    }

    /// Returns the current equatorial coordinate system used by this telescope (e.g. Topocentric or J2000).
    pub async fn get_equatorial_system(&self) -> AscomResult<EquatorialCoordinateType> {
        Ok(EquatorialCoordinateType::Topocentric)
    }

    /// The telescope's effective aperture diameter (meters)
    pub async fn get_aperture(&self) -> AscomResult<f64> {
        self.settings.telescope_details.aperture.ok_or_else(|| {
            AscomError::from_msg(
                AscomErrorType::ValueNotSet,
                "Aperture not defined".to_string(),
            )
        })
    }

    /// The area of the telescope's aperture, taking into account any obstructions (square meters)
    pub async fn get_aperture_area(&self) -> AscomResult<f64> {
        self.settings
            .telescope_details
            .aperture_area
            .ok_or_else(|| {
                AscomError::from_msg(
                    AscomErrorType::ValueNotSet,
                    "Aperture area not defined".to_string(),
                )
            })
    }

    /// The telescope's focal length in meters
    pub async fn get_focal_length(&self) -> AscomResult<f64> {
        self.settings.telescope_details.focal_length.ok_or_else(|| {
            AscomError::from_msg(
                AscomErrorType::ValueNotSet,
                "Focal length not defined".to_string(),
            )
        })
    }

    /// True if the mount is stopped in the Home position. Set only following a FindHome() operation, and reset with any slew operation.
    /// This property must be False if the telescope does not support homing.
    pub async fn is_home(&self) -> AscomResult<bool> {
        Ok(false)
    }

    /// True if the telescope or driver applies atmospheric refraction to coordinates.
    pub async fn does_refraction(&self) -> AscomResult<bool> {
        Ok(false)
    }

    /// Tell the telescope or driver whether to apply atmospheric refraction to coordinates.
    pub async fn set_does_refraction(&self, _does_refraction: bool) -> AscomResult<()> {
        // TODO implement this?
        Err(AscomError::from_msg(
            AscomErrorType::PropertyOrMethodNotImplemented,
            "Refraction calculations not available".to_string(),
        ))
    }

    /// Indicates the pointing state of the mount
    pub async fn get_side_of_pier(&self) -> AscomResult<PierSide> {
        Ok(*self.settings.pier_side.read().await)
    }

    /// True if the SideOfPier property can be set, meaning that the mount can be forced to flip.
    pub async fn can_set_side_of_pier(&self) -> AscomResult<bool> {
        Ok(false)
    }

    /// Sets the pointing state of the mount
    pub async fn set_side_of_pier(&self, _side: PierSide) -> AscomResult<()> {
        Err(AscomError::from_msg(
            AscomErrorType::PropertyOrMethodNotImplemented,
            "Side of pier control not implemented".to_string(),
        ))
    }
}
