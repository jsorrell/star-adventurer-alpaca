use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum EquatorialCoordinateType {
    Other = 0,
    Topocentric = 1,
    J2000 = 2,
    J2050 = 3,
    B1950 = 4,
}
