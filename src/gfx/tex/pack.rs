/*!
Texture packing types
*/

use serde::Deserialize;

/// Simple serde data for texture packer
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TexPackRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Simple serde data for texture packer
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TexPackSprite {
    pub filename: String,
    // hum, this name is very confusing
    pub frame: TexPackRect,
}

/// Deserialized from texture packing JSON
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TexPack {
    pub frames: Vec<TexPackSprite>,
}
