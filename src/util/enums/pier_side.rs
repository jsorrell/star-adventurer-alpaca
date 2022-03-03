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

impl PierSide {
    pub fn opposite(self) -> Self {
        match self {
            PierSide::Unknown => self,
            PierSide::East => PierSide::West,
            PierSide::West => PierSide::East,
        }
    }

    pub fn flip(&mut self) {
        match self {
            PierSide::Unknown => {}
            PierSide::East => *self = PierSide::West,
            PierSide::West => *self = PierSide::East,
        }
    }
}
