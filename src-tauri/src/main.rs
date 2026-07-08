#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod licenses;
mod pdf;

use pdf::bookmarks::{collect_outline_items, flat_outline_ids, remove_outline_item, PdfBookmarkEntry};
use pdf::content::append_page_content;
use pdf::coords::page_media_box;
use pdf::crop::apply_crop_margins;
use pdf::export::{validate_page_range, write_image_output as write_png_output, ExportImageKind, ParityPageRenderFn};
use pdf::fonts::dedup_fonts_after_insert;
use pdf::form_merge::merge_acroform_after_insert;
use pdf::history::HistorySnapshot;
use pdf::import::import_object;
use pdf::markdown::MarkdownSaveResult;
use pdf::markdown_images::MarkdownImageSink;
use pdf::markdown_pipeline::{pdf_plain_text_pages, pdf_to_markdown};
use pdf::metadata::{current_pdf_mod_date, ensure_info_dict_id, read_info_string, write_info_text_field};
use pdf::ocr::{
    build_tesseract_install_guide, OcrExportStats, OcrStatus, TesseractInstallGuide, OCR_RENDER_H, OCR_RENDER_W,
};
#[cfg(test)]
use pdf::ocr::{ocr_page_segmentation_mode, os_release_value};
use pdf::page_decor::build_page_border_ops;
use pdf::page_decor::{append_outline_item, build_page_number_ops, build_watermark_ops, create_blank_page};
use pdf::page_margins::{apply_expand_margins, apply_shrink_margins, page_size_preset_dims};
use pdf::page_sizes::PdfPageSize;
use pdf::page_text::{ensure_helvetica_font, viewer_point_to_pdf};
use pdf::page_tree::{flatten_pages, get_pages_kids, set_pages_kids};
use pdf::pdfium_bind::{
    get_pdfium, render_page_bmp, render_page_gif, render_page_ico, render_page_image, render_page_jpeg,
    render_page_png, render_page_ppm, render_page_tga, render_page_tiff, render_page_webp, set_bundled_pdfium_dir,
};
use pdf::rotation::{page_rotation, reset_page_rotation_at, rotate_all_pages_by, rotate_page_at, set_page_rotation};
use pdf::search::{search_pdf_text as search_pdf_text_impl, PdfTextSearchMatch};
use pdf::security::{PdfSignatureInfo, PdfSignatureVerificationSummary};
use pdf::summary::{PdfSummaryResult, SummarySaveResult};

use lopdf::{Document, Object, ObjectId};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::process::Command;
use tauri::Manager;
// `Emitter` (event emit) is only used by the single-instance handler, which is
// compiled out under the `wdio` (e2e) feature.
#[cfg(not(feature = "wdio"))]
use tauri::Emitter;

/// File paths captured from the initial process launch (file-association /
/// "Open With"). The frontend drains these via the `take_pending_open_paths`
/// command once its document loaders have mounted. Pulling on demand avoids a
/// startup race: emitting an event at launch could fire before any JS listener
/// is registered, silently dropping the file the user asked to open.
struct PendingOpenPaths(std::sync::Mutex<Vec<String>>);

/// Pick the window that should receive a forwarded "Open With" path: the focused
/// window, else "main", else any window. With multi-window, "main" may be closed
/// while spawned document windows remain, so always-targeting "main" could drop
/// the path. Only the single-instance handler calls this; it is compiled out
/// under the `wdio` (e2e) feature.
#[cfg(not(feature = "wdio"))]
fn target_open_window(app: &tauri::AppHandle) -> Option<tauri::WebviewWindow> {
    app.webview_windows()
        .values()
        .find(|w| w.is_focused().unwrap_or(false))
        .cloned()
        .or_else(|| app.get_webview_window("main"))
        .or_else(|| app.webview_windows().values().next().cloned())
}

include!("commands/types.inc.rs");
include!("commands/wrappers_render.inc.rs");
include!("commands/wrappers_page.inc.rs");

// COMMANDS_EXPORT_INCLUDE
include!("commands_export.inc.rs");
// END_COMMANDS_EXPORT_INCLUDE

