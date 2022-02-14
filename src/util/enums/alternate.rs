#![allow(unused)]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Alternate<T, U> {
    A(T),
    B(U),
}

impl<T, U> Alternate<T, U> {
    pub fn is_a(&self) -> bool {
        matches!(self, Self::A(_))
    }

    pub fn is_b(&self) -> bool {
        matches!(self, Self::B(_))
    }

    pub fn get_a(self) -> Option<T> {
        match self {
            Self::A(v) => Some(v),
            Self::B(_) => None,
        }
    }

    pub fn get_b(self) -> Option<U> {
        match self {
            Self::B(v) => Some(v),
            Self::A(_) => None,
        }
    }

    pub fn a_or(self, default: T) -> T {
        match self {
            Self::A(v) => v,
            Self::B(_) => default,
        }
    }

    pub fn b_or(self, default: U) -> U {
        match self {
            Self::B(v) => v,
            Self::A(_) => default,
        }
    }
}
