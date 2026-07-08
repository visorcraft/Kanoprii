#!/usr/bin/env python3
"""Generate parity-in-range + local-parity + ICO export commands for Kanoprii."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INC = ROOT / "src-tauri" / "src" / "parity_batch_generated.inc.rs"
MAIN = ROOT / "src-tauri" / "src" / "main.rs"

# (base_name, kind) - generates {base_name}_odd and {base_name}_even unless kind ends with _ico
GLOBAL_RANGE = [
    ("rotate_pages_in_range", "rotate_cw"),
    ("rotate_pages_in_range_ccw", "rotate_ccw"),
    ("rotate_180_pages_in_range", "rotate_180"),
    ("reset_rotation_pages_in_range", "reset_rot"),
    ("delete_pages_in_range", "delete"),
    ("keep_pages_in_range", "keep"),
    ("duplicate_pages_in_range", "dup_append"),
    ("duplicate_pages_in_range_before", "dup_before"),
    ("duplicate_pages_in_range_to_start", "dup_to_start"),
    ("duplicate_pages_in_range_to_end", "dup_to_end"),
    ("flatten_pages_in_range", "flatten"),
    ("reverse_pages_in_range", "reverse"),
    ("move_odd_pages_in_range_to_start", "move_odd_start"),
    ("move_even_pages_in_range_to_start", "move_even_start"),
    ("move_odd_pages_in_range_to_end", "move_odd_end"),
    ("move_even_pages_in_range_to_end", "move_even_end"),
    ("sort_pages_in_range_by_rotation", "sort_rot"),
    ("sort_pages_in_range_by_size", "sort_size"),
    ("crop_pages_in_range", "crop"),
    ("expand_pages_in_range", "expand"),
    ("shrink_pages_in_range", "shrink"),
    ("clear_crop_pages_in_range", "clear_crop"),
    ("insert_blank_before_pages_in_range", "blank_before"),
    ("insert_blank_after_pages_in_range", "blank_after"),
    ("bookmark_pages_in_range", "bookmark"),
    ("set_page_size_pages_in_range", "page_size"),
    ("extract_pages_in_range", "extract"),
    ("add_page_numbers_pages_in_range", "page_numbers"),
    ("add_text_watermark_pages_in_range", "watermark"),
    ("add_page_header_pages_in_range", "header"),
    ("add_page_footer_pages_in_range", "footer"),
    ("add_page_border_pages_in_range", "border"),
    ("export_pages_in_range_as_pdf", "export_pdf"),
    ("export_pages_in_range_png", "export_png"),
    ("export_pages_in_range_jpeg", "export_jpeg"),
    ("export_pages_in_range_webp", "export_webp"),
    ("export_pages_in_range_bmp", "export_bmp"),
    ("export_pages_in_range_tiff", "export_tiff"),
    ("export_pages_in_range_gif", "export_gif"),
    ("export_pages_in_range_ppm", "export_ppm"),
    ("export_pages_in_range_tga", "export_tga"),
    ("export_pages_in_range_ico", "export_ico"),
]

LOCAL_RANGE = [
    ("rotate_range_local_pages", "rotate_cw"),
    ("rotate_range_local_pages_ccw", "rotate_ccw"),
    ("rotate_180_range_local_pages", "rotate_180"),
    ("reset_rotation_range_local_pages", "reset_rot"),
    ("delete_range_local_pages", "delete"),
    ("keep_range_local_pages", "keep"),
    ("duplicate_range_local_pages", "dup_append"),
    ("duplicate_range_local_pages_before", "dup_before"),
    ("flatten_range_local_pages", "flatten"),
]

DOC_ICO = [
    ("export_odd_pages_ico", "export_ico_doc", True),
    ("export_even_pages_ico", "export_ico_doc", False),
]


def cmd_names() -> list[str]:
    names: list[str] = []
    for base, kind in GLOBAL_RANGE:
        names.append(f"{base.replace('_pages', '_odd_pages')}")
        names.append(f"{base.replace('_pages', '_even_pages')}")
    for base, kind in LOCAL_RANGE:
        names.append(f"{base.replace('_pages', '_odd_pages')}")
        names.append(f"{base.replace('_pages', '_even_pages')}")
    for name, _, _ in DOC_ICO:
        names.append(name)
    return names


def rust_helpers() -> str:
    return r"""
