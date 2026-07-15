/// Deep-copy `start_page`..=`end_page` and insert the copies at the document start.
#[tauri::command]
fn duplicate_page_range_to_start(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    pdf::page_range::duplicate_page_range_to_start(&PathBuf::from(path), start_page, end_page)
}
/// Insert a new page at `at_index` containing a centered copy of `image_path`.
#[tauri::command]
fn insert_image_page(path: String, at_index: u32, image_path: String) -> Result<u32, String> {
    pdf::page_images::insert_image_page(&PathBuf::from(path), at_index, &PathBuf::from(image_path))
}
/// Write a single page from the open PDF to `output_path` (does not modify the source).
#[tauri::command]
fn export_page_as_pdf(path: String, page_index: u32, output_path: String) -> Result<String, String> {
    pdf::page_images::export_page_as_pdf(&PathBuf::from(path), page_index, &PathBuf::from(output_path))
}
#[tauri::command]
fn get_image_dimensions(path: String) -> Result<[u32; 2], String> {
    pdf::page_images::get_image_dimensions(&PathBuf::from(path))
}
#[tauri::command]
fn add_page_image(
    path: String,
    page_index: u32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    image_path: String,
) -> Result<(), String> {
    pdf::page_images::add_page_image(&PathBuf::from(path), page_index, x, y, width, height, &PathBuf::from(image_path))
}
#[tauri::command]
fn add_page_text(path: String, page_index: u32, x: f64, y: f64, font_size: f64, text: String) -> Result<u32, String> {
    pdf::page_text::add_page_text(&PathBuf::from(path), page_index, x, y, font_size, text)
}
#[tauri::command]
fn list_page_text_edits(path: String, page_index: u32) -> Result<Vec<pdf::page_text::PageTextEdit>, String> {
    pdf::page_text::list_page_text_edits(&PathBuf::from(path), page_index)
}
#[tauri::command]
fn update_page_text(
    path: String,
    page_index: u32,
    index: u32,
    text: String,
    x: Option<f64>,
    y: Option<f64>,
    font_size: Option<f64>,
) -> Result<(), String> {
    pdf::page_text::update_page_text(&PathBuf::from(path), page_index, index, text, x, y, font_size)
}
#[tauri::command]
fn remove_page_text(path: String, page_index: u32, index: u32) -> Result<(), String> {
    pdf::page_text::remove_page_text(&PathBuf::from(path), page_index, index)
}
#[tauri::command]
fn add_page_vector_rect(path: String, page_index: u32, x: f64, y: f64, width: f64, height: f64) -> Result<u32, String> {
    pdf::page_text::add_page_vector_rect(&PathBuf::from(path), page_index, x, y, width, height)
}
#[tauri::command]
fn list_page_vectors(path: String, page_index: u32) -> Result<Vec<pdf::page_text::PageVectorEdit>, String> {
    pdf::page_text::list_page_vectors(&PathBuf::from(path), page_index)
}
#[tauri::command]
fn remove_page_vector(path: String, page_index: u32, index: u32) -> Result<(), String> {
    pdf::page_text::remove_page_vector(&PathBuf::from(path), page_index, index)
}
#[tauri::command]
fn get_pdf_form_fields(path: String) -> Result<Vec<pdf::forms::FormFieldData>, String> {
    pdf::forms::get_pdf_form_fields(&PathBuf::from(path))
}
#[tauri::command]
fn set_pdf_form_field(path: String, name: String, value: String) -> Result<(), String> {
    pdf::forms::set_pdf_form_field(&PathBuf::from(path), name, value)
}
#[tauri::command]
fn add_text_form_field(
    path: String,
    page_index: u32,
    name: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    pdf::forms::add_text_form_field(&PathBuf::from(path), page_index, name, x, y, width, height)
}
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn add_checkbox_form_field(
    path: String,
    page_index: u32,
    name: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    checked: bool,
) -> Result<(), String> {
    pdf::forms::add_checkbox_form_field(&PathBuf::from(path), page_index, name, x, y, width, height, checked)
}
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn add_choice_form_field(
    path: String,
    page_index: u32,
    name: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    options: Vec<String>,
    combo: bool,
) -> Result<(), String> {
    pdf::forms::add_choice_form_field(&PathBuf::from(path), page_index, name, x, y, width, height, options, combo)
}
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn add_radio_form_field(
    path: String,
    page_index: u32,
    group_name: String,
    option_name: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    pdf::forms::add_radio_form_field(&PathBuf::from(path), page_index, group_name, option_name, x, y, width, height)
}
#[tauri::command]
fn summarize_pdf(path: String) -> Result<PdfSummaryResult, String> {
    let path = PathBuf::from(path);
    if !path.is_file() {
        return Err(format!("file not found: {}", path.display()));
    }
    let (pages, scanned) = pdf_plain_text_pages(&path)?;
    Ok(pdf::summary::build_pdf_summary(&pages, scanned))
}
#[tauri::command]
fn save_pdf_summary(path: String, overwrite: bool) -> Result<SummarySaveResult, String> {
    let pdf_path = PathBuf::from(path);
    if !pdf_path.is_file() {
        return Err(format!("file not found: {}", pdf_path.display()));
    }
    let (pages, scanned) = pdf_plain_text_pages(&pdf_path)?;
    let summary = pdf::summary::build_pdf_summary(&pages, scanned);
    pdf::summary::save_summary_file(&pdf_path, &summary, overwrite)
}
/// Return on-disk byte length for undo snapshot sizing decisions.
#[tauri::command]
fn file_byte_size(path: String) -> Result<u64, String> {
    Ok(fs::metadata(path).map_err(|e| e.to_string())?.len())
}
/// Whether the UI should offer native open/save pickers. macOS/Windows and Linux
/// X11 use them by default; Linux Wayland requires `KANOPRII_NATIVE_DIALOGS=1`.
#[tauri::command]
fn native_file_dialogs_enabled() -> bool {
    pdf::browser::native_file_dialogs_policy(
        cfg!(target_os = "macos"),
        cfg!(target_os = "windows"),
        cfg!(target_os = "linux"),
        std::env::var_os("WAYLAND_DISPLAY").is_some(),
        std::env::var("KANOPRII_NATIVE_DIALOGS").ok().as_deref(),
        std::env::var("KANOPRII_DISABLE_NATIVE_DIALOGS").ok().as_deref(),
    )
}
/// Drain file paths captured at launch (file-association / "Open With").
/// Returns them once and clears the buffer, so the frontend can open them
/// after its document loaders mount - avoiding a startup event-delivery race.
#[tauri::command]
fn take_pending_open_paths(pending: tauri::State<'_, PendingOpenPaths>) -> Vec<String> {
    let mut guard = pending.0.lock().unwrap_or_else(|e| e.into_inner());
    std::mem::take(&mut *guard)
}
/// Resolve the on-disk target for a rename: same directory as `original`, base
/// name `new_name`, forcing a single `.pdf` extension. Rejects path separators.
fn resolve_rename_target(original: &Path, new_name: &str) -> Result<PathBuf, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("Name cannot contain path separators".into());
    }
    let dir = original.parent().ok_or("File has no parent directory")?;
    let mut base = trimmed.trim_end_matches('.').to_string();
    if !base.to_lowercase().ends_with(".pdf") {
        base.push_str(".pdf");
    }
    Ok(dir.join(base))
}
/// Rename the PDF on disk (in its current directory). Returns the new path.
#[tauri::command]
fn rename_document(original_path: String, new_file_name: String) -> Result<String, String> {
    let original = Path::new(&original_path);
    if !original.exists() {
        return Err("Original file no longer exists".into());
    }
    let target = resolve_rename_target(original, &new_file_name)?;
    if target == original {
        return Ok(original_path);
    }
    if target.exists() {
        let name = target.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        return Err(format!("A file named \"{name}\" already exists"));
    }
    std::fs::rename(original, &target).map_err(|e| e.to_string())?;
    Ok(target.to_string_lossy().into_owned())
}
/// Reveal a file in the OS file manager, selecting it where supported.
#[tauri::command]
fn reveal_in_file_manager(path: String) -> Result<(), String> {
    use std::process::Command;
    let p = Path::new(&path);
    if !p.exists() {
        return Err("File no longer exists".into());
    }
    let parent = p.parent().ok_or("No parent directory")?;
    #[cfg(target_os = "linux")]
    {
        let uri = format!("file://{path}");
        let dbus = Command::new("dbus-send")
            .args([
                "--session",
                "--print-reply",
                "--dest=org.freedesktop.FileManager1",
                "--type=method_call",
                "/org/freedesktop/FileManager1",
                "org.freedesktop.FileManager1.ShowItems",
                &format!("array:string:{uri}"),
                "string:",
            ])
            .status();
        if matches!(dbus, Ok(s) if s.success()) {
            return Ok(());
        }
        Command::new("xdg-open").arg(parent).status().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").args(["-R", &path]).status().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer").arg(format!("/select,{path}")).status().map_err(|e| e.to_string())?;
    }
    let _ = parent;
    Ok(())
}
/// Text/heuristic Markdown preview without assets or OCR (API/tests only).
/// The UI Markdown view uses `save_pdf_markdown`, which runs the full export pipeline.
#[tauri::command]
fn convert_pdf_to_markdown(path: String) -> Result<String, String> {
    pdf_to_markdown(&PathBuf::from(path), None, None)
}
#[tauri::command]
fn read_text_document(path: String) -> Result<String, String> {
    let path = PathBuf::from(path);
    let supported = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown" | "html" | "htm" | "css"));
    if !supported {
        return Err("Only Markdown, HTML, and CSS documents are supported".to_string());
    }
    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    if metadata.len() > 16 * 1024 * 1024 {
        return Err("Text document is larger than 16 MB".to_string());
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}
#[tauri::command]
fn create_pdf_from_document_pages(pages: Vec<Vec<u8>>) -> Result<String, String> {
    let path = pdf::history::temp_hist_path("document", "pdf");
    pdf::page_images::create_pdf_from_image_pages(&pages, &path)?;
    Ok(path.to_string_lossy().into_owned())
}
#[tauri::command]
fn create_pdf_from_markdown_text(text: String) -> Result<String, String> {
    pdf::markdown_pdf::write_markdown_pdf(&text)
}
#[tauri::command]
fn materialize_document_pdf(working: String, source: String) -> Result<String, String> {
    let source = PathBuf::from(source);
    let mut target = source.with_extension("pdf");
    if target.exists() {
        let stem = source.file_stem().and_then(|value| value.to_str()).unwrap_or("document");
        target = source.with_file_name(format!("{stem}_converted.pdf"));
        let mut suffix = 2;
        while target.exists() {
            target = source.with_file_name(format!("{stem}_converted_{suffix}.pdf"));
            suffix += 1;
        }
    }
    pdf::history::save_working_copy(working, target.to_string_lossy().into_owned())?;
    Ok(target.to_string_lossy().into_owned())
}
#[tauri::command]
fn discard_document_pdf(generated: String, source: String) -> Result<(), String> {
    pdf::history::discard_document_pdf(generated, source)
}
#[tauri::command]
fn save_pdf_markdown(path: String, overwrite: bool, output_path: Option<String>) -> Result<MarkdownSaveResult, String> {
    let pdf_path = PathBuf::from(path);
    let markdown_path = output_path.map(PathBuf::from).unwrap_or_else(|| pdf_path.with_extension("md"));
    let assets_folder = format!("{}_assets", markdown_path.file_stem().and_then(|s| s.to_str()).unwrap_or("document"));
    let assets_dir = markdown_path
        .parent()
        .map(|parent| parent.join(&assets_folder))
        .unwrap_or_else(|| PathBuf::from(&assets_folder));
    let sink = MarkdownImageSink { assets_dir: &assets_dir, rel_prefix: &assets_folder };
    let mut stats = OcrExportStats::default();
    let markdown = pdf_to_markdown(&pdf_path, Some(&sink), Some(&mut stats))?;
    let file_result = pdf::markdown::write_markdown_file(&markdown_path, &markdown, overwrite)?;
    Ok(pdf::markdown::markdown_save_result(
        file_result.markdown,
        &markdown_path,
        file_result.written,
        file_result.conflict,
        &stats,
    ))
}
#[tauri::command]
fn split_pdf(path: String, page_ranges: Vec<(u32, u32)>) -> Result<Vec<String>, String> {
    pdf::merge_split::split_pdf(&PathBuf::from(path), page_ranges)
}
/// Write a new PDF containing only `start_page`..=`end_page` from `path`.
#[tauri::command]
fn extract_pdf_pages(path: String, output_path: String, start_page: u32, end_page: u32) -> Result<String, String> {
    pdf::merge_split::extract_pdf_pages(&PathBuf::from(path), &PathBuf::from(output_path), start_page, end_page)
}
#[tauri::command]
fn insert_pdf(
    path: String,
    insert_path: String,
    at_index: u32,
    insert_start: u32,
    insert_end: u32,
) -> Result<(), String> {
    pdf::merge_split::insert_pdf(&PathBuf::from(path), &PathBuf::from(insert_path), at_index, insert_start, insert_end)
}
#[tauri::command]
fn optimize_pdf(path: String) -> Result<String, String> {
    pdf::optimize::optimize_pdf_file(&PathBuf::from(path))
}
/// OCR scanned pages and write an invisible text layer for search/select.
#[tauri::command]
fn make_pdf_searchable(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    pdf::ocr_layer::make_pdf_searchable(&PathBuf::from(path), start_page, end_page)
}
/// Burn in redaction boxes, replace affected pages, and optionally re-OCR.
#[tauri::command]
fn apply_redactions(path: String, ocr_after: bool) -> Result<u32, String> {
    pdf::redact::apply_redactions(&PathBuf::from(path), ocr_after)
}
#[tauri::command]
fn has_redaction_boxes(path: String) -> Result<bool, String> {
    pdf::redact::has_redaction_boxes(&PathBuf::from(path))
}
#[tauri::command]
fn pdf_is_encrypted(path: String) -> Result<bool, String> {
    pdf::security::pdf_is_encrypted(path)
}
#[tauri::command]
fn verify_pdf_password(path: String, password: String) -> Result<(), String> {
    pdf::security::verify_pdf_password(path, password)
}
#[tauri::command]
fn open_working_copy_with_password(original: String, password: String) -> Result<String, String> {
    pdf::security::open_working_copy_with_password(original, password)
}
#[tauri::command]
fn save_session_state(app: tauri::AppHandle, state: pdf::io::SessionState) -> Result<(), String> {
    pdf::io::save_session_state(&app, &state)
}
#[tauri::command]
fn load_session_state(app: tauri::AppHandle) -> Result<Option<pdf::io::SessionState>, String> {
    pdf::io::load_session_state(&app)
}
#[tauri::command]
fn inspect_pdfua(path: String) -> Result<PdfUaReport, String> {
    pdf::ua::inspect_pdfua(&PathBuf::from(path))
}
