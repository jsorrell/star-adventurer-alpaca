use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum AlignmentMode {
    /// Altitude-Azimuth alignment
    AltAz = 0,
    /// Polar (equatorial) mount other than German equatorial
    Polar = 1,
    /// German equatorial mount
    GermanPolar = 2,
}