#[allow(clippy::manual_is_multiple_of)]
fn parity_batch_match(page_index: u32, start_page: u32, end_page: u32, odd: bool, local: bool) -> bool {
    if page_index < start_page || page_index > end_page {
        return false;
    }
    let even_indexed = if local {
        (page_index - start_page).is_multiple_of(2)
    } else {
        page_index.is_multiple_of(2)
    };
    even_indexed == odd
}

fn parity_batch_indices(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool) -> Result<Vec<u32>, String> {
    validate_page_range(path, start_page, end_page)?;
    Ok((start_page..=end_page)
        .filter(|i| parity_batch_match(*i, start_page, end_page, odd, local))
        .collect())
}

fn parity_batch_rotate(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, delta: i64) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut rotated = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let current = page_rotation(&doc, page_id);
        set_page_rotation(&mut doc, page_id, current + delta)?;
        rotated += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(rotated)
}

fn parity_batch_reset_rotation(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut reset = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        set_page_rotation(&mut doc, page_id, 0)?;
        reset += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(reset)
}

fn parity_batch_delete(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let mut to_delete: Vec<usize> = (start_page as usize..=end_page as usize)
        .filter(|i| parity_batch_match(*i as u32, start_page, end_page, odd, local))
        .collect();
    if to_delete.is_empty() {
        return Ok(0);
    }
    if kids.len() - to_delete.len() < 1 {
        return Err("Cannot delete every page in the document".to_string());
    }
    to_delete.sort_by(|a, b| b.cmp(a));
    let deleted = to_delete.len() as u32;
    for idx in to_delete {
        kids.remove(idx);
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    doc.prune_objects();
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(deleted)
}

fn parity_batch_keep(path: &Path, start_page: u32, end_page: u32, keep_odd: bool, local: bool) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (kids, _) = get_pages_kids(&doc)?;
    let kept: Vec<Object> = kids
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            let idx = *i as u32;
            if idx < start_page || idx > end_page {
                return true;
            }
            parity_batch_match(idx, start_page, end_page, keep_odd, local)
        })
        .map(|(_, kid)| kid.clone())
        .collect();
    if kept.is_empty() {
        return Err("No pages would remain after keep filter".to_string());
    }
    if kept.len() == kids.len() {
        return Err("Nothing to delete - all pages in range match the keep filter".to_string());
    }
    let deleted = (kids.len() - kept.len()) as u32;
    set_pages_kids(&mut doc, pages_ref, kept)?;
    doc.prune_objects();
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(deleted)
}

fn parity_batch_dup_append(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool) -> Result<u32, String> {
    let indices = parity_batch_indices(path, start_page, end_page, odd, local)?;
    let path_str = path.to_string_lossy().into_owned();
    for &idx in &indices {
        duplicate_page(path_str.clone(), idx)?;
    }
    Ok(indices.len() as u32)
}

fn parity_batch_dup_before(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool) -> Result<u32, String> {
    let indices = parity_batch_indices(path, start_page, end_page, odd, local)?;
    let path_str = path.to_string_lossy().into_owned();
    for &idx in indices.iter().rev() {
        insert_pdf(path_str.clone(), path_str.clone(), idx, idx, idx)?;
    }
    Ok(indices.len() as u32)
}

fn parity_batch_dup_to_start(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool) -> Result<u32, String> {
    let indices = parity_batch_indices(path, start_page, end_page, odd, local)?;
    let path_str = path.to_string_lossy().into_owned();
    for (offset, &idx) in indices.iter().rev().enumerate() {
        let offset = offset as u32;
        let source = idx + offset;
        insert_pdf(path_str.clone(), path_str.clone(), offset, source, source)?;
    }
    Ok(indices.len() as u32)
}

