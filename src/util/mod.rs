pub mod consts;
pub mod enums;
pub mod result;

pub use consts::*;
pub use enums::*;
pub use result::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub struct AxisRateRange {
    #[serde(rename = "Maximum")]
    pub(crate) maximum: f64,
    #[serde(rename = "Minimum")]
    pub(crate) minimum: f64,
}
