#[derive(Copy, Clone, Debug)]
pub enum AbortResult<T, U> {
    Completed(T),
    Aborted(U),
}

impl<T, U> AbortResult<T, U> {
    pub fn unwrap_completed(self) -> T {
        match self {
            Self::Completed(v) => v,
            Self::Aborted(_) => panic!("Called unwrap on Aborted"),
        }
    }

    pub fn unwrap_aborted(self) -> U {
        match self {
            Self::Aborted(v) => v,
            Self::Completed(_) => panic!("Called unwrap_abort on Completed"),
        }
    }

    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed(_))
    }

    pub fn is_aborted(&self) -> bool {
        matches!(self, Self::Aborted(_))
    }
}

impl<T> AbortResult<T, T> {
    pub fn unwrap(self) -> T {
        match self {
            Self::Completed(v) => v,
            Self::Aborted(v) => v,
        }
    }
}