fn parity_batch_dup_to_end(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool) -> Result<u32, String> {
    let indices = parity_batch_indices(path, start_page, end_page, odd, local)?;
    let path_str = path.to_string_lossy().into_owned();
    for &idx in indices.iter().rev() {
        let new_idx = duplicate_page(path_str.clone(), idx)?;
        move_page_to_last(path_str.clone(), new_idx)?;
    }
    Ok(indices.len() as u32)
}

fn parity_batch_flatten(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut removed = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let count = doc
            .get_dictionary(page_id)
            .ok()
            .and_then(|d| d.get(b"Annots").ok())
            .and_then(|o| o.as_array().ok())
            .map(|a| a.len() as u32)
            .unwrap_or(0);
        if count > 0 {
            doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.remove(b"Annots");
            removed += count;
        }
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(removed)
}

fn parity_batch_reverse(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let parity_indices: Vec<usize> = (start_page as usize..=end_page as usize)
        .filter(|i| parity_batch_match(*i as u32, start_page, end_page, odd, local))
        .collect();
    if parity_indices.len() < 2 {
        return Ok(parity_indices.len() as u32);
    }
    let mut parity_kids: Vec<Object> = parity_indices.iter().map(|i| kids[*i].clone()).collect();
    parity_kids.reverse();
    for (pos, idx) in parity_indices.iter().enumerate() {
        kids[*idx] = parity_kids[pos].clone();
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(parity_indices.len() as u32)
}

#[allow(clippy::manual_is_multiple_of)]
fn parity_batch_move_seg(path: &Path, start_page: u32, end_page: u32, odd_first: bool, local: bool) -> Result<(), String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let start = start_page as usize;
    let end = end_page as usize;
    let segment: Vec<Object> = kids.drain(start..=end).collect();
    let mut odd_kids = Vec::new();
    let mut even_kids = Vec::new();
    for (i, kid) in segment.into_iter().enumerate() {
        let global_idx = start + i;
        let is_odd = if local {
            (i as u32).is_multiple_of(2)
        } else {
            global_idx.is_multiple_of(2)
        };
        if is_odd {
            odd_kids.push(kid);
        } else {
            even_kids.push(kid);
        }
    }
    let reordered: Vec<Object> = if odd_first {
        odd_kids.into_iter().chain(even_kids).collect()
    } else {
        even_kids.into_iter().chain(odd_kids).collect()
    };
    for (offset, kid) in reordered.into_iter().enumerate() {
        kids.insert(start + offset, kid);
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn parity_batch_sort_rotation(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, descending: bool) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let parity_indices: Vec<usize> = (start_page as usize..=end_page as usize)
        .filter(|i| parity_batch_match(*i as u32, start_page, end_page, odd, local))
        .collect();
    if parity_indices.len() < 2 {
        return Ok(parity_indices.len() as u32);
    }
    let mut indexed: Vec<(usize, i64, Object)> = parity_indices
        .iter()
        .map(|i| {
            let kid = kids[*i].clone();
            let rot = kid.as_reference().ok().map(|id| page_rotation(&doc, id).rem_euclid(360)).unwrap_or(0);
            (*i, rot, kid)
        })
        .collect();
    indexed.sort_by(|a, b| {
        let ord = a.1.cmp(&b.1);
        if descending { ord.reverse() } else { ord }
    });
    let sorted_kids: Vec<Object> = indexed.into_iter().map(|(_, _, kid)| kid).collect();
    for (pos, idx) in parity_indices.iter().enumerate() {
        kids[*idx] = sorted_kids[pos].clone();
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(parity_indices.len() as u32)
}

fn parity_batch_sort_size(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, descending: bool) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let parity_indices: Vec<usize> = (start_page as usize..=end_page as usize)
        .filter(|i| parity_batch_match(*i as u32, start_page, end_page, odd, local))
        .collect();
    if parity_indices.len() < 2 {
        return Ok(parity_indices.len() as u32);
    }
    let mut indexed: Vec<(usize, f64, Object)> = parity_indices
        .iter()
        .map(|i| {
            let kid = kids[*i].clone();
            let area = kid
                .as_reference()
                .ok()
                .and_then(|id| page_media_box(&doc, id).ok())
                .map(|m| (m[2] - m[0]) * (m[3] - m[1]))
                .unwrap_or(0.0);
            (*i, area, kid)
        })
        .collect();
    indexed.sort_by(|a, b| {
        let ord = a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal);
        if descending { ord.reverse() } else { ord }
    });
    let sorted_kids: Vec<Object> = indexed.into_iter().map(|(_, _, kid)| kid).collect();
    for (pos, idx) in parity_indices.iter().enumerate() {
        kids[*idx] = sorted_kids[pos].clone();
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(parity_indices.len() as u32)
}

#[allow(clippy::too_many_arguments)]
fn parity_batch_crop(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, margin_top: f64, margin_right: f64, margin_bottom: f64, margin_left: f64) -> Result<u32, String> {
    if margin_top < 0.0 || margin_right < 0.0 || margin_bottom < 0.0 || margin_left < 0.0 {
        return Err("Margins must be non-negative".to_string());
    }
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut cropped = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        apply_crop_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
        cropped += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(cropped)
}

#[allow(clippy::too_many_arguments)]
fn parity_batch_expand(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, margin_top: f64, margin_right: f64, margin_bottom: f64, margin_left: f64) -> Result<u32, String> {
    if margin_top < 0.0 || margin_right < 0.0 || margin_bottom < 0.0 || margin_left < 0.0 {
        return Err("Margins must be non-negative".to_string());
    }
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut expanded = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        apply_expand_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
        expanded += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(expanded)
}

#[allow(clippy::too_many_arguments)]
fn parity_batch_shrink(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, margin_top: f64, margin_right: f64, margin_bottom: f64, margin_left: f64) -> Result<u32, String> {
    if margin_top < 0.0 || margin_right < 0.0 || margin_bottom < 0.0 || margin_left < 0.0 {
        return Err("Margins must be non-negative".to_string());
    }
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut shrunk = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        apply_shrink_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
        shrunk += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(shrunk)
}

fn parity_batch_clear_crop(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut cleared = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        if doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.remove(b"CropBox").is_some() {
            cleared += 1;
        }
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(cleared)
}

fn parity_batch_blank(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, after: bool) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let indices: Vec<usize> = (start_page as usize..=end_page as usize)
        .filter(|i| parity_batch_match(*i as u32, start_page, end_page, odd, local))
        .collect();
    for &idx in indices.iter().rev() {
        let page_id = create_blank_page(&mut doc, pages_ref);
        let at = if after { idx + 1 } else { idx };
        kids.insert(at, Object::Reference(page_id));
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(indices.len() as u32)
}

fn parity_batch_bookmark(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, prefix: Option<String>) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let prefix = prefix.unwrap_or_else(|| "Page ".to_string());
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut count = 0u32;
    for (page_num, page_id) in doc.get_pages() {
        let page_index = page_num - 1;
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let title = format!("{prefix}{page_num}");
        append_outline_item(&mut doc, &title, page_id)?;
        count += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(count)
}

fn parity_batch_page_size(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, preset: &str) -> Result<u32, String> {
    let (w, h) = page_size_preset_dims(preset)?;
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut resized = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let page = doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?;
        page.set(
            "MediaBox",
            Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Real(w as f32), Object::Real(h as f32)]),
        );
        page.remove(b"CropBox");
        resized += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(resized)
}