// PARITY_DOCMOD_INCLUDE
include!("parity_docmod_generated.inc.rs");
// END_PARITY_DOCMOD_INCLUDE
// PARITY_BATCH_INCLUDE
include!("parity_batch_generated.inc.rs");
// END_PARITY_BATCH_INCLUDE
// PARITY_BATCH2_INCLUDE
include!("parity_batch2_generated.inc.rs");
// END_PARITY_BATCH2_INCLUDE
// PARITY_BATCH3_INCLUDE
include!("parity_batch3_generated.inc.rs");
// END_PARITY_BATCH3_INCLUDE
// PARITY_BATCH4_INCLUDE
include!("parity_batch4_generated.inc.rs");
// END_PARITY_BATCH4_INCLUDE
// PARITY_BATCH5_INCLUDE
include!("parity_batch5_generated.inc.rs");
// END_PARITY_BATCH5_INCLUDE
// PARITY_BATCH6_INCLUDE
include!("parity_batch6_generated.inc.rs");
// END_PARITY_BATCH6_INCLUDE
// PARITY_BATCH7_INCLUDE
include!("parity_batch7_generated.inc.rs");
// END_PARITY_BATCH7_INCLUDE
// PARITY_BATCH8_INCLUDE
include!("parity_batch8_generated.inc.rs");
// END_PARITY_BATCH8_INCLUDE

include!("commands/wrappers_doc.inc.rs");
include!("commands/wrappers_annot.inc.rs");

pub fn print_temp_dir() -> PathBuf {
    std::env::temp_dir().join("kanoprii_print")
}

fn cleanup_old_print_temp_files() {
    let dir = print_temp_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(24 * 60 * 60);
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if let Ok(modified) = meta.modified() {
            if modified < cutoff {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

fn main() {
    // webkit2gtk's DMABUF renderer aborts with `Gdk Error 71 (Protocol error)
    // dispatching to Wayland display` on some Wayland + GPU-driver combinations
    // (notably bleeding-edge multi-GPU NVIDIA + mesa stacks, where the cross-GPU
    // zero-copy buffer handoff to the compositor fails). Disabling it falls back
    // to the SHM presentation path - GPU compositing is still used, so the app
    // stays hardware-accelerated; only the zero-copy presentation is given up.
    // Must run before GTK/WebKit initialise. A value set by the user always
    // wins. Drop when webkit2gtk-4.1 ships a working DMABUF renderer here.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: single-threaded at the very start of main(), before the
        // Tauri/GTK builder or any other thread reads the environment. `unsafe`
        // is a no-op before edition 2024 (hence `allow(unused_unsafe)`) but
        // keeps the call correct after an edition bump.
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    cleanup_old_print_temp_files();

    #[cfg(feature = "wdio")]
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());
    #[cfg(not(feature = "wdio"))]
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // Forward non-flag argv entries to the frontend as open-path events.
            // `argv` is the second instance's full `std::env::args()`, so skip
            // argv[0] (the program path) - otherwise the binary's own path is
            // opened as a bogus document, accumulating a dead tab each time.
            let paths: Vec<String> = argv.into_iter().skip(1).filter(|a| !a.starts_with('-')).collect();
            if !paths.is_empty() {
                if let Some(w) = target_open_window(app) {
                    let _ = w.emit("open-path", paths);
                    let _ = w.set_focus();
                }
            }
        }));

    // Capture any file paths from this launch (file-association / "Open With").
    // argv[0] is the program path, so skip it and keep only non-flag arguments.
    // The frontend drains these via `take_pending_open_paths` after it mounts,
    // which is race-free - unlike emitting an event the listener may miss.
    let initial_open_paths: Vec<String> = std::env::args().skip(1).filter(|a| !a.starts_with('-')).collect();

    builder
        .manage(PendingOpenPaths(std::sync::Mutex::new(initial_open_paths)))
        .setup(|app| {
            // In a packaged build, PDFium ships under the app's resource
            // directory; record it so the loader can find it at runtime.
            if let Ok(resources) = app.path().resource_dir() {
                set_bundled_pdfium_dir(resources.join("vendor").join("pdfium"));
            }
            Ok(())
        })
        .invoke_handler(include!("commands/invoke_handler.inc.rs"))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
