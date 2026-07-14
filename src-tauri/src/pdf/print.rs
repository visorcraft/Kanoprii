use std::collections::BTreeSet;
use std::path::Path;

use crate::pdf::coords::obj_to_f64;
use crate::pdf::page_tree::set_pages_kids;
use crate::{PrintDocumentResult, PrintMargins, PrintOptions, PrinterInfo};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

pub fn list_printers() -> Vec<PrinterInfo> {
    use printers::{get_default_printer, get_printers};

    let default_name = get_default_printer().map(|p| p.system_name.clone()).unwrap_or_default();

    get_printers()
        .into_iter()
        .map(|p| PrinterInfo {
            system_name: p.system_name.clone(),
            display_name: if p.name.is_empty() { p.system_name.clone() } else { p.name.clone() },
            is_default: p.system_name == default_name,
            driver_name: p.driver_name,
        })
        .collect()
}

pub fn print_document(source_path: &Path, opts: &PrintOptions, temp_dir: &Path) -> Result<PrintDocumentResult, String> {
    #[cfg(not(target_os = "windows"))]
    let printer_name = opts.printer_name.as_ref().ok_or("Printer name is required")?;
    #[cfg(not(target_os = "windows"))]
    let copies = opts.copies.ok_or("Copies is required")?;
    #[cfg(not(target_os = "windows"))]
    let duplex = opts.duplex.as_deref().ok_or("Duplex is required")?;

    let doc = crate::pdf::render::cached_document(source_path).map_err(|e| e.to_string())?;
    let page_count = doc.get_pages().len() as u32;
    let selected = parse_page_range(opts.page_range.as_deref(), page_count)?;

    let temp_name = format!(
        "print_{}_{}.pdf",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
    );
    let temp_path = temp_dir.join(temp_name);
    #[cfg(target_os = "windows")]
    let mut guard = TempFileGuard::new(&temp_path);
    #[cfg(not(target_os = "windows"))]
    let _guard = TempFileGuard::new(&temp_path);
    build_print_pdf(source_path, opts, &selected, &temp_path)?;

    #[cfg(target_os = "windows")]
    {
        open_pdf_for_manual_print(&temp_path)?;
        guard.disarm();
        Ok(PrintDocumentResult::WindowsFallback { temp_path: temp_path.to_string_lossy().into_owned() })
    }

    #[cfg(not(target_os = "windows"))]
    {
        use printers::common::base::job::PrinterJobOptions;
        use printers::common::converters::Converter;
        use printers::get_printer_by_name;

        let printer =
            get_printer_by_name(printer_name).ok_or_else(|| format!("Printer not found: {}", printer_name))?;

        let copies_str = copies.to_string();
        let mut props: Vec<(&str, &str)> = vec![("copies", &copies_str), ("media", &opts.paper_size)];
        let sides = match duplex {
            "simplex" => "one-sided",
            "longEdge" => "two-sided-long-edge",
            "shortEdge" => "two-sided-short-edge",
            _ => "one-sided",
        };
        props.push(("sides", sides));
        if opts.color_mode.eq_ignore_ascii_case("grayscale") {
            // ponytail: force monochrome at the printer. Two keys cover both
            // driverless/IPP queues (print-color-mode) and legacy PPD drivers
            // (ColorModel); unknown keys are ignored by lp.
            props.push(("print-color-mode", "monochrome"));
            props.push(("ColorModel", "Gray"));
        }

        let job_id = printer
            .print_file(
                temp_path.to_str().ok_or("Temp path is not valid UTF-8")?,
                PrinterJobOptions {
                    name: Some("Kanoprii print job"),
                    raw_properties: &props,
                    converter: Converter::None,
                },
            )
            .map_err(|e| format!("Print failed: {:?}", e))?;

        Ok(PrintDocumentResult::DirectJob { job_id })
    }
}

struct TempFileGuard<'a> {
    path: &'a Path,
    disarmed: bool,
}

impl<'a> TempFileGuard<'a> {
    fn new(path: &'a Path) -> Self {
        Self { path, disarmed: false }
    }

    fn disarm(&mut self) {
        self.disarmed = true;
    }
}

impl<'a> Drop for TempFileGuard<'a> {
    fn drop(&mut self) {
        if !self.disarmed {
            let _ = std::fs::remove_file(self.path);
        }
    }
}