fn parity_batch_extract(path: &Path, output_path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool) -> Result<String, String> {
    if path == output_path {
        return Err("Output path must differ from the source PDF".to_string());
    }
    validate_page_range(path, start_page, end_page)?;
    let doc = Document::load(path).map_err(|e| e.to_string())?;
    let (all_kids, pages_ref) = get_pages_kids(&doc)?;
    let subset: Vec<Object> = all_kids
        .iter()
        .enumerate()
        .filter(|(i, _)| parity_batch_match(*i as u32, start_page, end_page, odd, local))
        .map(|(_, kid)| kid.clone())
        .collect();
    if subset.is_empty() {
        return Err("No pages match the extract filter in range".to_string());
    }
    let mut out = Document::with_version(doc.version.clone());
    set_pages_kids(&mut out, pages_ref, subset)?;
    out.save(output_path).map_err(|e| e.to_string())?;
    Ok(output_path.to_string_lossy().into_owned())
}

fn parity_batch_page_numbers(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, prefix: Option<String>) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let prefix = prefix.unwrap_or_default();
    let mut stamped = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let font_name = ensure_helvetica_font(&mut doc, page_id)?;
        let (px, py) = viewer_point_to_pdf(&doc, page_id, 380.0, 1100.0)?;
        let label = format!("{prefix}{}", page_index + 1);
        let ops = build_page_number_ops(&font_name, &label, px, py, 12.0);
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        stamped += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(stamped)
}

