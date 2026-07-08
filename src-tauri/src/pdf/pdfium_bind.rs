use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use pdfium_render::prelude::*;

use crate::pdf::render;

static PDFIUM: OnceLock<Result<Mutex<Pdfium>, String>> = OnceLock::new();
/// Directory holding the bundled PDFium library, populated during Tauri `setup`
/// from the app's resource directory (only meaningful in a packaged build).
static BUNDLED_PDFIUM_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Try to bind a standard PDFium build at `dir`, recording the attempted path on
/// failure.
fn try_pdfium_dir(dir: &std::path::Path, tried: &mut Vec<String>) -> Option<Pdfium> {
    let candidate = Pdfium::pdfium_platform_library_name_at_path(dir);
    match Pdfium::bind_to_library(&candidate) {
        Ok(bindings) => Some(Pdfium::new(bindings)),
        Err(_) => {
            tried.push(candidate.to_string_lossy().into_owned());
            None
        }
    }
}

/// Bind to a standard PDFium library (the C `FPDF_*` API that `pdfium-render`
/// requires). Search order: an explicit `PDFIUM_LIB_PATH`, a `libpdfium` shipped
/// next to the executable, a vendored copy under the crate, then any system
/// library. The system's `libdeepin-pdfium` is a *different*, incompatible C++
/// API and is intentionally never used.
fn bind_pdfium() -> Result<Pdfium, String> {
    let mut tried: Vec<String> = Vec::new();

    // 1. Explicit override.
    if let Some(path) = std::env::var_os("PDFIUM_LIB_PATH") {
        let path = PathBuf::from(path);
        match Pdfium::bind_to_library(&path) {
            Ok(bindings) => return Ok(Pdfium::new(bindings)),
            Err(_) => tried.push(path.to_string_lossy().into_owned()),
        }
    }
    // 2. Next to the executable (bundled distribution).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if let Some(pdfium) = try_pdfium_dir(dir, &mut tried) {
                return Ok(pdfium);
            }
        }
    }
    // 2b. Tauri resource directory of a packaged build (set during setup).
    if let Some(dir) = BUNDLED_PDFIUM_DIR.get() {
        if let Some(pdfium) = try_pdfium_dir(dir, &mut tried) {
            return Ok(pdfium);
        }
    }
    // 3. Vendored copy under the crate (developer runs).
    let vendor = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/pdfium");
    if let Some(pdfium) = try_pdfium_dir(&vendor, &mut tried) {
        return Ok(pdfium);
    }
    // 4. Any system-installed PDFium.
    if let Ok(bindings) = Pdfium::bind_to_system_library() {
        return Ok(Pdfium::new(bindings));
    }
    tried.push("system library".to_string());

    Err(format!(
        "Could not load a standard PDFium library. The system's libdeepin-pdfium is a \
         different, incompatible API. Install libpdfium or set PDFIUM_LIB_PATH. Tried: {}",
        tried.join(", ")
    ))
}

/// Returns the process-wide PDFium binding, or a user-facing error string if no
/// compatible library is available (so commands surface a message instead of
/// aborting the app).
pub fn get_pdfium() -> Result<MutexGuard<'static, Pdfium>, String> {
    PDFIUM
        .get_or_init(|| bind_pdfium().map(Mutex::new))
        .as_ref()
        .map_err(|e| e.clone())?
        .lock()
        .map_err(|_| "PDFium renderer lock poisoned".to_string())
}

/// Record bundled PDFium directory from Tauri setup (packaged builds).
pub fn set_bundled_pdfium_dir(dir: PathBuf) {
    let _ = BUNDLED_PDFIUM_DIR.set(dir);
}

pub fn render_page_image(
    path: &Path,
    page_index: u32,
    width: i32,
    height: i32,
    format: image::ImageFormat,
) -> Result<Vec<u8>, String> {
    let pdfium = get_pdfium()?;
    render::render_page_bytes(&pdfium, path, page_index, width, height, format)
}

pub fn render_page_png(path: &Path, page_index: u32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    render_page_image(path, page_index, width, height, image::ImageFormat::Png)
}

pub fn render_page_jpeg(path: &Path, page_index: u32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    render_page_image(path, page_index, width, height, image::ImageFormat::Jpeg)
}

pub fn render_page_webp(path: &Path, page_index: u32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    render_page_image(path, page_index, width, height, image::ImageFormat::WebP)
}

pub fn render_page_bmp(path: &Path, page_index: u32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    render_page_image(path, page_index, width, height, image::ImageFormat::Bmp)
}

pub fn render_page_tiff(path: &Path, page_index: u32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    render_page_image(path, page_index, width, height, image::ImageFormat::Tiff)
}

pub fn render_page_gif(path: &Path, page_index: u32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    render_page_image(path, page_index, width, height, image::ImageFormat::Gif)
}

pub fn render_page_ppm(path: &Path, page_index: u32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    render_page_image(path, page_index, width, height, image::ImageFormat::Pnm)
}

pub fn render_page_tga(path: &Path, page_index: u32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    render_page_image(path, page_index, width, height, image::ImageFormat::Tga)
}

pub fn render_page_ico(path: &Path, page_index: u32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    render_page_image(path, page_index, width, height, image::ImageFormat::Ico)
}
