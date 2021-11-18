use crate::asset_pack::color_overrides::ColorOverrides;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct PackMeta {
    pub name: String,
    pub id: String,
    pub version: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_color_overrides: Option<ColorOverrides>,
}