fn parity_batch_watermark(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Watermark text cannot be empty".to_string());
    }
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut stamped = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let font_name = ensure_helvetica_font(&mut doc, page_id)?;
        let (cx, cy) = viewer_point_to_pdf(&doc, page_id, 400.0, 566.0)?;
        let ops = build_watermark_ops(&font_name, trimmed, cx, cy);
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        stamped += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(stamped)
}

fn parity_batch_header(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Header text cannot be empty".to_string());
    }
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut stamped = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let font_name = ensure_helvetica_font(&mut doc, page_id)?;
        let (px, py) = viewer_point_to_pdf(&doc, page_id, 380.0, 40.0)?;
        let ops = build_page_number_ops(&font_name, trimmed, px, py, 12.0);
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        stamped += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(stamped)
}

fn parity_batch_footer(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Footer text cannot be empty".to_string());
    }
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut stamped = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let font_name = ensure_helvetica_font(&mut doc, page_id)?;
        let (px, py) = viewer_point_to_pdf(&doc, page_id, 380.0, 1100.0)?;
        let ops = build_page_number_ops(&font_name, trimmed, px, py, 12.0);
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        stamped += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(stamped)
}

fn parity_batch_border(path: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, inset: f64) -> Result<u32, String> {
    validate_page_range(path, start_page, end_page)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut drawn = 0u32;
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let ops = build_page_border_ops(&doc, page_id, inset)?;
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        drawn += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(drawn)
}

fn parity_batch_export_pdf(path: &Path, output_dir: &Path, start_page: u32, end_page: u32, odd: bool, local: bool) -> Result<Vec<String>, String> {
    if !path.is_file() {
        return Err("File not found".to_string());
    }
    validate_page_range(path, start_page, end_page)?;
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    let path_str = path.to_string_lossy().into_owned();
    let mut written = Vec::new();
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let file_name = format!("page-{:03}.pdf", page_index + 1);
        let output_path = output_dir.join(file_name);
        let out = extract_pdf_pages(path_str.clone(), output_path.to_string_lossy().into_owned(), page_index, page_index)?;
        written.push(out);
    }
    Ok(written)
}

#[allow(clippy::too_many_arguments)]
fn parity_batch_export_rendered(path: &Path, output_dir: &Path, start_page: u32, end_page: u32, odd: bool, local: bool, ext: &str, render: ParityPageRenderFn) -> Result<Vec<String>, String> {
    if !path.is_file() {
        return Err("File not found".to_string());
    }
    validate_page_range(path, start_page, end_page)?;
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    let mut written = Vec::new();
    for page_index in start_page..=end_page {
        if !parity_batch_match(page_index, start_page, end_page, odd, local) {
            continue;
        }
        let bytes = render(path, page_index, EXPORT_PNG_W, EXPORT_PNG_H)?;
        let file_name = format!("page-{:03}.{ext}", page_index + 1);
        let output_path = output_dir.join(file_name);
        write_png_output(&output_path, &bytes)?;
        written.push(output_path.to_string_lossy().into_owned());
    }
    Ok(written)
}

