/// Deep-copy `start_page`..=`end_page` and insert the copies immediately before the range.
#[tauri::command]
fn duplicate_page_range_before(path: String, start_page: u32, end_page: u32) -> Result<u32, String> {
    let path_buf = PathBuf::from(&path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let count = end_page - start_page + 1;
    let path_str = path_buf.to_string_lossy().into_owned();
    insert_pdf(path_str.clone(), path_str, start_page, start_page, end_page)?;
    Ok(count)
}
/// Shrink `/MediaBox` inward by viewer-pixel margins on each page in the range.
#[tauri::command]
fn shrink_page_margins(
    path: String,
    start_page: u32,
    end_page: u32,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    pdf::page_margins::shrink_page_margins(
        &PathBuf::from(path),
        start_page,
        end_page,
        margin_top,
        margin_right,
        margin_bottom,
        margin_left,
    )
}
/// Rotate pages 1, 3, 5, … by 90° clockwise.
#[tauri::command]
fn rotate_odd_pages(path: String) -> Result<u32, String> {
    pdf::page_range::rotate_odd_pages(&PathBuf::from(path))
}
/// Rotate pages 2, 4, 6, … by 90° clockwise.
#[tauri::command]
fn rotate_even_pages(path: String) -> Result<u32, String> {
    pdf::page_range::rotate_even_pages(&PathBuf::from(path))
}
/// Delete every `nth` page (1-based: pages n, 2n, 3n, …). Keeps at least one page.
#[tauri::command]
fn delete_every_nth_page(path: String, nth: u32) -> Result<u32, String> {
    pdf::page_range::delete_every_nth_page(&PathBuf::from(&path), nth)
}
/// Move `start_page`..=`end_page` to the beginning of the document.
#[tauri::command]
fn move_page_range_to_start(path: String, start_page: u32, end_page: u32) -> Result<(), String> {
    move_page_range(path, start_page, end_page, 0)
}
/// Move `start_page`..=`end_page` to the end of the document.
#[tauri::command]
fn move_page_range_to_end(path: String, start_page: u32, end_page: u32) -> Result<(), String> {
    pdf::page_range::move_page_range_to_end(&PathBuf::from(&path), start_page, end_page)
}
/// Write odd-indexed pages (1, 3, 5, …) to `output_path` without modifying the source.
#[tauri::command]
fn extract_odd_pages(path: String, output_path: String) -> Result<String, String> {
    pdf::parity_helpers::extract_pages_by_parity(&PathBuf::from(&path), &PathBuf::from(&output_path), true)
}
/// Write even-indexed pages (2, 4, 6, …) to `output_path` without modifying the source.
#[tauri::command]
fn extract_even_pages(path: String, output_path: String) -> Result<String, String> {
    pdf::parity_helpers::extract_pages_by_parity(&PathBuf::from(&path), &PathBuf::from(&output_path), false)
}
/// Deep-copy `page_index` and insert the copy immediately before it.
#[tauri::command]
fn duplicate_page_before(path: String, page_index: u32) -> Result<u32, String> {
    let path_buf = PathBuf::from(&path);
    let page_count = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len();
    let idx = page_index as usize;
    if idx >= page_count {
        return Err("Page index out of bounds".to_string());
    }
    let path_str = path_buf.to_string_lossy().into_owned();
    insert_pdf(path_str.clone(), path_str, page_index, page_index, page_index)?;
    Ok(page_index)
}
/// Split into `_part1.pdf` (pages before `at_page`) and `_part2.pdf` (from `at_page` onward).
#[tauri::command]
fn split_pdf_at_page(path: String, at_page: u32) -> Result<Vec<String>, String> {
    let path = PathBuf::from(&path);
    let doc = Document::load(&path).map_err(|e| e.to_string())?;
    let (all_kids, pages_ref) = get_pages_kids(&doc)?;
    let total = all_kids.len() as u32;
    if total < 2 {
        return Err("Need at least 2 pages to split".to_string());
    }
    if at_page == 0 || at_page >= total {
        return Err(format!("Split page must be between 2 and {total} (1-based start of the second file)"));
    }
    let part1_kids: Vec<Object> = all_kids[..at_page as usize].to_vec();
    let part2_kids: Vec<Object> = all_kids[at_page as usize..].to_vec();
    let stem = path.file_stem().unwrap_or_else(|| std::ffi::OsStr::new("document")).to_string_lossy();
    let mut output_paths = Vec::new();
    for (suffix, kids) in [("_part1", part1_kids), ("_part2", part2_kids)] {
        let mut part = Document::load(&path).map_err(|e| e.to_string())?;
        set_pages_kids(&mut part, pages_ref, kids)?;
        part.prune_objects();
        let output_path = path.with_file_name(format!("{stem}{suffix}.pdf"));
        crate::pdf::io::save_atomic(&mut part, &output_path)?;
        output_paths.push(output_path.to_string_lossy().into_owned());
    }
    Ok(output_paths)
}
/// Rotate pages 1, 3, 5, … by 90° counter-clockwise.
#[tauri::command]
fn rotate_odd_pages_ccw(path: String) -> Result<u32, String> {
    pdf::page_range::rotate_pages_by_parity(&PathBuf::from(&path), true, Some(-90))
}
/// Rotate pages 2, 4, 6, … by 90° counter-clockwise.
#[tauri::command]
fn rotate_even_pages_ccw(path: String) -> Result<u32, String> {
    pdf::page_range::rotate_pages_by_parity(&PathBuf::from(&path), false, Some(-90))
}
/// Clear `/Rotate` on pages 1, 3, 5, ….
#[tauri::command]
fn reset_rotation_odd_pages(path: String) -> Result<u32, String> {
    pdf::page_range::rotate_pages_by_parity(&PathBuf::from(&path), true, None)
}
/// Clear `/Rotate` on pages 2, 4, 6, ….
#[tauri::command]
fn reset_rotation_even_pages(path: String) -> Result<u32, String> {
    pdf::page_range::rotate_pages_by_parity(&PathBuf::from(&path), false, None)
}
/// Delete even-indexed pages; keep pages 1, 3, 5, … only.
#[tauri::command]
fn keep_odd_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::keep_pages_by_parity(&PathBuf::from(&path), true)
}
/// Delete odd-indexed pages; keep pages 2, 4, 6, … only.
#[tauri::command]
fn keep_even_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::keep_pages_by_parity(&PathBuf::from(&path), false)
}
/// Reorder pages by `/Rotate` value (0° first unless `descending` is true).
#[tauri::command]
fn sort_pages_by_rotation(path: String, descending: bool) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    let mut doc = Document::load(&path_buf).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (kids, _) = get_pages_kids(&doc)?;
    let mut indexed: Vec<(usize, i64, Object)> = kids
        .into_iter()
        .enumerate()
        .map(|(i, kid)| {
            let rot = kid.as_reference().ok().map(|id| page_rotation(&doc, id).rem_euclid(360)).unwrap_or(0);
            (i, rot, kid)
        })
        .collect();
    indexed.sort_by(|a, b| {
        let ord = a.1.cmp(&b.1);
        if descending {
            ord.reverse()
        } else {
            ord
        }
    });
    let sorted: Vec<Object> = indexed.into_iter().map(|(_, _, kid)| kid).collect();
    set_pages_kids(&mut doc, pages_ref, sorted)?;
    crate::pdf::io::save_atomic(&mut doc, &path_buf)?;
    Ok(())
}
/// Delete pages 1, 3, 5, … (odd-indexed in 1-based terms).
#[tauri::command]
fn delete_odd_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::delete_pages_by_parity(&PathBuf::from(&path), true)
}
/// Delete pages 2, 4, 6, … (even-indexed in 1-based terms).
#[tauri::command]
fn delete_even_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::delete_pages_by_parity(&PathBuf::from(&path), false)
}
/// Rotate pages 1, 3, 5, … by 180°.
#[tauri::command]
fn rotate_180_odd_pages(path: String) -> Result<u32, String> {
    let path = PathBuf::from(&path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut rotated = 0u32;
    for page_index in 0..total {
        if page_index % 2 != 0 {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let current = page_rotation(&doc, page_id);
        set_page_rotation(&mut doc, page_id, current + 180)?;
        rotated += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(rotated)
}
/// Rotate pages 2, 4, 6, … by 180°.
#[tauri::command]
fn rotate_180_even_pages(path: String) -> Result<u32, String> {
    let path = PathBuf::from(&path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut rotated = 0u32;
    for page_index in 0..total {
        if page_index % 2 != 1 {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let current = page_rotation(&doc, page_id);
        set_page_rotation(&mut doc, page_id, current + 180)?;
        rotated += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(rotated)
}
/// Deep-copy odd-indexed pages and append the copies at the end.
#[tauri::command]
fn duplicate_odd_pages(path: String) -> Result<u32, String> {
    let path_buf = PathBuf::from(&path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    let indices: Vec<u32> = (0..total).filter(|i| i % 2 == 0).collect();
    if indices.is_empty() {
        return Ok(0);
    }
    let path_str = path_buf.to_string_lossy().into_owned();
    let copied = indices.len() as u32;
    for &idx in &indices {
        let at = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
        insert_pdf(path_str.clone(), path_str.clone(), at, idx, idx)?;
    }
    Ok(copied)
}
/// Deep-copy even-indexed pages and append the copies at the end.
#[tauri::command]
fn duplicate_even_pages(path: String) -> Result<u32, String> {
    let path_buf = PathBuf::from(&path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    let indices: Vec<u32> = (0..total).filter(|i| i % 2 == 1).collect();
    if indices.is_empty() {
        return Ok(0);
    }
    let path_str = path_buf.to_string_lossy().into_owned();
    let copied = indices.len() as u32;
    for &idx in &indices {
        let at = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
        insert_pdf(path_str.clone(), path_str.clone(), at, idx, idx)?;
    }
    Ok(copied)
}
/// Insert one blank page between each consecutive pair of pages.
#[tauri::command]
fn insert_blank_between_pages(path: String) -> Result<u32, String> {
    let path_buf = PathBuf::from(&path);
    let mut doc = Document::load(&path_buf).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (kids, _) = get_pages_kids(&doc)?;
    let n = kids.len();
    if n < 2 {
        return Err("Need at least 2 pages to insert blanks between".to_string());
    }
    let mut new_kids = Vec::with_capacity(n + n - 1);
    for (i, kid) in kids.into_iter().enumerate() {
        new_kids.push(kid);
        if i + 1 < n {
            let page_id = create_blank_page(&mut doc, pages_ref);
            new_kids.push(Object::Reference(page_id));
        }
    }
    set_pages_kids(&mut doc, pages_ref, new_kids)?;
    crate::pdf::io::save_atomic(&mut doc, &path_buf)?;
    Ok((n - 1) as u32)
}
/// Remove annotations from odd-indexed pages only.
#[tauri::command]
fn flatten_odd_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::flatten_annotations_by_parity(&PathBuf::from(&path), true)
}
/// Remove annotations from even-indexed pages only.
#[tauri::command]
fn flatten_even_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::flatten_annotations_by_parity(&PathBuf::from(&path), false)
}
/// Rotate every page by 180°.
#[tauri::command]
fn rotate_all_pages_180(path: String) -> Result<u32, String> {
    let path = PathBuf::from(&path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    for page_index in 0..total {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let current = page_rotation(&doc, page_id);
        set_page_rotation(&mut doc, page_id, current + 180)?;
    }
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(total)
}
/// Apply uniform crop margins to odd-indexed pages (1, 3, 5, …).
#[tauri::command]
fn crop_odd_pages(
    path: String,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    pdf::parity_helpers::crop_pages_by_parity(
        &PathBuf::from(&path),
        true,
        margin_top,
        margin_right,
        margin_bottom,
        margin_left,
    )
}
/// Apply uniform crop margins to even-indexed pages (2, 4, 6, …).
#[tauri::command]
fn crop_even_pages(
    path: String,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    pdf::parity_helpers::crop_pages_by_parity(
        &PathBuf::from(&path),
        false,
        margin_top,
        margin_right,
        margin_bottom,
        margin_left,
    )
}
/// Expand MediaBox outward on odd-indexed pages.
#[tauri::command]
fn expand_odd_pages(
    path: String,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    pdf::parity_helpers::expand_pages_by_parity(
        &PathBuf::from(&path),
        true,
        margin_top,
        margin_right,
        margin_bottom,
        margin_left,
    )
}
/// Expand MediaBox outward on even-indexed pages.
#[tauri::command]
fn expand_even_pages(
    path: String,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    pdf::parity_helpers::expand_pages_by_parity(
        &PathBuf::from(&path),
        false,
        margin_top,
        margin_right,
        margin_bottom,
        margin_left,
    )
}
/// Shrink MediaBox inward on odd-indexed pages.
#[tauri::command]
fn shrink_odd_pages(
    path: String,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    pdf::parity_helpers::shrink_pages_by_parity(
        &PathBuf::from(&path),
        true,
        margin_top,
        margin_right,
        margin_bottom,
        margin_left,
    )
}
/// Shrink MediaBox inward on even-indexed pages.
#[tauri::command]
fn shrink_even_pages(
    path: String,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    pdf::parity_helpers::shrink_pages_by_parity(
        &PathBuf::from(&path),
        false,
        margin_top,
        margin_right,
        margin_bottom,
        margin_left,
    )
}
/// Reverse order among odd-indexed pages only.
#[tauri::command]
fn reverse_odd_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::reverse_pages_by_parity(&PathBuf::from(&path), true)
}
/// Reverse order among even-indexed pages only.
#[tauri::command]
fn reverse_even_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::reverse_pages_by_parity(&PathBuf::from(&path), false)
}
/// Move odd-indexed pages to the beginning (even pages follow).
#[tauri::command]
fn move_odd_pages_to_start(path: String) -> Result<(), String> {
    pdf::parity_helpers::move_pages_by_parity_to_start(&PathBuf::from(&path), true)
}
/// Move even-indexed pages to the beginning (odd pages follow).
#[tauri::command]
fn move_even_pages_to_start(path: String) -> Result<(), String> {
    pdf::parity_helpers::move_pages_by_parity_to_start(&PathBuf::from(&path), false)
}
/// Move odd-indexed pages to the end (even pages stay at the start).
#[tauri::command]
fn move_odd_pages_to_end(path: String) -> Result<(), String> {
    pdf::parity_helpers::move_pages_by_parity_to_end(&PathBuf::from(&path), true)
}
/// Move even-indexed pages to the end (odd pages stay at the start).
#[tauri::command]
fn move_even_pages_to_end(path: String) -> Result<(), String> {
    pdf::parity_helpers::move_pages_by_parity_to_end(&PathBuf::from(&path), false)
}
/// Remove `/CropBox` from odd-indexed pages only.
#[tauri::command]
fn clear_crop_odd_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::clear_crop_pages_by_parity(&PathBuf::from(&path), true)
}
/// Remove `/CropBox` from even-indexed pages only.
#[tauri::command]
fn clear_crop_even_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::clear_crop_pages_by_parity(&PathBuf::from(&path), false)
}
/// Deep-copy odd-indexed pages and insert each copy immediately before the original.
#[tauri::command]
fn duplicate_odd_pages_before(path: String) -> Result<u32, String> {
    pdf::parity_helpers::duplicate_pages_by_parity_before(&PathBuf::from(&path), true)
}
/// Deep-copy even-indexed pages and insert each copy immediately before the original.
#[tauri::command]
fn duplicate_even_pages_before(path: String) -> Result<u32, String> {
    pdf::parity_helpers::duplicate_pages_by_parity_before(&PathBuf::from(&path), false)
}
/// Sort odd-indexed pages by `/Rotate` while leaving even pages in place.
#[tauri::command]
fn sort_odd_pages_by_rotation(path: String, descending: bool) -> Result<u32, String> {
    pdf::parity_helpers::sort_pages_by_parity_rotation(&PathBuf::from(&path), true, descending)
}
/// Sort even-indexed pages by `/Rotate` while leaving odd pages in place.
#[tauri::command]
fn sort_even_pages_by_rotation(path: String, descending: bool) -> Result<u32, String> {
    pdf::parity_helpers::sort_pages_by_parity_rotation(&PathBuf::from(&path), false, descending)
}
/// Sort odd-indexed pages by MediaBox area while leaving even pages in place.
#[tauri::command]
fn sort_odd_pages_by_size(path: String, descending: bool) -> Result<u32, String> {
    pdf::parity_helpers::sort_pages_by_parity_size(&PathBuf::from(&path), true, descending)
}
/// Sort even-indexed pages by MediaBox area while leaving odd pages in place.
#[tauri::command]
fn sort_even_pages_by_size(path: String, descending: bool) -> Result<u32, String> {
    pdf::parity_helpers::sort_pages_by_parity_size(&PathBuf::from(&path), false, descending)
}
/// Stamp footer page numbers on odd-indexed pages only (1, 3, 5…).
#[tauri::command]
fn add_page_numbers_odd_pages(path: String, prefix: Option<String>) -> Result<u32, String> {
    pdf::parity_helpers::add_page_numbers_by_parity(&PathBuf::from(&path), true, prefix)
}
/// Stamp footer page numbers on even-indexed pages only (2, 4, 6…).
#[tauri::command]
fn add_page_numbers_even_pages(path: String, prefix: Option<String>) -> Result<u32, String> {
    pdf::parity_helpers::add_page_numbers_by_parity(&PathBuf::from(&path), false, prefix)
}
/// Add a diagonal watermark to odd-indexed pages only.
#[tauri::command]
fn add_text_watermark_odd_pages(path: String, text: String) -> Result<u32, String> {
    pdf::parity_helpers::add_text_watermark_by_parity(&PathBuf::from(&path), true, &text)
}
/// Add a diagonal watermark to even-indexed pages only.
#[tauri::command]
fn add_text_watermark_even_pages(path: String, text: String) -> Result<u32, String> {
    pdf::parity_helpers::add_text_watermark_by_parity(&PathBuf::from(&path), false, &text)
}
/// Stamp header text on odd-indexed pages only.
#[tauri::command]
fn add_page_header_odd_pages(path: String, text: String) -> Result<u32, String> {
    pdf::page_decor::add_page_header_by_parity(&PathBuf::from(&path), true, &text)
}
/// Stamp header text on even-indexed pages only.
#[tauri::command]
fn add_page_header_even_pages(path: String, text: String) -> Result<u32, String> {
    pdf::page_decor::add_page_header_by_parity(&PathBuf::from(&path), false, &text)
}
/// Stamp footer text on odd-indexed pages only.
#[tauri::command]
fn add_page_footer_odd_pages(path: String, text: String) -> Result<u32, String> {
    pdf::page_decor::add_page_footer_by_parity(&PathBuf::from(&path), true, &text)
}
/// Stamp footer text on even-indexed pages only.
#[tauri::command]
fn add_page_footer_even_pages(path: String, text: String) -> Result<u32, String> {
    pdf::page_decor::add_page_footer_by_parity(&PathBuf::from(&path), false, &text)
}
/// Draw a page border on odd-indexed pages only.
#[tauri::command]
fn add_page_border_odd_pages(path: String, inset: f64) -> Result<u32, String> {
    pdf::page_decor::add_page_border_by_parity(&PathBuf::from(&path), true, inset)
}
/// Draw a page border on even-indexed pages only.
#[tauri::command]
fn add_page_border_even_pages(path: String, inset: f64) -> Result<u32, String> {
    pdf::page_decor::add_page_border_by_parity(&PathBuf::from(&path), false, inset)
}
/// Append outline entries for odd-indexed pages only (1, 3, 5…).
#[tauri::command]
fn bookmark_odd_pages(path: String, prefix: Option<String>) -> Result<u32, String> {
    pdf::parity_helpers::bookmark_pages_by_parity(&PathBuf::from(&path), true, prefix)
}
/// Append outline entries for even-indexed pages only (2, 4, 6…).
#[tauri::command]
fn bookmark_even_pages(path: String, prefix: Option<String>) -> Result<u32, String> {
    pdf::parity_helpers::bookmark_pages_by_parity(&PathBuf::from(&path), false, prefix)
}
/// Set MediaBox preset on odd-indexed pages only.
#[tauri::command]
fn set_page_size_odd_pages(path: String, preset: String) -> Result<u32, String> {
    pdf::page_margins::set_page_size_by_parity(&PathBuf::from(&path), true, &preset)
}
/// Set MediaBox preset on even-indexed pages only.
#[tauri::command]
fn set_page_size_even_pages(path: String, preset: String) -> Result<u32, String> {
    pdf::page_margins::set_page_size_by_parity(&PathBuf::from(&path), false, &preset)
}
/// Insert a blank page before each odd-indexed page.
#[tauri::command]
fn insert_blank_before_odd_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::insert_blank_by_parity(&PathBuf::from(&path), true, false)
}
/// Insert a blank page before each even-indexed page.
#[tauri::command]
fn insert_blank_before_even_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::insert_blank_by_parity(&PathBuf::from(&path), false, false)
}
/// Insert a blank page after each odd-indexed page.
#[tauri::command]
fn insert_blank_after_odd_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::insert_blank_by_parity(&PathBuf::from(&path), true, true)
}
/// Insert a blank page after each even-indexed page.
#[tauri::command]
fn insert_blank_after_even_pages(path: String) -> Result<u32, String> {
    pdf::parity_helpers::insert_blank_by_parity(&PathBuf::from(&path), false, true)
}
/// Deep-copy each odd-indexed page and move the copy to the document end.
#[tauri::command]
fn duplicate_odd_pages_to_end(path: String) -> Result<u32, String> {
    pdf::parity_helpers::duplicate_pages_by_parity_to_end(&PathBuf::from(&path), true)
}
/// Deep-copy each even-indexed page and move the copy to the document end.
#[tauri::command]
fn duplicate_even_pages_to_end(path: String) -> Result<u32, String> {
    pdf::parity_helpers::duplicate_pages_by_parity_to_end(&PathBuf::from(&path), false)
}
/// Deep-copy each odd-indexed page and insert the copies at the document start.
#[tauri::command]
fn duplicate_odd_pages_to_start(path: String) -> Result<u32, String> {
    pdf::parity_helpers::duplicate_pages_by_parity_to_start(&PathBuf::from(&path), true)
}
/// Deep-copy each even-indexed page and insert the copies at the document start.
#[tauri::command]
fn duplicate_even_pages_to_start(path: String) -> Result<u32, String> {
    pdf::parity_helpers::duplicate_pages_by_parity_to_start(&PathBuf::from(&path), false)
}
/// Export each odd-indexed page as a separate single-page PDF in `output_dir`.
#[tauri::command]
fn export_odd_pages_as_pdf(path: String, output_dir: String) -> Result<Vec<String>, String> {
    pdf::parity_helpers::export_pages_by_parity_as_pdf(&PathBuf::from(&path), &PathBuf::from(&output_dir), true)
}
/// Export each even-indexed page as a separate single-page PDF in `output_dir`.
#[tauri::command]
fn export_even_pages_as_pdf(path: String, output_dir: String) -> Result<Vec<String>, String> {
    pdf::parity_helpers::export_pages_by_parity_as_pdf(&PathBuf::from(&path), &PathBuf::from(&output_dir), false)
}
