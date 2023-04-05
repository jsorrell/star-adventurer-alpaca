use ascom_alpaca::api::SideOfPier;

pub trait PierSideExt {
    fn is_unknown(&self) -> bool;
    fn opposite(self) -> Self;
    fn flip(&mut self);
}

impl PierSideExt for SideOfPier {
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    fn opposite(self) -> Self {
        match self {
            SideOfPier::Unknown => self,
            SideOfPier::East => SideOfPier::West,
            SideOfPier::West => SideOfPier::East,
        }
    }

    fn flip(&mut self) {
        match self {
            SideOfPier::Unknown => {}
            SideOfPier::East => *self = SideOfPier::West,
            SideOfPier::West => *self = SideOfPier::East,
        }
    }
}
