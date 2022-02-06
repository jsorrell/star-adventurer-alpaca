use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize_repr, Deserialize_repr, FromFormField)]
#[repr(u8)]
pub enum Axis {
    /// RA or Az
    #[field(value = "0")]
    Primary = 0,
    /// Dec or Alt
    #[field(value = "1")]
    Secondary = 1,
    /// imager rotator/de-rotator
    #[field(value = "2")]
    Tertiary = 2,
}