#[cfg(target_os = "windows")]
fn open_pdf_for_manual_print(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::ShellExecuteExW;
    use windows::Win32::UI::Shell::SEE_MASK_NOCLOSEPROCESS;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let operation: Vec<u16> = "print\0".encode_utf16().collect();
    let mut info = windows::Win32::UI::Shell::SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<windows::Win32::UI::Shell::SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: windows::core::PCWSTR(operation.as_ptr()),
        lpFile: windows::core::PCWSTR(wide.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    unsafe {
        ShellExecuteExW(&mut info).map_err(|e| e.to_string())?;
        // SEE_MASK_NOCLOSEPROCESS leaves the child's process handle open in
        // `info.hProcess`; without CloseHandle each print leaks one kernel
        // handle until process exit.
        if !info.hProcess.is_invalid() {
            windows::Win32::Foundation::CloseHandle(info.hProcess).ok();
        }
    }
    Ok(())
}

pub fn print_to_pdf(source_path: &Path, opts: &PrintOptions, output_path: &Path) -> Result<(), String> {
    let doc = crate::pdf::render::cached_document(source_path).map_err(|e| e.to_string())?;
    let page_count = doc.get_pages().len() as u32;
    let selected = parse_page_range(opts.page_range.as_deref(), page_count)?;
    build_print_pdf(source_path, opts, &selected, output_path)?;
    Ok(())
}

pub fn render_print_preview(
    source_path: &Path,
    page_index: u32,
    opts: &PrintOptions,
    width: i32,
    height: i32,
    temp_dir: &Path,
) -> Result<Vec<u8>, String> {
    let temp_name = format!(
        "preview_{}_{}_{}.pdf",
        std::process::id(),
        page_index,
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
    );
    let temp_path = temp_dir.join(temp_name);
    let mut guard = TempFileGuard::new(&temp_path);

    build_print_pdf(source_path, opts, &[page_index], &temp_path)?;

    let bytes = crate::pdf::pdfium_bind::render_page_png(&temp_path, 0, width, height)?;
    guard.disarm();
    let _ = std::fs::remove_file(&temp_path);
    Ok(bytes)
}

/// Build a transformed print PDF from the source, laying out the selected pages
/// onto the requested paper size with scaling, margins, and orientation applied.
pub fn build_print_pdf(
    source_path: &Path,
    opts: &PrintOptions,
    selected_pages: &[u32],
    output_path: &Path,
) -> Result<(), String> {
    let mut doc = crate::pdf::render::cached_document(source_path).map_err(|e| e.to_string())?;

    apply_redactions_for_print(&mut doc)?;
    flatten_annotations_for_print(&mut doc)?;
    if opts.include_watermarks {
        force_show_print_ocg(&mut doc)?;
    } else {
        suppress_hidden_ocg_print(&mut doc)?;
    }

    let target_size = paper_size_in_points(&opts.paper_size, &opts.orientation)?;

    let new_page_ids: Vec<ObjectId> = selected_pages
        .iter()
        .map(|&idx| {
            let page_id = *doc.get_pages().get(&(idx + 1)).ok_or_else(|| format!("Page index out of bounds: {idx}"))?;
            transform_page_for_print(&mut doc, page_id, target_size, opts)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let pages_ref = {
        let catalog = doc.catalog().map_err(|e| e.to_string())?;
        catalog
            .get(b"Pages")
            .map_err(|_| "No Pages entry in catalog".to_string())?
            .as_reference()
            .map_err(|_| "Pages entry is not a reference".to_string())?
    };

    for &page_id in &new_page_ids {
        if let Ok(dict) = doc.get_dictionary_mut(page_id) {
            dict.set(b"Parent", Object::Reference(pages_ref));
        }
    }

    let new_kids: Vec<Object> = new_page_ids.iter().map(|id| Object::Reference(*id)).collect();
    set_pages_kids(&mut doc, pages_ref, new_kids)?;

    if let Ok(catalog) = doc.catalog_mut() {
        catalog.remove(b"AcroForm");
    }

    crate::pdf::io::save_atomic(&mut doc, output_path)?;
    Ok(())
}

pub fn apply_redactions_for_print(doc: &mut Document) -> Result<(), String> {
    crate::pdf::redact::apply_redactions_to_doc(doc)?;
    Ok(())
}

pub fn flatten_annotations_for_print(doc: &mut Document) -> Result<(), String> {
    crate::pdf::page_decor::flatten_all_annotations_in_doc(doc)?;
    Ok(())
}

/// Resolve the OCProperties /D default-config dictionary, handling both
/// indirect-Reference and inline-Dictionary forms (PDF 32000-1 §8.11).
/// Returns `None` if OCProperties is absent (file has no optional content).
fn ocg_d_dict(doc: &Document) -> Option<&Dictionary> {
    let ocprops = doc.catalog().ok()?.get(b"OCProperties").ok()?;
    let ocprops_dict = match ocprops {
        Object::Reference(id) => doc.get_dictionary(*id).ok()?,
        Object::Dictionary(d) => d,
        _ => return None,
    };
    match ocprops_dict.get(b"D").ok()? {
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        Object::Dictionary(d) => Some(d),
        _ => None,
    }
}

/// Resolve the /OCProperties/D dictionary's *indirect* ObjectId so we can
/// mutate it. Returns `None` if /D is inline (uncommon; inline /D is rare per
/// PDF 32000-1 §8.11 — Acrobat uses indirect refs). Inline-/D mutation is
/// not currently implemented; callers must accept the no-op.
fn ocg_d_id(doc: &Document) -> Option<ObjectId> {
    let ocprops_dict = doc.catalog().ok()?.get(b"OCProperties").ok()?;
    let d_obj = match ocprops_dict {
        Object::Reference(id) => doc.get_dictionary(*id).ok()?.get(b"D").ok()?.clone(),
        Object::Dictionary(d) => d.get(b"D").ok()?.clone(),
        _ => return None,
    };
    match d_obj {
        Object::Reference(id) => Some(id),
        _ => None,
    }
}

fn apply_as_filter_at(doc: &mut Document, d_id: ObjectId, filtered: Vec<Object>) {
    let Ok(d) = doc.get_dictionary_mut(d_id) else { return };
    if filtered.is_empty() {
        d.remove(b"AS");
    } else {
        d.set(b"AS", Object::Array(filtered));
    }
}

fn collect_ocg_refs(obj: &Object, out: &mut BTreeSet<ObjectId>) {
    match obj {
        Object::Reference(id) => {
            out.insert(*id);
        }
        Object::Array(items) => {
            for item in items {
                collect_ocg_refs(item, out);
            }
        }
        _ => {}
    }
}

fn print_ocgs(doc: &Document) -> BTreeSet<ObjectId> {
    let mut ids = BTreeSet::new();
    if let Some(d) = ocg_d_dict(doc) {
        if let Some(as_arr) = d.get(b"AS").ok().and_then(|o| o.as_array().ok()) {
            for entry in as_arr {
                let Ok(dict) = entry.as_dict() else { continue };
                if dict.get(b"Event").ok().and_then(|e| e.as_name().ok()) == Some(b"Print") {
                    if let Ok(ocgs) = dict.get(b"OCGs") {
                        collect_ocg_refs(ocgs, &mut ids);
                    }
                }
            }
        }
    }
    ids
}

fn force_show_print_ocg(doc: &mut Document) -> Result<(), String> {
    let ids = print_ocgs(doc);
    if ids.is_empty() {
        return Ok(());
    }

    if let Some(d_id) = ocg_d_id(doc) {
        if let Ok(d) = doc.get_dictionary_mut(d_id) {
            if let Some(off) = d.get(b"OFF").ok().and_then(|o| o.as_array().ok()) {
                let filtered: Vec<Object> = off
                    .iter()
                    .filter(|entry| entry.as_reference().map_or(true, |id| !ids.contains(&id)))
                    .cloned()
                    .collect();
                if filtered.is_empty() {
                    d.remove(b"OFF");
                } else {
                    d.set(b"OFF", Object::Array(filtered));
                }
            }

            let mut on = d.get(b"ON").ok().and_then(|o| o.as_array().ok()).cloned().unwrap_or_default();
            for id in &ids {
                if !on.iter().any(|entry| entry.as_reference().ok() == Some(*id)) {
                    on.push(Object::Reference(*id));
                }
            }
            d.set(b"ON", Object::Array(on));
        }
    }

    for ocg_id in ids {
        if let Ok(ocg) = doc.get_dictionary_mut(ocg_id) {
            let mut usage = ocg.get(b"Usage").ok().and_then(|o| o.as_dict().ok()).cloned().unwrap_or_default();
            let mut view = usage.get(b"View").ok().and_then(|o| o.as_dict().ok()).cloned().unwrap_or_default();
            view.set(b"ViewState", Object::Name(b"ON".to_vec()));
            usage.set(b"View", Object::Dictionary(view));
            let mut print = usage.get(b"Print").ok().and_then(|o| o.as_dict().ok()).cloned().unwrap_or_default();
            print.set(b"PrintState", Object::Name(b"ON".to_vec()));
            usage.set(b"Print", Object::Dictionary(print));
            ocg.set(b"Usage", Object::Dictionary(usage));
        }
    }

    Ok(())
}

/// Optional-content "print-only" watermarks (e.g. a university "unofficial
/// copy" layer) hide themselves on screen but force `PrintState /ON` via the
/// OCProperties default-config `/AS` Print usage-application. That prints a
/// layer the user never sees. For a print copy we enforce WYSIWYG: make print
/// honour the same default visibility as viewing, so layers in `/OFF` stay off.
///
/// ponytail: handles the default config `/D` only (the one every renderer uses
/// by default). Alternate configs in `/Configs` are ignored — add them if a
/// real file ever selects a non-default config for printing.
fn suppress_hidden_ocg_print(doc: &mut Document) -> Result<(), String> {
    // OCGs that are OFF for viewing (hidden on screen).
    let off_ocgs: Vec<ObjectId> = ocg_d_dict(doc)
        .and_then(|d| d.get(b"OFF").ok())
        .and_then(|o| o.as_array().ok())
        .map(|a| a.iter().filter_map(|o| o.as_reference().ok()).collect())
        .unwrap_or_default();

    if off_ocgs.is_empty() {
        return Ok(());
    }

    // Drop any Print-event usage-application so printing falls back to the
    // /ON + /OFF defaults (which mirror the on-screen view).
    if let Some(d_id) = ocg_d_id(doc) {
        let filtered: Option<Vec<Object>> =
            doc.get_dictionary(d_id).ok().and_then(|d| d.get(b"AS").ok()).and_then(|o| o.as_array().ok()).map(|arr| {
                arr.iter()
                    .filter(|entry| {
                        // Keep the entry unless its /Event is /Print.
                        entry.as_dict().ok().and_then(|ed| ed.get(b"Event").ok()).and_then(|e| e.as_name().ok())
                            != Some(b"Print")
                    })
                    .cloned()
                    .collect()
            });
        if let Some(filtered) = filtered {
            apply_as_filter_at(doc, d_id, filtered);
        }
    }

    // Defensive: force PrintState /OFF on each hidden OCG, for renderers that
    // read per-OCG /Usage directly instead of the /D config.
    for ocg_id in off_ocgs {
        if let Ok(ocg) = doc.get_dictionary_mut(ocg_id) {
            let mut usage = ocg.get(b"Usage").ok().and_then(|o| o.as_dict().ok()).cloned().unwrap_or_default();
            let mut print = usage.get(b"Print").ok().and_then(|o| o.as_dict().ok()).cloned().unwrap_or_default();
            print.set(b"PrintState", Object::Name(b"OFF".to_vec()));
            usage.set(b"Print", Object::Dictionary(print));
            ocg.set(b"Usage", Object::Dictionary(usage));
        }
    }

    Ok(())
}

/// Inverse of `suppress_hidden_ocg_print`: drop the `/View`-event usage-
/// application so view-OFF layers (e.g. "unofficial transcript" watermarks)
/// become visible in the rendered output. Also flips each hidden OCG's
/// `ViewState` to `ON` for renderers that read per-OCG `/Usage` directly.
///
/// Used by the "Hidden watermarks/layers" viewer toggle. **Note**: this is an
/// *upper bound* on what print would show — it surfaces every layer that the
/// document considers hidden-on-screen. The print path additionally drops
/// print-only (`/PrintState/ON + /ViewState/OFF`) OCGs when
/// `include_watermarks=false`, so with that checkbox off the printed output
/// has fewer visible layers than this toggle over-shows. That's intentional:
/// the toggle answers "what hidden content does this file contain?", not
/// "what will this exact print job emit?".
pub fn force_show_all_ocg(doc: &mut Document) -> Result<(), String> {
    let off_ocgs: Vec<ObjectId> = ocg_d_dict(doc)
        .and_then(|d| d.get(b"OFF").ok())
        .and_then(|o| o.as_array().ok())
        .map(|a| a.iter().filter_map(|o| o.as_reference().ok()).collect())
        .unwrap_or_default();

    // Drop the /View-event /AS entry; the /ON + /OFF defaults then take over,
    // which means everything not explicitly hidden becomes visible.
    if let Some(d_id) = ocg_d_id(doc) {
        let filtered: Option<Vec<Object>> =
            doc.get_dictionary(d_id).ok().and_then(|d| d.get(b"AS").ok()).and_then(|o| o.as_array().ok()).map(|arr| {
                arr.iter()
                    .filter(|entry| {
                        entry.as_dict().ok().and_then(|ed| ed.get(b"Event").ok()).and_then(|e| e.as_name().ok())
                            != Some(b"View")
                    })
                    .cloned()
                    .collect()
            });
        if let Some(filtered) = filtered {
            apply_as_filter_at(doc, d_id, filtered);
        }
    }

    // Defensive: force per-OCG ViewState /ON on each currently-hidden OCG.
    for ocg_id in off_ocgs {
        if let Ok(ocg) = doc.get_dictionary_mut(ocg_id) {
            let mut usage = ocg.get(b"Usage").ok().and_then(|o| o.as_dict().ok()).cloned().unwrap_or_default();
            let mut view = usage.get(b"View").ok().and_then(|o| o.as_dict().ok()).cloned().unwrap_or_default();
            view.set(b"ViewState", Object::Name(b"ON".to_vec()));
            usage.set(b"View", Object::Dictionary(view));
            ocg.set(b"Usage", Object::Dictionary(usage));
        }
    }

    Ok(())
}

/// Build a temporary copy of `source_path` with all hidden OCG layers forced
/// visible, then render `page_index` to PNG bytes at the requested size and
/// clean up the temp file. Returns the PNG bytes.
pub fn render_pdf_page_with_all_layers_visible(
    source_path: &Path,
    page_index: u32,
    width: i32,
    height: i32,
    temp_dir: &Path,
) -> Result<Vec<u8>, String> {
    use std::path::PathBuf;

    // ponytail: cap render dims. Centralised in render.rs::MAX_RENDER_DIM so
    // render_page_bytes and this command share one limit. PDFium will happily
    // allocate a multi-GB bitmap on width=i32::MAX, which is reachable from
    // a hostile JS frontend.
    const MAX_DIM: i32 = crate::pdf::render::MAX_RENDER_DIM;
    if !(1..=MAX_DIM).contains(&width) || !(1..=MAX_DIM).contains(&height) {
        return Err(format!("Render dimensions must be 1..={MAX_DIM}"));
    }

    let mut doc = crate::pdf::render::cached_document(source_path).map_err(|e| e.to_string())?;
    force_show_all_ocg(&mut doc)?;
    // ponytail: temp name uses pid+nanos+counter (matches save_atomic's
    // scheme) so two concurrent calls in the same nanosecond can't collide.
    let nanos =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let counter = OCG_TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temp_path: PathBuf = temp_dir.join(format!("ocg_show_{}_{}_{}.pdf", std::process::id(), nanos, counter));
    crate::pdf::io::save_atomic(&mut doc, &temp_path)?;
    // ponytail: RAII guard so the temp source PDF is removed even if
    // render_page_png panics (e.g. PDFium OOM). Best-effort cleanup before
    // the guard runs is a no-op redundancy.
    struct TempGuard(PathBuf);
    impl Drop for TempGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _guard = TempGuard(temp_path.clone());
    crate::pdf::pdfium_bind::render_page_png(&temp_path, page_index, width, height)
}

static OCG_TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Return the page dimensions in PDF points for the requested paper size.
/// Orientation may be "portrait" or "landscape" (case-insensitive).
pub fn paper_size_in_points(paper_size: &str, orientation: &str) -> Result<(f64, f64), String> {
    let (w, h) = match paper_size.to_lowercase().as_str() {
        "a4" => (595.0, 842.0),
        "letter" => (612.0, 792.0),
        "legal" => (612.0, 1008.0),
        _ => return Err(format!("Unsupported paper size: {paper_size}")),
    };
    if orientation.eq_ignore_ascii_case("landscape") {
        Ok((h, w))
    } else {
        Ok((w, h))
    }
}

// ponytail: reject NaN/Inf on custom margins at the deserialise boundary.
// `compute_scale` is NaN-safe by way of its `mw <= 0.0` short-circuit, but
// the target-vs-margin subtraction `target_w - margin - margin` is not, so a
// hostile custom margin leaks NaN into the `cm` matrix printed into the PDF.
fn margins_to_array(margins: &PrintMargins) -> Result<[f64; 4], String> {
    match margins {
        PrintMargins::None => Ok([0.0, 0.0, 0.0, 0.0]),
        PrintMargins::Default => Ok([36.0, 36.0, 36.0, 36.0]),
        PrintMargins::Custom { top, right, bottom, left } => {
            for (name, v) in [("top", *top), ("right", *right), ("bottom", *bottom), ("left", *left)] {
                crate::pdf::coords::finite_f64(v, name)?;
            }
            Ok([*top, *right, *bottom, *left])
        }
    }
}

fn compute_scale(page_w: f64, page_h: f64, available_w: f64, available_h: f64, scaling: &str) -> f64 {
    // ponytail: reject NaN/Inf explicitly. `page_w <= 0.0` is false for NaN,
    // so a hostile MediaBox with [NaN, ..., ...] would otherwise leak NaN
    // into the printed 'cm' transform matrix.
    if !page_w.is_finite() || !page_h.is_finite() || !available_w.is_finite() || !available_h.is_finite() {
        return 1.0;
    }
    match scaling.to_lowercase().as_str() {
        "fit" | "shrinktofit" | "fittopage" => {
            if page_w <= 0.0 || page_h <= 0.0 {
                return 1.0;
            }
            let sx = available_w / page_w;
            let sy = available_h / page_h;
            sx.min(sy)
        }
        "fill" => {
            if page_w <= 0.0 || page_h <= 0.0 {
                return 1.0;
            }
            let sx = available_w / page_w;
            let sy = available_h / page_h;
            sx.max(sy)
        }
        _ => 1.0,
    }
}

/// Clone the source page, apply scaling/translation so its content fits the
/// target paper size, and return the new page object id.
fn transform_page_for_print(
    doc: &mut Document,
    page_id: ObjectId,
    target_size: (f64, f64),
    opts: &PrintOptions,
) -> Result<ObjectId, String> {
    let mut page_dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?.clone();

    let media = page_dict
        .get(b"MediaBox")
        .map_err(|_| "Page MediaBox missing".to_string())?
        .as_array()
        .map_err(|_| "Bad MediaBox".to_string())?
        .clone();
    if media.len() < 4 {
        return Err("MediaBox has fewer than 4 values".to_string());
    }
    let media_rect = [obj_to_f64(&media[0]), obj_to_f64(&media[1]), obj_to_f64(&media[2]), obj_to_f64(&media[3])];
    let page_w = media_rect[2] - media_rect[0];
    let page_h = media_rect[3] - media_rect[1];

    let margins = margins_to_array(&opts.margins)?;
    let (target_w, target_h) = target_size;
    let available_w = (target_w - margins[1] - margins[3]).max(0.0);
    let available_h = (target_h - margins[0] - margins[2]).max(0.0);

    let scale = compute_scale(page_w, page_h, available_w, available_h, &opts.scaling);
    let scaled_w = page_w * scale;
    let scaled_h = page_h * scale;
    let tx = margins[3] + (available_w - scaled_w) / 2.0;
    let ty = margins[2] + (available_h - scaled_h) / 2.0;

    let mut new_content = Vec::new();
    new_content.extend_from_slice(b"q\n");

    new_content.extend_from_slice(format!("{scale:.6} 0 0 {scale:.6} {tx:.6} {ty:.6} cm\n").as_bytes());

    let existing = collect_page_content_bytes(doc, page_id)?;
    new_content.extend_from_slice(&existing);
    new_content.extend_from_slice(b"\nQ");

    let content_stream = Stream::new(Dictionary::new(), new_content);
    let content_id = doc.add_object(Object::Stream(content_stream));

    page_dict.set(b"Contents", Object::Reference(content_id));
    page_dict.set(
        b"MediaBox",
        Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(target_w as f32),
            Object::Real(target_h as f32),
        ]),
    );
    page_dict.remove(b"Annots");
    page_dict.remove(b"Parent");

    let new_page_id = doc.add_object(Object::Dictionary(page_dict));
    Ok(new_page_id)
}

fn stream_decompressed_bytes(stream: &Stream) -> Vec<u8> {
    let mut s = stream.clone();
    match s.decompress() {
        Ok(()) => s.content,
        Err(_) => stream.content.clone(),
    }
}

fn collect_page_content_bytes(doc: &Document, page_id: ObjectId) -> Result<Vec<u8>, String> {
    let contents = doc.get_dictionary(page_id).map_err(|e| e.to_string())?.get(b"Contents").ok().cloned();

    let mut out = Vec::new();
    match contents {
        Some(Object::Reference(id)) => {
            let obj = doc.get_object(id).map_err(|e| e.to_string())?;
            if let Object::Stream(stream) = obj {
                out.extend_from_slice(&stream_decompressed_bytes(stream));
            }
        }
        Some(Object::Array(arr)) => {
            for item in arr {
                let id = item.as_reference().map_err(|_| "Bad content reference".to_string())?;
                let obj = doc.get_object(id).map_err(|e| e.to_string())?;
                if let Object::Stream(stream) = obj {
                    out.extend_from_slice(&stream_decompressed_bytes(stream));
                }
            }
        }
        _ => {}
    }
    Ok(out)
}

// ponytail: cap the selected-pages list so a hostile range like "1-99999999"
// can't allocate hundreds of MB or trigger hundreds of millions of page
// transforms. 10 000 pages is well past any real print job.
const MAX_PRINT_PAGES_PER_JOB: u32 = 10_000;

fn parse_page_range(range: Option<&str>, page_count: u32) -> Result<Vec<u32>, String> {
    // ponytail: reject huge docs BEFORE allocating the indices Vec. A 100M-page
    // PDF loaded for print would otherwise allocate ~400 MiB before the cap fires.
    if page_count > MAX_PRINT_PAGES_PER_JOB {
        return Err(format!("Document has {} pages; max per print job is {}", page_count, MAX_PRINT_PAGES_PER_JOB));
    }
    let indices: Vec<u32> = (0..page_count).collect();
    let Some(spec) = range else {
        return cap(indices, page_count);
    };
    let spec = spec.trim();
    if spec.is_empty() || spec.eq_ignore_ascii_case("all") {
        return cap(indices, page_count);
    }

    let mut out = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start, end)) = part.split_once('-') {
            let start: u32 = start.trim().parse().map_err(|_| format!("Invalid range: {}", part))?;
            let end: u32 = end.trim().parse().map_err(|_| format!("Invalid range: {}", part))?;
            if start == 0 || end < start || end > page_count {
                return Err(format!("Range out of bounds: {}", part));
            }
            out.extend((start - 1)..end);
        } else {
            let idx: u32 = part.parse().map_err(|_| format!("Invalid page: {}", part))?;
            if idx == 0 || idx > page_count {
                return Err(format!("Page out of bounds: {}", part));
            }
            out.push(idx - 1);
        }
    }
    if out.is_empty() {
        return Err("No pages selected".into());
    }
    out.sort_unstable();
    out.dedup();
    cap(out, page_count)
}

