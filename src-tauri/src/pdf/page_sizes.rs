use crate::pdf::coords::page_media_box;
use crate::pdf::rotation::page_rotation;
use lopdf::Document;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct PdfPageSize {
    pub width: f64,
    pub height: f64,
    pub rotation: i64,
}

pub fn get_pdf_page_sizes(path: &Path) -> Result<Vec<PdfPageSize>, String> {
    let doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut sizes = Vec::new();
    for page_id in doc.get_pages().into_values() {
        let media = page_media_box(&doc, page_id)?;
        sizes.push(PdfPageSize {
            width: media[2] - media[0],
            height: media[3] - media[1],
            rotation: page_rotation(&doc, page_id),
        });
    }
    Ok(sizes)
}
