use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize_repr, Deserialize_repr, FromFormField)]
#[repr(u8)]
pub enum GuideDirection {
    #[field(value = "0")]
    North = 0,
    #[field(value = "1")]
    South = 1,
    #[field(value = "2")]
    East = 2,
    #[field(value = "3")]
    West = 3,
}