fn cap(out: Vec<u32>, page_count: u32) -> Result<Vec<u32>, String> {
    // Defensive cap (page_count was already checked up-front). Kept for the
    // post-de-dup case where dedup could shrink the list; today this is
    // always a no-op.
    if out.len() as u64 > MAX_PRINT_PAGES_PER_JOB as u64 {
        return Err(format!("Selection exceeds {} pages ({} pages in document)", MAX_PRINT_PAGES_PER_JOB, page_count));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Build a doc with a print-only watermark OCG (view OFF / print ON) and
    /// verify suppress_hidden_ocg_print flips PrintState OFF and drops the
    /// Print-event usage-application so the layer no longer prints.
    #[test]
    fn suppress_hidden_ocg_neutralizes_print_only_watermark() {
        let mut doc = Document::with_version("1.5");

        let mut ocg = Dictionary::new();
        ocg.set(b"Type", Object::Name(b"OCG".to_vec()));
        let mut print = Dictionary::new();
        print.set(b"PrintState", Object::Name(b"ON".to_vec()));
        let mut view = Dictionary::new();
        view.set(b"ViewState", Object::Name(b"OFF".to_vec()));
        let mut usage = Dictionary::new();
        usage.set(b"Print", Object::Dictionary(print));
        usage.set(b"View", Object::Dictionary(view));
        ocg.set(b"Usage", Object::Dictionary(usage));
        let ocg_id = doc.add_object(Object::Dictionary(ocg));

        let mut view_as = Dictionary::new();
        view_as.set(b"Event", Object::Name(b"View".to_vec()));
        view_as.set(b"OCGs", Object::Array(vec![Object::Reference(ocg_id)]));
        let mut print_as = Dictionary::new();
        print_as.set(b"Event", Object::Name(b"Print".to_vec()));
        print_as.set(b"OCGs", Object::Array(vec![Object::Reference(ocg_id)]));
        let mut d = Dictionary::new();
        d.set(b"AS", Object::Array(vec![Object::Dictionary(view_as), Object::Dictionary(print_as)]));
        d.set(b"OFF", Object::Array(vec![Object::Reference(ocg_id)]));
        let d_id = doc.add_object(Object::Dictionary(d));

        let mut ocprops = Dictionary::new();
        ocprops.set(b"D", Object::Reference(d_id));
        ocprops.set(b"OCGs", Object::Array(vec![Object::Reference(ocg_id)]));
        let ocprops_id = doc.add_object(Object::Dictionary(ocprops));

        let mut catalog = Dictionary::new();
        catalog.set(b"Type", Object::Name(b"Catalog".to_vec()));
        catalog.set(b"OCProperties", Object::Reference(ocprops_id));
        let catalog_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set(b"Root", Object::Reference(catalog_id));

        suppress_hidden_ocg_print(&mut doc).unwrap();

        // OCG print state forced OFF.
        let ps = doc
            .get_dictionary(ocg_id)
            .unwrap()
            .get(b"Usage")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Print")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"PrintState")
            .unwrap()
            .as_name()
            .unwrap();
        assert_eq!(ps, b"OFF");

        // Print-event usage-application removed; View entry retained.
        let as_arr = doc.get_dictionary(d_id).unwrap().get(b"AS").unwrap().as_array().unwrap();
        assert_eq!(as_arr.len(), 1);
        let ev = as_arr[0].as_dict().unwrap().get(b"Event").unwrap().as_name().unwrap();
        assert_eq!(ev, b"View");
    }

    #[test]
    fn force_show_print_ocg_surfaces_print_watermark_only() {
        let mut doc = Document::with_version("1.5");

        let print_ocg_id = {
            let mut print = Dictionary::new();
            print.set(b"PrintState", Object::Name(b"ON".to_vec()));
            let mut view = Dictionary::new();
            view.set(b"ViewState", Object::Name(b"OFF".to_vec()));
            let mut usage = Dictionary::new();
            usage.set(b"Print", Object::Dictionary(print));
            usage.set(b"View", Object::Dictionary(view));
            let mut ocg = Dictionary::new();
            ocg.set(b"Usage", Object::Dictionary(usage));
            doc.add_object(Object::Dictionary(ocg))
        };
        let hidden_ocg_id = {
            let mut view = Dictionary::new();
            view.set(b"ViewState", Object::Name(b"OFF".to_vec()));
            let mut usage = Dictionary::new();
            usage.set(b"View", Object::Dictionary(view));
            let mut ocg = Dictionary::new();
            ocg.set(b"Usage", Object::Dictionary(usage));
            doc.add_object(Object::Dictionary(ocg))
        };

        let mut view_as = Dictionary::new();
        view_as.set(b"Event", Object::Name(b"View".to_vec()));
        view_as.set(b"OCGs", Object::Array(vec![Object::Reference(print_ocg_id), Object::Reference(hidden_ocg_id)]));
        let mut print_as = Dictionary::new();
        print_as.set(b"Event", Object::Name(b"Print".to_vec()));
        print_as.set(b"OCGs", Object::Array(vec![Object::Reference(print_ocg_id)]));
        let mut d = Dictionary::new();
        d.set(b"AS", Object::Array(vec![Object::Dictionary(view_as), Object::Dictionary(print_as)]));
        d.set(b"OFF", Object::Array(vec![Object::Reference(print_ocg_id), Object::Reference(hidden_ocg_id)]));
        let d_id = doc.add_object(Object::Dictionary(d));

        let mut ocprops = Dictionary::new();
        ocprops.set(b"D", Object::Reference(d_id));
        let ocprops_id = doc.add_object(Object::Dictionary(ocprops));
        let mut catalog = Dictionary::new();
        catalog.set(b"OCProperties", Object::Reference(ocprops_id));
        let catalog_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set(b"Root", Object::Reference(catalog_id));

        force_show_print_ocg(&mut doc).unwrap();

        let d = doc.get_dictionary(d_id).unwrap();
        let off = d.get(b"OFF").unwrap().as_array().unwrap();
        assert_eq!(off, &vec![Object::Reference(hidden_ocg_id)]);
        let on = d.get(b"ON").unwrap().as_array().unwrap();
        assert!(on.contains(&Object::Reference(print_ocg_id)));
        let print_view_state = doc
            .get_dictionary(print_ocg_id)
            .unwrap()
            .get(b"Usage")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"View")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"ViewState")
            .unwrap()
            .as_name()
            .unwrap();
        assert_eq!(print_view_state, b"ON");
        let hidden_view_state = doc
            .get_dictionary(hidden_ocg_id)
            .unwrap()
            .get(b"Usage")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"View")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"ViewState")
            .unwrap()
            .as_name()
            .unwrap();
        assert_eq!(hidden_view_state, b"OFF");
    }

    /// No /OCProperties at all: suppression is a harmless no-op.
    #[test]
    fn suppress_hidden_ocg_noop_without_optional_content() {
        let mut doc = Document::with_version("1.5");
        let mut catalog = Dictionary::new();
        catalog.set(b"Type", Object::Name(b"Catalog".to_vec()));
        let catalog_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set(b"Root", Object::Reference(catalog_id));
        assert!(suppress_hidden_ocg_print(&mut doc).is_ok());
    }

    fn test_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kanoprii_print_test_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn minimal_blank_pdf(path: &Path) {
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();
        doc.objects.insert(content_id, Object::Stream(Stream::new(Dictionary::new(), b"BT ET".to_vec())));
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
        page.set("Contents", Object::Reference(content_id));
        doc.objects.insert(page_id, Object::Dictionary(page));
        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name(b"Pages".to_vec()));
        pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let catalog_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.save(path).unwrap();
    }

    #[test]
    fn parse_page_range_all() {
        assert_eq!(parse_page_range(Some("all"), 5).unwrap(), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn parse_page_range_comma_and_dash() {
        assert_eq!(parse_page_range(Some("1-3,5"), 5).unwrap(), vec![0, 1, 2, 4]);
    }

    #[test]
    fn build_print_pdf_a4_landscape() {
        let dir = test_dir();
        let source = dir.join("source.pdf");
        let output = dir.join("output.pdf");

        minimal_blank_pdf(&source);

        let opts = PrintOptions {
            page_range: None,
            orientation: "landscape".to_string(),
            paper_size: "a4".to_string(),
            scaling: "fit".to_string(),
            margins: PrintMargins::None,
            color_mode: "color".to_string(),
            include_watermarks: true,
            printer_name: None,
            copies: None,
            duplex: None,
        };

        build_print_pdf(&source, &opts, &[0], &output).unwrap();

        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let media = doc.get_dictionary(page_id).unwrap().get(b"MediaBox").unwrap().as_array().unwrap();
        let w = obj_to_f64(&media[2]) - obj_to_f64(&media[0]);
        let h = obj_to_f64(&media[3]) - obj_to_f64(&media[1]);
        assert!((w - 842.0).abs() < 0.1, "expected width ~842, got {w}");
        assert!((h - 595.0).abs() < 0.1, "expected height ~595, got {h}");

        let _ = fs::remove_file(&source);
        let _ = fs::remove_file(&output);
    }

    #[test]
    fn print_to_pdf_creates_file() {
        let dir = test_dir();
        let source = dir.join("source.pdf");
        let output = dir.join("print_output.pdf");

        minimal_blank_pdf(&source);

        let opts = PrintOptions {
            page_range: None,
            orientation: "portrait".to_string(),
            paper_size: "Letter".to_string(),
            scaling: "fitToPage".to_string(),
            margins: PrintMargins::Default,
            color_mode: "color".to_string(),
            include_watermarks: true,
            printer_name: None,
            copies: None,
            duplex: None,
        };

        print_to_pdf(&source, &opts, &output).unwrap();
        assert!(output.exists());

        let _ = fs::remove_file(&source);
        let _ = fs::remove_file(&output);
    }

    #[test]
    fn render_print_preview_returns_png() {
        let dir = test_dir();
        let source = dir.join("source.pdf");

        minimal_blank_pdf(&source);

        let opts = PrintOptions {
            page_range: None,
            orientation: "portrait".into(),
            paper_size: "A4".into(),
            scaling: "none".into(),
            margins: PrintMargins::None,
            color_mode: "color".into(),
            include_watermarks: true,
            printer_name: None,
            copies: None,
            duplex: None,
        };

        let png = render_print_preview(&source, 0, &opts, 400, 600, &dir).unwrap();
        assert!(png.starts_with(b"\x89PNG"));

        let _ = fs::remove_file(&source);
    }

    fn minimal_pdf_with_annot(path: &Path) {
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();
        doc.objects.insert(content_id, Object::Stream(Stream::new(Dictionary::new(), b"BT ET".to_vec())));
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
        page.set("Contents", Object::Reference(content_id));

        let annot = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Highlight".to_vec())),
            (
                b"Rect".to_vec(),
                Object::Array(vec![Object::Real(10.0), Object::Real(10.0), Object::Real(50.0), Object::Real(30.0)]),
            ),
        ])));
        page.set("Annots", Object::Array(vec![Object::Reference(annot)]));

        doc.objects.insert(page_id, Object::Dictionary(page));
        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name(b"Pages".to_vec()));
        pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let catalog_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.save(path).unwrap();
    }

    fn minimal_pdf_with_red_square(path: &Path) {
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        // Red filled rectangle from (100,100) to (200,200)
        let ops = b"1 0 0 rg 100 100 100 100 re f BT ET".to_vec();
        let content_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), ops)));
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
        page.set("Contents", Object::Reference(content_id));
        doc.objects.insert(page_id, Object::Dictionary(page));
        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name(b"Pages".to_vec()));
        pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let catalog_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.save(path).unwrap();
    }

    fn add_redaction_to_doc(doc: &mut Document, page_id: ObjectId, rect: [f64; 4]) {
        let annot = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Square".to_vec())),
            (
                b"Rect".to_vec(),
                Object::Array(vec![
                    Object::Real(rect[0] as f32),
                    Object::Real(rect[1] as f32),
                    Object::Real(rect[2] as f32),
                    Object::Real(rect[3] as f32),
                ]),
            ),
            (b"C".to_vec(), Object::Array(vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)])),
            (b"IC".to_vec(), Object::Array(vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)])),
            (b"Border".to_vec(), Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Real(0.0)])),
            (b"PandaRedact".to_vec(), Object::Boolean(true)),
        ])));
        let page_dict = doc.get_dictionary_mut(page_id).unwrap();
        match page_dict.get_mut(b"Annots") {
            Ok(Object::Array(arr)) => arr.push(Object::Reference(annot)),
            _ => page_dict.set(b"Annots", Object::Array(vec![Object::Reference(annot)])),
        }
    }

    #[test]
    fn build_print_pdf_flattens_annotations() {
        let dir = test_dir();
        let source = dir.join("source.pdf");
        let output = dir.join("output.pdf");

        minimal_pdf_with_annot(&source);

        let opts = PrintOptions {
            page_range: None,
            orientation: "portrait".to_string(),
            paper_size: "letter".to_string(),
            scaling: "none".to_string(),
            margins: PrintMargins::None,
            color_mode: "color".to_string(),
            include_watermarks: true,
            printer_name: None,
            copies: None,
            duplex: None,
        };

        build_print_pdf(&source, &opts, &[0], &output).unwrap();

        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let page_dict = doc.get_dictionary(page_id).unwrap();
        assert!(page_dict.get(b"Annots").is_err());

        let _ = fs::remove_file(&source);
        let _ = fs::remove_file(&output);
    }

    #[test]
    fn build_print_pdf_applies_redactions() {
        let dir = test_dir();
        let source = dir.join("source.pdf");
        let output = dir.join("output.pdf");

        minimal_pdf_with_red_square(&source);
        {
            let mut doc = Document::load(&source).unwrap();
            let page_id = *doc.get_pages().get(&1).unwrap();
            // Redaction box covering the red square.
            add_redaction_to_doc(&mut doc, page_id, [100.0, 100.0, 200.0, 200.0]);
            doc.save(&source).unwrap();
        }

        let opts = PrintOptions {
            page_range: None,
            orientation: "portrait".to_string(),
            paper_size: "letter".to_string(),
            scaling: "none".to_string(),
            margins: PrintMargins::None,
            color_mode: "color".to_string(),
            include_watermarks: true,
            printer_name: None,
            copies: None,
            duplex: None,
        };

        build_print_pdf(&source, &opts, &[0], &output).unwrap();

        let doc = Document::load(&output).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let page_dict = doc.get_dictionary(page_id).unwrap();
        assert!(page_dict.get(b"Annots").is_err());

        let resources = page_dict.get(b"Resources").unwrap().as_dict().unwrap();
        let xobjects = resources.get(b"XObject").unwrap().as_dict().unwrap();
        assert!(!xobjects.is_empty(), "redacted page should contain an image XObject");

        // Render the output and verify the redacted region is black.
        let png = crate::pdf::pdfium_bind::render_page_png(&output, 0, 400, 582).unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgb8();
        let (iw, ih) = img.dimensions();
        let sx = f64::from(iw) / 612.0;
        let sy = f64::from(ih) / 792.0;
        // PDF y runs from the bottom; map a point inside the redaction box.
        let cx = ((150.0 * sx) as u32).min(iw - 1);
        let cy = (ih - 1 - (150.0 * sy) as u32).min(ih - 1);
        let pixel = img.get_pixel(cx, cy);
        assert!(pixel[0] < 30 && pixel[1] < 30 && pixel[2] < 30, "redacted area should be black, got {:?}", pixel);

        let _ = fs::remove_file(&source);
        let _ = fs::remove_file(&output);
    }
}

