use rocket::form::FromFormField;
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize_repr, Deserialize_repr, FromFormField)]
#[repr(i8)]
pub enum PierSide {
    #[field(value = "-1")]
    Unknown = -1,
    #[field(value = "0")]
    East = 0,
    #[field(value = "1")]
    West = 1,
}
