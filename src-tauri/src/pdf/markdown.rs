use crate::pdf::ocr::{ocr_language, OcrExportStats};
use std::path::Path;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownSaveResult {
    pub markdown: String,
    pub markdown_path: String,
    pub written: bool,
    pub conflict: bool,
    pub ocr_available: bool,
    pub ocr_language: String,
    /// Scanned or sparse-text pages where OCR was attempted during export.
    pub pages_needing_ocr: u32,
    pub ocr_text_blocks: u32,
    pub ocr_missing_hints: u32,
}

pub fn write_markdown_file(
    markdown_path: &Path,
    markdown: &str,
    overwrite: bool,
) -> Result<MarkdownSaveResult, String> {
    if markdown_path.exists() {
        let existing = std::fs::read(markdown_path).map_err(|e| e.to_string())?;
        if existing == markdown.as_bytes() {
            return Ok(MarkdownSaveResult {
                markdown: markdown.to_string(),
                markdown_path: markdown_path.to_string_lossy().to_string(),
                written: false,
                conflict: false,
                ocr_available: false,
                ocr_language: ocr_language(),
                pages_needing_ocr: 0,
                ocr_text_blocks: 0,
                ocr_missing_hints: 0,
            });
        }
        if !overwrite {
            return Ok(MarkdownSaveResult {
                markdown: markdown.to_string(),
                markdown_path: markdown_path.to_string_lossy().to_string(),
                written: false,
                conflict: true,
                ocr_available: false,
                ocr_language: ocr_language(),
                pages_needing_ocr: 0,
                ocr_text_blocks: 0,
                ocr_missing_hints: 0,
            });
        }
    }

    std::fs::write(markdown_path, markdown).map_err(|e| e.to_string())?;
    Ok(MarkdownSaveResult {
        markdown: markdown.to_string(),
        markdown_path: markdown_path.to_string_lossy().to_string(),
        written: true,
        conflict: false,
        ocr_available: false,
        ocr_language: ocr_language(),
        pages_needing_ocr: 0,
        ocr_text_blocks: 0,
        ocr_missing_hints: 0,
    })
}

pub fn markdown_save_result(
    markdown: String,
    markdown_path: &Path,
    written: bool,
    conflict: bool,
    stats: &OcrExportStats,
) -> MarkdownSaveResult {
    MarkdownSaveResult {
        markdown,
        markdown_path: markdown_path.to_string_lossy().into_owned(),
        written,
        conflict,
        ocr_available: stats.available,
        ocr_language: stats.language.clone(),
        pages_needing_ocr: stats.scanned_pages + stats.sparse_supplements,
        ocr_text_blocks: stats.text_blocks,
        ocr_missing_hints: stats.missing_install_hints,
    }
}