fn parity_batch_export_ico_doc(path: &Path, output_dir: &Path, odd: bool) -> Result<Vec<String>, String> {
    export_pages_by_parity_rendered(path, output_dir, odd, "ico", render_page_ico)
}
"""


def impl_call(kind: str, odd: bool, local: bool) -> str:
    o = "true" if odd else "false"
    l = "true" if local else "false"
    path = "&PathBuf::from(&path)"
    rng = f"{path}, start_page, end_page"
    pl = f"{rng}, {o}, {l}"
    calls = {
        "rotate_cw": f"parity_batch_rotate({pl}, 90)",
        "rotate_ccw": f"parity_batch_rotate({pl}, -90)",
        "rotate_180": f"parity_batch_rotate({pl}, 180)",
        "reset_rot": f"parity_batch_reset_rotation({pl})",
        "delete": f"parity_batch_delete({pl})",
        "keep": f"parity_batch_keep({pl})",
        "dup_append": f"parity_batch_dup_append({pl})",
        "dup_before": f"parity_batch_dup_before({pl})",
        "dup_to_start": f"parity_batch_dup_to_start({pl})",
        "dup_to_end": f"parity_batch_dup_to_end({pl})",
        "flatten": f"parity_batch_flatten({pl})",
        "reverse": f"parity_batch_reverse({pl})",
        "move_odd_start": f"parity_batch_move_seg({rng}, true, {l})",
        "move_even_start": f"parity_batch_move_seg({rng}, false, {l})",
        "move_odd_end": f"parity_batch_move_seg({rng}, false, {l})",
        "move_even_end": f"parity_batch_move_seg({rng}, true, {l})",
        "sort_rot": f"parity_batch_sort_rotation({pl}, false)",
        "sort_size": f"parity_batch_sort_size({pl}, false)",
        "crop": f"parity_batch_crop({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "expand": f"parity_batch_expand({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "shrink": f"parity_batch_shrink({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "clear_crop": f"parity_batch_clear_crop({pl})",
        "blank_before": f"parity_batch_blank({pl}, false)",
        "blank_after": f"parity_batch_blank({pl}, true)",
        "bookmark": f"parity_batch_bookmark({pl}, prefix)",
        "page_size": f"parity_batch_page_size({pl}, &preset)",
        "extract": f"parity_batch_extract({path}, &PathBuf::from(&output_path), start_page, end_page, {o}, {l})",
        "page_numbers": f"parity_batch_page_numbers({pl}, prefix)",
        "watermark": f"parity_batch_watermark({pl}, &text)",
        "header": f"parity_batch_header({pl}, &text)",
        "footer": f"parity_batch_footer({pl}, &text)",
        "border": f"parity_batch_border({pl}, inset)",
        "export_pdf": f"parity_batch_export_pdf({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, {l})",
        "export_png": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, {l}, "png", render_page_png)',
        "export_jpeg": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, {l}, "jpeg", render_page_jpeg)',
        "export_webp": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, {l}, "webp", render_page_webp)',
        "export_bmp": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, {l}, "bmp", render_page_bmp)',
        "export_tiff": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, {l}, "tiff", render_page_tiff)',
        "export_gif": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, {l}, "gif", render_page_gif)',
        "export_ppm": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, {l}, "ppm", render_page_ppm)',
        "export_tga": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, {l}, "tga", render_page_tga)',
        "export_ico": f'parity_batch_export_rendered({path}, &PathBuf::from(&output_dir), start_page, end_page, {o}, {l}, "ico", render_page_ico)',
    }
    return calls[kind]


def command_sig(kind: str) -> tuple[str, str]:
    base = "path: String, start_page: u32, end_page: u32"
    ret = "Result<u32, String>"
    if kind == "extract":
        return f"{base}, output_path: String", "Result<String, String>"
    if kind.startswith("export"):
        return f"{base}, output_dir: String", "Result<Vec<String>, String>"
    if kind in ("crop", "expand", "shrink"):
        return f"{base}, margin_top: f64, margin_right: f64, margin_bottom: f64, margin_left: f64", ret
    if kind == "bookmark" or kind == "page_numbers":
        return f"{base}, prefix: Option<String>", ret
    if kind in ("watermark", "header", "footer"):
        return f"{base}, text: String", ret
    if kind == "page_size":
        return f"{base}, preset: String", ret
    if kind == "border":
        return f"{base}, inset: f64", ret
    if kind in ("move_odd_start", "move_even_start", "move_odd_end", "move_even_end"):
        return base, "Result<(), String>"
    return base, ret


def gen_command(name: str, kind: str, odd: bool, local: bool, doc_comment: str) -> str:
    if kind == "export_ico_doc":
        o = "true" if odd else "false"
        return f"""
