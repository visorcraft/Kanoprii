use crate::pdf::render;
use std::fs;
use std::path::Path;

pub const EXPORT_RENDER_W: i32 = 1600;
pub const EXPORT_RENDER_H: i32 = 2264;

pub type ParityPageRenderFn = fn(&Path, u32, i32, i32) -> Result<Vec<u8>, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportImageKind {
    Png,
    Jpeg,
    Webp,
    Bmp,
    Tiff,
    Gif,
    Ppm,
    Tga,
    Ico,
}

impl ExportImageKind {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::Gif => "gif",
            Self::Ppm => "ppm",
            Self::Tga => "tga",
            Self::Ico => "ico",
        }
    }

    pub fn page_file_name(self, page_index: u32) -> String {
        format!("page-{:03}.{}", page_index + 1, self.extension())
    }
}

pub fn validate_page_range(path: &Path, start_page: u32, end_page: u32) -> Result<(), String> {
    let total = render::cached_document(path).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    Ok(())
}

pub fn write_image_output(output_path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    fs::write(output_path, bytes).map_err(|e| e.to_string())
}

/// Render one PDF page to an image file (2× viewer resolution by default).
pub fn export_pdf_page(
    path: &Path,
    page_index: u32,
    output_path: &Path,
    render: ParityPageRenderFn,
    width: i32,
    height: i32,
) -> Result<String, String> {
    if !path.is_file() {
        return Err("File not found".to_string());
    }
    validate_page_range(path, page_index, page_index)?;
    let bytes = render(path, page_index, width, height)?;
    write_image_output(output_path, &bytes)?;
    Ok(output_path.to_string_lossy().into_owned())
}

/// Render a page range to `output_dir/page-NNN.<ext>` files.
#[allow(clippy::too_many_arguments)]
pub fn export_pdf_pages(
    path: &Path,
    start_page: u32,
    end_page: u32,
    output_dir: &Path,
    kind: ExportImageKind,
    render: ParityPageRenderFn,
    width: i32,
    height: i32,
) -> Result<Vec<String>, String> {
    if !path.is_file() {
        return Err("File not found".to_string());
    }
    validate_page_range(path, start_page, end_page)?;
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    let mut written = Vec::new();
    for page_index in start_page..=end_page {
        let bytes = render(path, page_index, width, height)?;
        let output_path = output_dir.join(kind.page_file_name(page_index));
        write_image_output(&output_path, &bytes)?;
        written.push(output_path.to_string_lossy().into_owned());
    }
    Ok(written)
}

/// Render odd- or even-indexed pages to `output_dir/page-NNN.<ext>` files.
pub fn export_pages_by_parity_rendered(
    path: &Path,
    output_dir: &Path,
    odd: bool,
    kind: ExportImageKind,
    render: ParityPageRenderFn,
    width: i32,
    height: i32,
) -> Result<Vec<String>, String> {
    if !path.is_file() {
        return Err("File not found".to_string());
    }
    let total = render::cached_document(path).map_err(|e| e.to_string())?.get_pages().len() as u32;
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    let mut written = Vec::new();
    for page_index in 0..total {
        if (page_index % 2 == 0) != odd {
            continue;
        }
        let bytes = render(path, page_index, width, height)?;
        let output_path = output_dir.join(kind.page_file_name(page_index));
        write_image_output(&output_path, &bytes)?;
        written.push(output_path.to_string_lossy().into_owned());
    }
    Ok(written)
}
