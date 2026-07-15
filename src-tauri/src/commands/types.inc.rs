use serde::Deserialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterInfo {
    pub system_name: String,
    pub display_name: String,
    pub is_default: bool,
    pub driver_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintOptions {
    pub page_range: Option<String>,
    pub orientation: String,
    pub paper_size: String,
    pub scaling: String,
    pub margins: PrintMargins,
    pub color_mode: String,
    #[serde(default = "default_true")]
    pub include_watermarks: bool,
    pub printer_name: Option<String>,
    pub copies: Option<u32>,
    pub duplex: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PrintMargins {
    Default,
    None,
    Custom { top: f64, right: f64, bottom: f64, left: f64 },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "camelCase")]
pub enum PrintDocumentResult {
    DirectJob { job_id: u64 },
    WindowsFallback { temp_path: String },
}

#[derive(Debug, Clone, Serialize)]
struct PdfDocumentMetadata {
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    keywords: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
    creation_date: Option<String>,
    mod_date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LinuxPackageRef {
    url: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Default)]
struct LinuxPackages {
    deb: Option<LinuxPackageRef>,
    rpm: Option<LinuxPackageRef>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LatestVersionInfo {
    version: String,
    notes: Option<String>,
    current: String,
    newer: bool,
    linux_packages: Option<LinuxPackages>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextLineInfo {
    text: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    font_family: String,
    font_size: f64,
    bold: bool,
    italic: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParagraphInfo {
    line_indices: Vec<usize>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfUaReport {
    pub tagged: bool,
    pub has_title: bool,
    pub language: Option<String>,
    pub figures_total: u32,
    pub figures_with_alt: u32,
    pub image_xobjects: u32,
    pub page_count: u32,
    pub encrypted: bool,
}

const EXPORT_PNG_W: i32 = pdf::export::EXPORT_RENDER_W;
const EXPORT_PNG_H: i32 = pdf::export::EXPORT_RENDER_H;