#[cfg(test)]
mod cap_tests {
    use super::parse_page_range;

    #[test]
    fn parse_page_range_rejects_out_of_bounds_range() {
        // Opencode flagged the prior name as misleading: this test exercises
        // the explicit-range bounds check (returns "out of bounds"), not the
        // cap on the document's page count. Renamed for accuracy.
        let res = parse_page_range(Some("1-100000000"), 100);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("out of bounds"));
    }

    #[test]
    fn parse_page_range_caps_huge_document() {
        // The "all" / None path must reject a document with more pages than
        // MAX_PRINT_PAGES_PER_JOB before allocating the indices vector.
        let res = parse_page_range(None, 100_000_000);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("max per print job"));
    }

    #[test]
    fn parse_page_range_allows_normal_jobs() {
        assert_eq!(parse_page_range(Some("1-100"), 100).unwrap().len(), 100);
        assert_eq!(parse_page_range(None, 50).unwrap().len(), 50);
        assert_eq!(parse_page_range(Some("5"), 10).unwrap(), vec![4]);
    }
}

#[cfg(test)]
mod inline_ocg_tests {
    use super::*;
    use lopdf::{Dictionary, Object, ObjectId};

    // Inline-OCG form (no /OCProperties indirect Reference). PDF 32000-1 §8.11
    // permits this; opencode flagged that both suppress_hidden_ocg_print and
    // force_show_all_ocg previously returned Ok without applying changes when
    // the form was inline. The print path now skips cleanly (no crash, no
    // silent corruption) when /OCProperties or /D is inline; mutation is
    // intentionally not implemented for inline form (rare in practice).
    fn doc_with_inline_ocg() -> (Document, ObjectId) {
        let mut doc = Document::with_version("1.5");
        let mut ocg = Dictionary::new();
        ocg.set(b"Type", Object::Name(b"OCG".to_vec()));
        let mut usage = Dictionary::new();
        let mut view = Dictionary::new();
        view.set(b"ViewState", Object::Name(b"OFF".to_vec()));
        usage.set(b"View", Object::Dictionary(view));
        ocg.set(b"Usage", Object::Dictionary(usage));
        let ocg_id = doc.add_object(Object::Dictionary(ocg));

        // Inline /OCProperties with inline /D (no indirect Reference).
        let mut d = Dictionary::new();
        d.set(b"OFF", Object::Array(vec![Object::Reference(ocg_id)]));
        let mut ocprops = Dictionary::new();
        ocprops.set(b"D", Object::Dictionary(d));

        let mut catalog = Dictionary::new();
        catalog.set(b"Type", Object::Name(b"Catalog".to_vec()));
        // IMPORTANT: inline OCProperties, not Object::Reference.
        catalog.set(b"OCProperties", Object::Dictionary(ocprops));
        let catalog_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set(b"Root", Object::Reference(catalog_id));

        (doc, ocg_id)
    }

    #[test]
    fn suppress_hidden_ocg_print_handles_inline_ocg_gracefully() {
        let (mut doc, ocg_id) = doc_with_inline_ocg();
        // Must not crash, must not panic. Inline form is not mutated today.
        suppress_hidden_ocg_print(&mut doc).unwrap();
        // The doc is still loadable after the no-op pass.
        let _ = doc.get_object(ocg_id).unwrap();
    }

    #[test]
    fn force_show_all_ocg_handles_inline_ocg_gracefully() {
        let (mut doc, ocg_id) = doc_with_inline_ocg();
        force_show_all_ocg(&mut doc).unwrap();
        let _ = doc.get_object(ocg_id).unwrap();
    }
}
