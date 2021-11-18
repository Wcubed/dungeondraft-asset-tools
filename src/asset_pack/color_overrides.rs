use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ColorOverrides {
    pub enabled: bool,
    pub min_redness: f32,
    pub min_saturation: f32,
    pub red_tolerance: f32,
}
