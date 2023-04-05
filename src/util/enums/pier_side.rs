pub use ascom_alpaca::api::SideOfPierResponse as PierSide;

pub trait PierSideExt {
    fn is_unknown(&self) -> bool;
    fn opposite(self) -> Self;
    fn flip(&mut self);
}

impl PierSideExt for PierSide {
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    fn opposite(self) -> Self {
        match self {
            PierSide::Unknown => self,
            PierSide::East => PierSide::West,
            PierSide::West => PierSide::East,
        }
    }

    fn flip(&mut self) {
        match self {
            PierSide::Unknown => {}
            PierSide::East => *self = PierSide::West,
            PierSide::West => *self = PierSide::East,
        }
    }
}