/// {doc_comment}
#[tauri::command]
fn {name}(path: String, output_dir: String) -> Result<Vec<String>, String> {{
    parity_batch_export_ico_doc(&PathBuf::from(&path), &PathBuf::from(&output_dir), {o})
}}
"""
    sig, ret = command_sig(kind)
    call = impl_call(kind, odd, local)
    return f"""
/// {doc_comment}
#[tauri::command]
fn {name}({sig}) -> {ret} {{
    {call}
}}
"""


def gen_test(name: str, kind: str) -> str:
    if kind == "export_ico_doc":
        return f"""
    #[test]
    fn {name}_rejects_missing_file() {{
        let missing = tmp("{name}_missing_src");
        let output_dir = tmp("{name}_missing_dir");
        let err = {name}(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned()).unwrap_err();
        assert!(err.contains("File not found"));
    }}
"""
    setup = ""
    extra = ""
    if kind == "extract":
        setup = '        let output_path = tmp("extract_out.pdf");\n'
        extra = ", output_path.to_string_lossy().into_owned()"
    elif kind.startswith("export"):
        setup = '        let output_dir = tmp("export_dir");\n'
        extra = ", output_dir.to_string_lossy().into_owned()"
    elif kind in ("crop", "expand", "shrink"):
        extra = ", 0.0, 0.0, 0.0, 0.0"
    elif kind == "bookmark" or kind == "page_numbers":
        extra = ", None"
    elif kind in ("watermark", "header", "footer"):
        extra = ', "wm".to_string()'
    elif kind == "page_size":
        extra = ', "letter".to_string()'
    elif kind == "border":
        extra = ", 1.0"
    return f"""
    #[test]
    fn {name}_rejects_invalid_range() {{
        let path = save(&mut build_pdf(2), "{name}");
{setup}        let err = {name}(path.clone(), 5, 10{extra}).unwrap_err();
        assert!(err.contains("Invalid page range"));
        let _ = std::fs::remove_file(&path);
    }}
"""


def build_specs() -> list[tuple[str, str, bool, bool, str]]:
    specs: list[tuple[str, str, bool, bool, str]] = []
    for base, kind in GLOBAL_RANGE:
        if kind in ("move_odd_start", "move_even_start", "move_odd_end", "move_even_end"):
            specs.append((base, kind, kind.startswith("move_odd"), False, f"Global parity move in range - {kind}"))
            continue
        for odd, label in ((True, "odd"), (False, "even")):
            name = base.replace("_pages", f"_{label}_pages")
            specs.append((name, kind, odd, False, f"Global {label} parity within page range - {kind}"))
    for base, kind in LOCAL_RANGE:
        for odd, label in ((True, "odd"), (False, "even")):
            name = base.replace("_pages", f"_{label}_pages")
            specs.append((name, kind, odd, True, f"Local {label} parity within page range - {kind}"))
    for name, kind, odd in DOC_ICO:
        specs.append((name, kind, odd, False, f"Export {'odd' if odd else 'even'} pages as ICO"))
    return specs


def main() -> None:
    specs = build_specs()
    assert len(specs) == 100, len(specs)

    lines = ["// Auto-generated by scripts/gen-parity-batch.py - do not edit.", rust_helpers()]
    tests = ["// Auto-generated parity batch tests"]
    handlers: list[str] = []

    for name, kind, odd, local, comment in specs:
        lines.append(gen_command(name, kind, odd, local, comment))
        tests.append(gen_test(name, kind))
        handlers.append(f"            {name},")

    INC.write_text("\n".join(lines))
    print(f"Wrote {INC} ({len(specs)} commands)")

    import sys

    sys.path.insert(0, str(ROOT / "scripts"))
    from parity_patch import patch_sources

    patch_sources("BATCH", handlers, tests)

    ui_path = ROOT / "src" / "parity_batch_commands.json"
    # Keep UI list in sync for App.tsx import
    import json
    ui_path.write_text(json.dumps([s[0] for s in specs], indent=2))
    print(f"Wrote {ui_path}")


if __name__ == "__main__":
    main()
