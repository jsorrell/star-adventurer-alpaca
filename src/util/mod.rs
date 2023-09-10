#![allow(unused)]

use serde::{Deserialize, Serialize};

pub use crate::telescope_control::mount_limits::*;
pub use abort_result::*;
pub use enums::*;
pub use infinite_future::*;
pub use lockable::*;
pub use result::*;
pub use tasks::*;

mod abort_result;
mod tasks {
    pub use abortable_task::*;
    pub use waitable_task::*;

    mod abortable_task;
    mod waitable_task;
}

mod enums;
mod infinite_future;
mod lockable;
mod result;

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub struct AxisRate {
    #[serde(rename = "Maximum")]
    pub(crate) maximum: f64,
    #[serde(rename = "Minimum")]
    pub(crate) minimum: f64,
}
