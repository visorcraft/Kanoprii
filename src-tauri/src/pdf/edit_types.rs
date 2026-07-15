use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RgbColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    pub font_family: String,
    pub font_size: f64,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: RgbColor,
    pub align: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageImageInfo {
    pub index: usize,
    pub object_id: (u32, u16),
    pub resource_name: String,
    pub occurrence: usize,
    /// Axis-aligned hit bounds after rotation.
    pub bbox: PdfRect,
    /// Unrotated placement rectangle centered on the transformed image.
    pub rect: PdfRect,
    pub rotation: f64,
    pub width: u32,
    pub height: u32,
}
