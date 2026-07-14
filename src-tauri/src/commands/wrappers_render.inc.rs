#[tauri::command]
fn get_pdf_page_count(path: String) -> Result<u32, String> {
    let path = PathBuf::from(path);
    let pdfium = get_pdfium()?;
    let document = pdfium.load_pdf_from_file(&path, None).map_err(|e| e.to_string())?;
    Ok(document.pages().len() as u32)
}
/// Return the PDF outline/bookmark tree as a flat, depth-indented list.
#[tauri::command]
fn get_pdf_bookmarks(path: String) -> Result<Vec<PdfBookmarkEntry>, String> {
    let path = PathBuf::from(path);
    let doc = Document::load(&path).map_err(|e| e.to_string())?;
    let catalog = doc.catalog().map_err(|e| e.to_string())?;
    let outlines_id = match catalog.get(b"Outlines") {
        Ok(Object::Reference(id)) => *id,
        _ => return Ok(Vec::new()),
    };
    let outlines = doc.get_dictionary(outlines_id).map_err(|e| e.to_string())?;
    let first_id = match outlines.get(b"First") {
        Ok(Object::Reference(id)) => *id,
        _ => return Ok(Vec::new()),
    };
    let mut entries = Vec::new();
    collect_outline_items(&doc, first_id, 0, &mut entries);
    Ok(entries)
}
/// Read document Info dictionary metadata from a PDF.
#[tauri::command]
fn get_pdf_metadata(path: String) -> Result<PdfDocumentMetadata, String> {
    let path = PathBuf::from(path);
    let doc = Document::load(&path).map_err(|e| e.to_string())?;
    Ok(PdfDocumentMetadata {
        title: read_info_string(&doc, b"Title"),
        author: read_info_string(&doc, b"Author"),
        subject: read_info_string(&doc, b"Subject"),
        keywords: read_info_string(&doc, b"Keywords"),
        creator: read_info_string(&doc, b"Creator"),
        producer: read_info_string(&doc, b"Producer"),
        creation_date: read_info_string(&doc, b"CreationDate"),
        mod_date: read_info_string(&doc, b"ModDate"),
    })
}
/// Update document Info dictionary metadata in the working copy.
#[tauri::command]
fn set_pdf_metadata(
    path: String,
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    keywords: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
) -> Result<(), String> {
    let path = PathBuf::from(path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    if doc.is_encrypted() {
        return Err("Cannot edit metadata on an encrypted PDF".to_string());
    }
    let info_id = ensure_info_dict_id(&mut doc)?;
    let needs_creation_date = read_info_string(&doc, b"CreationDate").is_none();
    let mod_date = current_pdf_mod_date();
    let dict = doc.get_dictionary_mut(info_id).map_err(|e| e.to_string())?;
    write_info_text_field(dict, b"Title", title);
    write_info_text_field(dict, b"Author", author);
    write_info_text_field(dict, b"Subject", subject);
    write_info_text_field(dict, b"Keywords", keywords);
    write_info_text_field(dict, b"Creator", creator);
    write_info_text_field(dict, b"Producer", producer);
    if needs_creation_date {
        dict.set(b"CreationDate", Object::String(mod_date.clone().into_bytes(), lopdf::StringFormat::Literal));
    }
    dict.set(b"ModDate", Object::String(mod_date.into_bytes(), lopdf::StringFormat::Literal));
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(())
}
#[tauri::command]
fn list_pdf_browser_entries(path: Option<String>, include_documents: Option<bool>) -> Result<pdf::browser::PdfBrowserListing, String> {
    let dir = path
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(pdf::browser::default_browser_dir);
    let dir = if dir.is_file() {
        dir.parent().map(Path::to_path_buf).unwrap_or_else(pdf::browser::default_browser_dir)
    } else {
        dir
    };
    if include_documents.unwrap_or(false) {
        pdf::browser::list_entries_for_dir(&dir, true)
    } else {
        pdf::browser::list_pdf_entries_for_dir(&dir)
    }
}
#[tauri::command]
fn render_pdf_page(path: String, page_index: u32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    render_page_image(&PathBuf::from(path), page_index, width, height, image::ImageFormat::Png)
}
#[tauri::command]
fn render_pdf_page_all_layers(path: String, page_index: u32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    let temp_dir = std::env::temp_dir();
    pdf::print::render_pdf_page_with_all_layers_visible(
        &PathBuf::from(path),
        page_index,
        width,
        height,
        &temp_dir,
    )
}
#[tauri::command]
fn ocr_available() -> bool {
    pdf::ocr::ocr_available()
}
#[tauri::command]
fn tesseract_install_guide() -> TesseractInstallGuide {
    build_tesseract_install_guide()
}
#[tauri::command]
fn ocr_status() -> OcrStatus {
    pdf::ocr::ocr_status()
}
/// OCR a single rendered PDF page (for scanned documents without a text layer).
#[tauri::command]
fn ocr_pdf_page(path: String, page: u32) -> Result<String, String> {
    let path = PathBuf::from(path);
    let png = render_page_image(&path, page, OCR_RENDER_W, OCR_RENDER_H, image::ImageFormat::Png)?;
    pdf::ocr::ocr_pdf_page_from_png(&png)
}
/// Find all occurrences of `query` in the PDF using PDFium's text layer.
#[tauri::command]
fn search_pdf_text(
    path: String,
    query: String,
    match_case: bool,
    match_whole_word: bool,
) -> Result<Vec<PdfTextSearchMatch>, String> {
    let pdfium = get_pdfium()?;
    search_pdf_text_impl(&pdfium, &PathBuf::from(path), &query, match_case, match_whole_word)
}
/// Character layout for the viewer text-selection layer (viewer px at 800×1132).
#[tauri::command]
fn get_page_text_layout(path: String, page_index: u32) -> Result<Vec<pdf::text_layer::PageTextRun>, String> {
    let pdfium = get_pdfium()?;
    pdf::text_layer::get_page_text_layout(&pdfium, &PathBuf::from(path), page_index)
}
#[tauri::command]
fn get_pdf_thumbnails(path: String, width: i32, height: i32) -> Result<Vec<Vec<u8>>, String> {
    let path = PathBuf::from(path);
    let pdfium = get_pdfium()?;
    pdf::render::render_pdf_thumbnails(&pdfium, &path, width, height)
}
#[tauri::command]
fn delete_page(path: String, page_index: u32) -> Result<(), String> {
    pdf::page_ops::delete_page(&PathBuf::from(path), page_index)
}
#[tauri::command]
fn move_page(path: String, from_index: u32, to_index: u32) -> Result<(), String> {
    pdf::page_ops::move_page(&PathBuf::from(path), from_index, to_index)
}
#[tauri::command]
fn move_page_between_pdfs(source_path: String, dest_path: String, source_index: u32, dest_index: u32) -> Result<(), String> {
    pdf::page_ops::move_page_between_documents(
        &PathBuf::from(source_path),
        &PathBuf::from(dest_path),
        source_index,
        dest_index,
    )
}
/// Deep-copy `page_index` and insert the copy immediately after it.
#[tauri::command]
fn duplicate_page(path: String, page_index: u32) -> Result<u32, String> {
    pdf::page_ops::duplicate_page(&PathBuf::from(&path), page_index)
}
/// Append pages from `merge_path` to the end of `path`.
#[tauri::command]
fn merge_pdf(path: String, merge_path: String, merge_start: u32, merge_end: u32) -> Result<u32, String> {
    pdf::page_ops::merge_pdf(&PathBuf::from(&path), &PathBuf::from(&merge_path), merge_start, merge_end)
}
#[tauri::command]
fn rotate_page(path: String, page_index: u32) -> Result<(), String> {
    let path = PathBuf::from(path);
    pdf::io::mutate_pdf(&path, |doc| {
        rotate_page_at(doc, page_index, 90)?;
        Ok(())
    })
}
/// Rotate every page in the document 90° clockwise.
#[tauri::command]
fn rotate_all_pages(path: String) -> Result<u32, String> {
    let path = PathBuf::from(path);
    pdf::io::mutate_pdf(&path, |doc| rotate_all_pages_by(doc, 90))
}
/// Reverse the document page order.
#[tauri::command]
fn reverse_pages(path: String) -> Result<(), String> {
    pdf::page_ops::reverse_pages(&PathBuf::from(path))
}
/// Insert a blank page before `at_index` (0 = first page).
#[tauri::command]
fn add_blank_page(path: String, at_index: u32) -> Result<u32, String> {
    pdf::page_ops::add_blank_page(&PathBuf::from(path), at_index)
}
/// Delete `start_page`..=`end_page` (inclusive, 0-based). At least one page must remain.
#[tauri::command]
fn delete_page_range(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    pdf::page_ops::delete_page_range(&PathBuf::from(path), start_page, end_page)
}
/// Append a bookmark pointing at `page_index`.
#[tauri::command]
fn add_pdf_bookmark(path: String, title: String, page_index: u32) -> Result<(), String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("Bookmark title cannot be empty".to_string());
    }
    let path = PathBuf::from(path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    append_outline_item(&mut doc, title, page_id)?;
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(())
}
/// Stamp page numbers on the footer of each page in the range (1-based labels).
#[tauri::command]
fn add_page_numbers(path: String, start_page: u32, end_page: u32, prefix: Option<String>) -> Result<u32, String> {
    pdf::page_decor::add_page_numbers(&PathBuf::from(path), start_page, end_page, prefix)
}
/// Stamp Bates numbers (prefix + zero-padded counter) on each page in the range.
#[tauri::command]
fn add_bates_numbers(
    path: String,
    start_page: u32,
    end_page: u32,
    prefix: String,
    start_number: u64,
    digits: u32,
    position: String,
) -> Result<(), String> {
    pdf::page_decor::add_bates_numbers(
        &PathBuf::from(path),
        start_page,
        end_page,
        &prefix,
        start_number,
        digits as usize,
        &position,
    )
}
/// Add a diagonal text watermark to each page in the range.
#[tauri::command]
fn add_text_watermark(path: String, text: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    pdf::page_decor::add_text_watermark(&PathBuf::from(path), &text, start_page, end_page)
}
/// Remove all annotation dictionaries from pages in the range (flatten markup).
#[tauri::command]
fn flatten_annotations(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    pdf::page_decor::flatten_annotations(&PathBuf::from(path), start_page, end_page)
}
/// Crop `page_index` by viewer-pixel margins (top/right/bottom/left).
#[tauri::command]
fn crop_page(
    path: String,
    page_index: u32,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<(), String> {
    pdf::crop::crop_page(&PathBuf::from(path), page_index, margin_top, margin_right, margin_bottom, margin_left)
}
/// Rotate `page_index` 90° counter-clockwise.
#[tauri::command]
fn rotate_page_ccw(path: String, page_index: u32) -> Result<(), String> {
    let path = PathBuf::from(path);
    pdf::io::mutate_pdf(&path, |doc| {
        rotate_page_at(doc, page_index, 270)?;
        Ok(())
    })
}
/// Clear rotation on `page_index`.
#[tauri::command]
fn reset_page_rotation(path: String, page_index: u32) -> Result<(), String> {
    let path = PathBuf::from(path);
    pdf::io::mutate_pdf(&path, |doc| {
        reset_page_rotation_at(doc, page_index)?;
        Ok(())
    })
}
/// Clear rotation on every page.
#[tauri::command]
fn reset_all_page_rotations(path: String) -> Result<u32, String> {
    let path = PathBuf::from(path);
    pdf::io::mutate_pdf(&path, pdf::rotation::reset_all_page_rotations)
}
/// Deep-copy `start_page`..=`end_page` and insert the copies immediately after the range.
#[tauri::command]
fn duplicate_page_range(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    pdf::page_ops::duplicate_page_range(&PathBuf::from(&path), start_page, end_page)
}
/// Remove a bookmark by flat index (same order as `get_pdf_bookmarks`).
#[tauri::command]
fn remove_pdf_bookmark(path: String, bookmark_index: u32) -> Result<(), String> {
    let path = PathBuf::from(path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let catalog = doc.catalog().map_err(|e| e.to_string())?;
    let outlines_id = catalog
        .get(b"Outlines")
        .map_err(|_| "No bookmarks in this PDF".to_string())?
        .as_reference()
        .map_err(|_| "Bad Outlines".to_string())?;
    let ids = flat_outline_ids(&doc)?;
    let idx = bookmark_index as usize;
    if idx >= ids.len() {
        return Err("Bookmark index out of bounds".to_string());
    }
    remove_outline_item(&mut doc, outlines_id, ids[idx])?;
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(())
}
/// Rename a bookmark by flat index (same order as `get_pdf_bookmarks`).
#[tauri::command]
fn rename_pdf_bookmark(path: String, bookmark_index: u32, title: String) -> Result<(), String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("Bookmark title cannot be empty".to_string());
    }
    let path = PathBuf::from(path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let ids = flat_outline_ids(&doc)?;
    let idx = bookmark_index as usize;
    if idx >= ids.len() {
        return Err("Bookmark index out of bounds".to_string());
    }
    doc.get_dictionary_mut(ids[idx])
        .map_err(|e| e.to_string())?
        .set("Title", Object::String(title.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(())
}
/// Return MediaBox width/height (points) and rotation for every page.
#[tauri::command]
fn get_pdf_page_sizes(path: String) -> Result<Vec<PdfPageSize>, String> {
    pdf::page_sizes::get_pdf_page_sizes(&PathBuf::from(path))
}
/// Remove `/CropBox` from `page_index`.
#[tauri::command]
fn clear_page_crop(path: String, page_index: u32) -> Result<(), String> {
    pdf::crop::clear_page_crop(&PathBuf::from(path), page_index)
}
/// Apply the same viewer-pixel crop margins to every page.
#[tauri::command]
fn crop_all_pages(
    path: String,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    pdf::crop::crop_all_pages(&PathBuf::from(path), margin_top, margin_right, margin_bottom, margin_left)
}
/// Rotate `page_index` 180°.
#[tauri::command]
fn rotate_page_180(path: String, page_index: u32) -> Result<(), String> {
    let path = PathBuf::from(path);
    pdf::io::mutate_pdf(&path, |doc| {
        rotate_page_at(doc, page_index, 180)?;
        Ok(())
    })
}
/// Rotate every page 90° counter-clockwise.
#[tauri::command]
fn rotate_all_pages_ccw(path: String) -> Result<u32, String> {
    let path = PathBuf::from(path);
    pdf::io::mutate_pdf(&path, |doc| rotate_all_pages_by(doc, 270))
}
/// Move `page_index` to the first position.
#[tauri::command]
fn move_page_to_first(path: String, page_index: u32) -> Result<(), String> {
    pdf::page_ops::move_page_to_first(&PathBuf::from(path), page_index)
}
/// Move `page_index` to the last position.
#[tauri::command]
fn move_page_to_last(path: String, page_index: u32) -> Result<(), String> {
    pdf::page_ops::move_page_to_last(&PathBuf::from(path), page_index)
}
/// Remove `/CropBox` from every page.
#[tauri::command]
fn clear_all_page_crops(path: String) -> Result<u32, String> {
    pdf::crop::clear_all_page_crops(&PathBuf::from(path))
}
/// Remove every PDF outline/bookmark entry.
#[tauri::command]
fn clear_pdf_bookmarks(path: String) -> Result<u32, String> {
    let path = PathBuf::from(path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let catalog_id = doc.trailer.get(b"Root").map_err(|e| e.to_string())?.as_reference().map_err(|_| "Bad Root")?;
    let outlines_id = match doc.get_dictionary(catalog_id).map_err(|e| e.to_string())?.get(b"Outlines") {
        Ok(Object::Reference(id)) => *id,
        _ => return Ok(0),
    };
    let ids = flat_outline_ids(&doc)?;
    let count = ids.len() as u32;
    for id in ids {
        doc.objects.remove(&id);
    }
    doc.objects.remove(&outlines_id);
    doc.get_dictionary_mut(catalog_id).map_err(|e| e.to_string())?.remove(b"Outlines");
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(count)
}
/// Stamp header text near the top of each page in the range.
#[tauri::command]
fn add_page_header(path: String, start_page: u32, end_page: u32, text: String) -> Result<u32, String> {
    pdf::page_decor::add_page_header(&PathBuf::from(path), start_page, end_page, &text)
}
/// Stamp footer text near the bottom of each page in the range.
#[tauri::command]
fn add_page_footer(path: String, start_page: u32, end_page: u32, text: String) -> Result<u32, String> {
    pdf::page_decor::add_page_footer(&PathBuf::from(&path), start_page, end_page, &text)
}
/// Swap two pages by 0-based index.
#[tauri::command]
fn swap_pages(path: String, page_index_a: u32, page_index_b: u32) -> Result<(), String> {
    if page_index_a == page_index_b {
        return Ok(());
    }
    let path_buf = PathBuf::from(&path);
    let mut doc = Document::load(&path_buf).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let a = page_index_a as usize;
    let b = page_index_b as usize;
    if a >= kids.len() || b >= kids.len() {
        return Err("Page index out of bounds".to_string());
    }
    kids.swap(a, b);
    set_pages_kids(&mut doc, pages_ref, kids)?;
    crate::pdf::io::save_atomic(&mut doc, &path_buf)?;
    Ok(())
}
/// Move `page_index` one position earlier (toward the first page).
#[tauri::command]
fn move_page_up(path: String, page_index: u32) -> Result<(), String> {
    if page_index == 0 {
        return Err("Page is already first".to_string());
    }
    move_page(path, page_index, page_index - 1)
}
/// Move `page_index` one position later (toward the last page).
#[tauri::command]
fn move_page_down(path: String, page_index: u32) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    let last = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if last == 0 {
        return Err("Document has no pages".to_string());
    }
    if page_index + 1 >= last {
        return Err("Page is already last".to_string());
    }
    move_page(path, page_index, page_index + 1)
}
/// Replace `page_index` with a deep-copied page from `source_path`.
#[tauri::command]
fn replace_page(path: String, page_index: u32, source_path: String, source_page_index: u32) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    let source_path_buf = PathBuf::from(&source_path);
    let mut doc = Document::load(&path_buf).map_err(|e| e.to_string())?;
    let source_doc = Document::load(&source_path_buf).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let idx = page_index as usize;
    if idx >= kids.len() {
        return Err("Page index out of bounds".to_string());
    }
    let source_pages: Vec<ObjectId> = source_doc.get_pages().into_values().collect();
    let src_idx = source_page_index as usize;
    if src_idx >= source_pages.len() {
        return Err("Source page index out of bounds".to_string());
    }
    let mut remap = BTreeMap::new();
    let new_page_id = import_object(&mut doc, &source_doc, source_pages[src_idx], pages_ref, &mut remap);
    kids[idx] = Object::Reference(new_page_id);
    set_pages_kids(&mut doc, pages_ref, kids)?;
    merge_acroform_after_insert(&mut doc, &source_doc, &[new_page_id], &remap)?;
    dedup_fonts_after_insert(&mut doc, &[new_page_id])?;
    doc.prune_objects();
    crate::pdf::io::save_atomic(&mut doc, &path_buf)?;
    Ok(())
}
/// Interleave pages from `other_path` after each page of `path` (A0, B0, A1, B1, …).
#[tauri::command]
fn interleave_pdf(path: String, other_path: String, other_start: u32, other_end: u32) -> Result<u32, String> {
    let path_buf = PathBuf::from(&path);
    let other_path_buf = PathBuf::from(&other_path);
    if path_buf == other_path_buf {
        return Err("Cannot interleave a PDF with itself".to_string());
    }
    let mut doc = Document::load(&path_buf).map_err(|e| e.to_string())?;
    let other_doc = Document::load(&other_path_buf).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (dest_kids, _) = get_pages_kids(&doc)?;
    let other_pages: Vec<ObjectId> = other_doc.get_pages().into_values().collect();
    let start = other_start as usize;
    let end = other_end as usize;
    if start > end || end >= other_pages.len() {
        return Err("Invalid interleave page range".to_string());
    }
    let mut remap = BTreeMap::new();
    let other_imported: Vec<ObjectId> = other_pages[start..=end]
        .iter()
        .map(|&src_page| import_object(&mut doc, &other_doc, src_page, pages_ref, &mut remap))
        .collect();
    let dest_len = dest_kids.len();
    let other_len = other_imported.len();
    let max_len = dest_len.max(other_len);
    let mut new_kids = Vec::with_capacity(dest_len + other_len);
    for i in 0..max_len {
        if i < dest_len {
            new_kids.push(dest_kids[i].clone());
        }
        if i < other_len {
            new_kids.push(Object::Reference(other_imported[i]));
        }
    }
    set_pages_kids(&mut doc, pages_ref, new_kids)?;
    merge_acroform_after_insert(&mut doc, &other_doc, &other_imported, &remap)?;
    dedup_fonts_after_insert(&mut doc, &other_imported)?;
    crate::pdf::io::save_atomic(&mut doc, &path_buf)?;
    Ok(other_len as u32)
}
/// Split the document into odd-indexed and even-indexed page files.
#[tauri::command]
fn split_odd_even_pages(path: String) -> Result<Vec<String>, String> {
    let path = PathBuf::from(&path);
    let doc = Document::load(&path).map_err(|e| e.to_string())?;
    let (all_kids, pages_ref) = get_pages_kids(&doc)?;
    if all_kids.len() < 2 {
        return Err("Need at least 2 pages to split odd/even".to_string());
    }
    let odd_kids: Vec<Object> =
        all_kids.iter().enumerate().filter(|(i, _)| i % 2 == 0).map(|(_, k)| k.clone()).collect();
    let even_kids: Vec<Object> =
        all_kids.iter().enumerate().filter(|(i, _)| i % 2 == 1).map(|(_, k)| k.clone()).collect();
    let stem = path.file_stem().unwrap_or_else(|| std::ffi::OsStr::new("document")).to_string_lossy();
    let mut output_paths = Vec::new();
    for (suffix, kids) in [("_odd", odd_kids), ("_even", even_kids)] {
        let mut part = Document::load(&path).map_err(|e| e.to_string())?;
        set_pages_kids(&mut part, pages_ref, kids)?;
        part.prune_objects();
        let output_path = path.with_file_name(format!("{stem}{suffix}.pdf"));
        crate::pdf::io::save_atomic(&mut part, &output_path)?;
        output_paths.push(output_path.to_string_lossy().into_owned());
    }
    Ok(output_paths)
}
/// Deep-copy every page and append the copies to the end of the document.
#[tauri::command]
fn duplicate_all_pages(path: String) -> Result<u32, String> {
    let path_buf = PathBuf::from(&path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if total == 0 {
        return Err("Document has no pages".to_string());
    }
    duplicate_page_range(path, 0, total - 1)
}
/// Set `/MediaBox` on each page in the range to a standard paper size (points).
#[tauri::command]
fn set_page_size(path: String, start_page: u32, end_page: u32, preset: String) -> Result<u32, String> {
    pdf::page_margins::set_page_size(&PathBuf::from(&path), start_page, end_page, &preset)
}
/// Write a decrypted sibling `<stem>_decrypted.pdf` next to an encrypted `path`.
#[tauri::command]
fn remove_pdf_password(path: String, password: String) -> Result<String, String> {
    pdf::security::remove_pdf_password(path, password)
}
/// Export each page in the range as a separate single-page PDF in `output_dir`.
#[tauri::command]
fn export_pdf_pages_as_pdf(
    path: String,
    start_page: u32,
    end_page: u32,
    output_dir: String,
) -> Result<Vec<String>, String> {
    let path = PathBuf::from(&path);
    if !path.is_file() {
        return Err("File not found".to_string());
    }
    validate_page_range(&path, start_page, end_page)?;
    let output_dir = PathBuf::from(&output_dir);
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    let mut written = Vec::new();
    for page_index in start_page..=end_page {
        let file_name = format!("page-{:03}.pdf", page_index + 1);
        let output_path = output_dir.join(&file_name);
        let out = extract_pdf_pages(
            path.to_string_lossy().into_owned(),
            output_path.to_string_lossy().into_owned(),
            page_index,
            page_index,
        )?;
        written.push(out);
    }
    Ok(written)
}
/// Rotate every page in `start_page`..=`end_page` 90° clockwise.
#[tauri::command]
fn rotate_page_range(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    let path = PathBuf::from(&path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let mut rotated = 0u32;
    for page_index in start_page..=end_page {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let current = page_rotation(&doc, page_id);
        set_page_rotation(&mut doc, page_id, current + 90)?;
        rotated += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(rotated)
}
/// Rotate every page in `start_page`..=`end_page` 90° counter-clockwise.
#[tauri::command]
fn rotate_page_range_ccw(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    let path = PathBuf::from(&path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let mut rotated = 0u32;
    for page_index in start_page..=end_page {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let current = page_rotation(&doc, page_id);
        set_page_rotation(&mut doc, page_id, current - 90)?;
        rotated += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(rotated)
}
/// Delete every page outside `start_page`..=`end_page` (at least one page must remain).
#[tauri::command]
fn keep_page_range(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    let path = PathBuf::from(&path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (kids, _) = get_pages_kids(&doc)?;
    let total = kids.len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let kept: Vec<Object> = kids[start_page as usize..=end_page as usize].to_vec();
    if kept.is_empty() {
        return Err("Cannot delete every page in the document".to_string());
    }
    let deleted = total - kept.len() as u32;
    set_pages_kids(&mut doc, pages_ref, kept)?;
    doc.prune_objects();
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(deleted)
}
/// Move `start_page`..=`end_page` so the first page lands at `to_index`.
#[tauri::command]
fn move_page_range(path: String, start_page: u32, end_page: u32, to_index: u32) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    let mut doc = Document::load(&path_buf).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let start = start_page as usize;
    let end = end_page as usize;
    if start > end || end >= kids.len() {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let to = to_index as usize;
    if to > kids.len() {
        return Err("Target index out of bounds".to_string());
    }
    let segment: Vec<Object> = kids.drain(start..=end).collect();
    let insert_at = to.min(kids.len());
    for (offset, kid) in segment.into_iter().enumerate() {
        kids.insert(insert_at + offset, kid);
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    crate::pdf::io::save_atomic(&mut doc, &path_buf)?;
    Ok(())
}
/// Insert pages from `source_path` at the beginning of `path`.
#[tauri::command]
fn prepend_pdf(path: String, source_path: String, source_start: u32, source_end: u32) -> Result<u32, String> {
    let path_buf = PathBuf::from(&path);
    let source_path_buf = PathBuf::from(&source_path);
    if path_buf == source_path_buf {
        return Err("Cannot prepend a PDF into itself".to_string());
    }
    let source_count = Document::load(&source_path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if source_count == 0 {
        return Err("Source PDF has no pages".to_string());
    }
    if source_start > source_end || source_end >= source_count {
        return Err("Invalid source page range".to_string());
    }
    insert_pdf(path, source_path, 0, source_start, source_end)?;
    Ok(source_end - source_start + 1)
}
/// Split the document into consecutive files with at most `pages_per_file` pages each.
#[tauri::command]
fn split_every_n_pages(path: String, pages_per_file: u32) -> Result<Vec<String>, String> {
    if pages_per_file == 0 {
        return Err("Pages per file must be at least 1".to_string());
    }
    let path_buf = PathBuf::from(&path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if total == 0 {
        return Err("Document has no pages".to_string());
    }
    let mut ranges = Vec::new();
    let mut start = 0u32;
    while start < total {
        let end = (start + pages_per_file - 1).min(total - 1);
        ranges.push((start, end));
        start = end + 1;
    }
    split_pdf(path, ranges)
}
/// Draw a rectangular border inset on each page in the range (viewer pixels).
#[tauri::command]
fn add_page_border(path: String, start_page: u32, end_page: u32, inset: f64) -> Result<u32, String> {
    pdf::page_decor::add_page_border(&PathBuf::from(path), start_page, end_page, inset)
}
/// Append a bookmark for every page using `prefix` + page number.
#[tauri::command]
fn bookmark_all_pages(path: String, prefix: Option<String>) -> Result<u32, String> {
    let prefix = prefix.unwrap_or_else(|| "Page ".to_string());
    let path = PathBuf::from(&path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();
    for (index, page_id) in page_ids.iter().enumerate() {
        let title = format!("{prefix}{}", index + 1);
        append_outline_item(&mut doc, &title, *page_id)?;
    }
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(page_ids.len() as u32)
}
/// Deep-copy `page_index` and move the copy to the last position.
#[tauri::command]
fn duplicate_page_to_end(path: String, page_index: u32) -> Result<u32, String> {
    let new_index = duplicate_page(path.clone(), page_index)?;
    move_page_to_last(path.clone(), new_index)?;
    let path_buf = PathBuf::from(&path);
    Ok(Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32 - 1)
}
/// Expand `/MediaBox` outward by viewer-pixel margins on each page in the range.
#[tauri::command]
fn expand_page_margins(
    path: String,
    start_page: u32,
    end_page: u32,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    pdf::page_margins::expand_page_margins(
        &PathBuf::from(path),
        start_page,
        end_page,
        margin_top,
        margin_right,
        margin_bottom,
        margin_left,
    )
}
/// Clear `/Rotate` on every page in `start_page`..=`end_page`.
#[tauri::command]
fn reset_rotation_range(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    pdf::page_range::reset_rotation_range(&PathBuf::from(path), start_page, end_page)
}
/// Rotate every page in `start_page`..=`end_page` by 180°.
#[tauri::command]
fn rotate_page_180_range(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    pdf::page_range::rotate_page_180_range(&PathBuf::from(path), start_page, end_page)
}
/// Reverse page order within `start_page`..=`end_page` only.
#[tauri::command]
fn reverse_page_range(path: String, start_page: u32, end_page: u32) -> Result<(), String> {
    pdf::page_range::reverse_page_range(&PathBuf::from(&path), start_page, end_page)
}
/// Deep-copy `start_page`..=`end_page` and append the copies at the end of the document.
#[tauri::command]
fn duplicate_page_range_to_end(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    let path_buf = PathBuf::from(&path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let path_str = path_buf.to_string_lossy().into_owned();
    insert_pdf(path_str.clone(), path_str, total, start_page, end_page)?;
    Ok(end_page - start_page + 1)
}
/// Insert `count` blank pages starting at `at_index`.
#[tauri::command]
fn insert_blank_pages(path: String, at_index: u32, count: u32) -> Result<u32, String> {
    pdf::page_range::insert_blank_pages(&PathBuf::from(path), at_index, count)
}
/// Apply the same viewer-pixel crop margins to `start_page`..=`end_page`.
#[tauri::command]
fn crop_page_range(
    path: String,
    start_page: u32,
    end_page: u32,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    pdf::page_range::crop_page_range(
        &PathBuf::from(path),
        start_page,
        end_page,
        margin_top,
        margin_right,
        margin_bottom,
        margin_left,
    )
}
/// Remove all annotation dictionaries from every page in the document.
#[tauri::command]
fn flatten_all_annotations(path: String) -> Result<u32, String> {
    let path_buf = PathBuf::from(&path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if total == 0 {
        return Ok(0);
    }
    flatten_annotations(path, 0, total - 1)
}
/// Remove document Info/XMP metadata from the working copy.
#[tauri::command]
fn clear_pdf_metadata(path: String) -> Result<(), String> {
    let path = PathBuf::from(&path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    if doc.is_encrypted() {
        return Err("Cannot clear metadata on an encrypted PDF".to_string());
    }
    if let Ok(catalog) = doc.catalog_mut() {
        catalog.set(b"Metadata", Object::Null);
    }
    if let Ok(info) = doc.trailer.get_mut(b"Info") {
        *info = Object::Null;
    }
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(())
}
/// Reorder pages by MediaBox area (smallest first unless `descending` is true).
#[tauri::command]
fn sort_pages_by_size(path: String, descending: bool) -> Result<(), String> {
    pdf::page_range::sort_pages_by_size(&PathBuf::from(&path), descending)
}
