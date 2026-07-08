use crate::pdf::content::append_page_content;
use crate::pdf::coords::page_media_box;
use crate::pdf::crop::apply_crop_margins;
use crate::pdf::merge_split::{extract_pdf_pages, insert_pdf};
use crate::pdf::page_decor::{append_outline_item, build_page_number_ops, build_watermark_ops, create_blank_page};
use crate::pdf::page_margins::{apply_expand_margins, apply_shrink_margins};
use crate::pdf::page_ops::{duplicate_page, move_page_to_last};
use crate::pdf::page_text::{ensure_helvetica_font, viewer_point_to_pdf};
use crate::pdf::page_tree::{flatten_pages, get_pages_kids, set_pages_kids};
use crate::pdf::rotation::page_rotation;
use lopdf::{Document, Object};
use std::fs;
use std::path::{Path, PathBuf};

pub fn extract_pages_by_parity(path: &Path, output_path: &Path, odd: bool) -> Result<String, String> {
    if path == output_path {
        return Err("Output path must differ from the source PDF".to_string());
    }
    let doc = Document::load(path).map_err(|e| e.to_string())?;
    let (all_kids, pages_ref) = get_pages_kids(&doc)?;
    let subset: Vec<Object> = all_kids
        .iter()
        .enumerate()
        .filter(|(i, _)| if odd { i % 2 == 0 } else { i % 2 == 1 })
        .map(|(_, kid)| kid.clone())
        .collect();
    if subset.is_empty() {
        return Err(if odd {
            "Document has no odd-indexed pages".to_string()
        } else {
            "Document has no even-indexed pages".to_string()
        });
    }
    let mut out = Document::load(path).map_err(|e| e.to_string())?;
    set_pages_kids(&mut out, pages_ref, subset)?;
    out.prune_objects();
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    crate::pdf::io::save_atomic(&mut out, output_path)?;
    Ok(output_path.to_string_lossy().into_owned())
}

/// Drop every page whose 0-based index parity is `remove_rem`, then save.
/// `validate` sees (kept, total) before any mutation and can reject; a no-op
/// removal returns `Ok(0)` without rewriting the file.
fn remove_pages_by_index_parity(
    path: &Path,
    remove_rem: usize,
    validate: impl Fn(usize, usize) -> Result<(), String>,
) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (kids, _) = get_pages_kids(&doc)?;
    let total = kids.len();
    let kept: Vec<Object> =
        kids.iter().enumerate().filter(|(i, _)| i % 2 != remove_rem).map(|(_, kid)| kid.clone()).collect();
    validate(kept.len(), total)?;
    let deleted = (total - kept.len()) as u32;
    if deleted == 0 {
        return Ok(0);
    }
    set_pages_kids(&mut doc, pages_ref, kept)?;
    doc.prune_objects();
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(deleted)
}

pub fn keep_pages_by_parity(path: &Path, keep_odd: bool) -> Result<u32, String> {
    remove_pages_by_index_parity(path, if keep_odd { 1 } else { 0 }, |kept, total| {
        if kept == 0 {
            return Err(if keep_odd {
                "Document has no odd-indexed pages".to_string()
            } else {
                "Document has no even-indexed pages".to_string()
            });
        }
        if kept == total {
            return Err("Nothing to delete - all pages match the keep filter".to_string());
        }
        Ok(())
    })
}

pub fn delete_pages_by_parity(path: &Path, delete_odd: bool) -> Result<u32, String> {
    remove_pages_by_index_parity(path, if delete_odd { 0 } else { 1 }, |kept, total| {
        if kept == 0 && total > 0 {
            return Err("Cannot delete every page in the document".to_string());
        }
        Ok(())
    })
}

pub fn flatten_annotations_by_parity(path: &Path, odd: bool) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut removed = 0u32;
    for page_index in 0..total {
        if (page_index % 2 == 0) != odd {
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
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(removed)
}

pub fn crop_pages_by_parity(
    path: &Path,
    odd: bool,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    if margin_top < 0.0 || margin_right < 0.0 || margin_bottom < 0.0 || margin_left < 0.0 {
        return Err("Margins must be non-negative".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut cropped = 0u32;
    for page_index in 0..total {
        if (page_index % 2 == 0) != odd {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        apply_crop_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
        cropped += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(cropped)
}

pub fn expand_pages_by_parity(
    path: &Path,
    odd: bool,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    if margin_top < 0.0 || margin_right < 0.0 || margin_bottom < 0.0 || margin_left < 0.0 {
        return Err("Margins must be non-negative".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut expanded = 0u32;
    for page_index in 0..total {
        if (page_index % 2 == 0) != odd {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        apply_expand_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
        expanded += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(expanded)
}

pub fn shrink_pages_by_parity(
    path: &Path,
    odd: bool,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    if margin_top < 0.0 || margin_right < 0.0 || margin_bottom < 0.0 || margin_left < 0.0 {
        return Err("Margins must be non-negative".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut shrunk = 0u32;
    for page_index in 0..total {
        if (page_index % 2 == 0) != odd {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        apply_shrink_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
        shrunk += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(shrunk)
}

pub fn reverse_pages_by_parity(path: &Path, odd: bool) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let parity_indices: Vec<usize> = (0..kids.len()).filter(|i| (i % 2 == 0) == odd).collect();
    if parity_indices.len() < 2 {
        return Ok(0);
    }
    let mut parity_kids: Vec<Object> = parity_indices.iter().map(|i| kids[*i].clone()).collect();
    parity_kids.reverse();
    for (pos, idx) in parity_indices.iter().enumerate() {
        kids[*idx] = parity_kids[pos].clone();
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(parity_indices.len() as u32)
}

pub fn move_pages_by_parity_to_start(path: &Path, odd_first: bool) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (kids, _) = get_pages_kids(&doc)?;
    let mut odd = Vec::new();
    let mut even = Vec::new();
    for (i, kid) in kids.into_iter().enumerate() {
        if i % 2 == 0 {
            odd.push(kid);
        } else {
            even.push(kid);
        }
    }
    let new_kids =
        if odd_first { odd.into_iter().chain(even).collect() } else { even.into_iter().chain(odd).collect() };
    set_pages_kids(&mut doc, pages_ref, new_kids)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn move_pages_by_parity_to_end(path: &Path, odd_last: bool) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (kids, _) = get_pages_kids(&doc)?;
    let mut odd = Vec::new();
    let mut even = Vec::new();
    for (i, kid) in kids.into_iter().enumerate() {
        if i % 2 == 0 {
            odd.push(kid);
        } else {
            even.push(kid);
        }
    }
    let new_kids = if odd_last { even.into_iter().chain(odd).collect() } else { odd.into_iter().chain(even).collect() };
    set_pages_kids(&mut doc, pages_ref, new_kids)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn clear_crop_pages_by_parity(path: &Path, odd: bool) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut cleared = 0u32;
    for page_index in 0..total {
        if (page_index % 2 == 0) != odd {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        if doc.get_dictionary(page_id).ok().and_then(|d| d.get(b"CropBox").ok()).is_some() {
            doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.remove(b"CropBox");
            cleared += 1;
        }
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(cleared)
}

pub fn duplicate_pages_by_parity_before(path: &Path, odd: bool) -> Result<u32, String> {
    let path_buf = PathBuf::from(path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    let indices: Vec<u32> = (0..total).filter(|i| (i % 2 == 0) == odd).collect();
    if indices.is_empty() {
        return Ok(0);
    }
    let copied = indices.len() as u32;
    for &idx in indices.iter().rev() {
        insert_pdf(path, path, idx, idx, idx)?;
    }
    Ok(copied)
}

pub fn sort_pages_by_parity_rotation(path: &Path, odd: bool, descending: bool) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let parity_indices: Vec<usize> = (0..kids.len()).filter(|i| (i % 2 == 0) == odd).collect();
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
        if descending {
            ord.reverse()
        } else {
            ord
        }
    });
    let sorted_kids: Vec<Object> = indexed.into_iter().map(|(_, _, kid)| kid).collect();
    for (pos, idx) in parity_indices.iter().enumerate() {
        kids[*idx] = sorted_kids[pos].clone();
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(parity_indices.len() as u32)
}

pub fn sort_pages_by_parity_size(path: &Path, odd: bool, descending: bool) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let parity_indices: Vec<usize> = (0..kids.len()).filter(|i| (i % 2 == 0) == odd).collect();
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
        if descending {
            ord.reverse()
        } else {
            ord
        }
    });
    let sorted_kids: Vec<Object> = indexed.into_iter().map(|(_, _, kid)| kid).collect();
    for (pos, idx) in parity_indices.iter().enumerate() {
        kids[*idx] = sorted_kids[pos].clone();
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(parity_indices.len() as u32)
}

pub fn add_page_numbers_by_parity(path: &Path, odd: bool, prefix: Option<String>) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let prefix = prefix.unwrap_or_default();
    let mut stamped = 0u32;
    for page_index in 0..total {
        if (page_index % 2 == 0) != odd {
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
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(stamped)
}

pub fn add_text_watermark_by_parity(path: &Path, odd: bool, text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Watermark text cannot be empty".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut stamped = 0u32;
    for page_index in 0..total {
        if (page_index % 2 == 0) != odd {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let font_name = ensure_helvetica_font(&mut doc, page_id)?;
        let (cx, cy) = viewer_point_to_pdf(&doc, page_id, 400.0, 566.0)?;
        let ops = build_watermark_ops(&font_name, trimmed, cx, cy);
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        stamped += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(stamped)
}

pub fn bookmark_pages_by_parity(path: &Path, odd: bool, prefix: Option<String>) -> Result<u32, String> {
    let prefix = prefix.unwrap_or_else(|| "Page ".to_string());
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut count = 0u32;
    for (page_num, page_id) in doc.get_pages() {
        let page_index = page_num - 1;
        if (page_index % 2 == 0) != odd {
            continue;
        }
        let title = format!("{prefix}{page_num}");
        append_outline_item(&mut doc, &title, page_id)?;
        count += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(count)
}

pub fn insert_blank_by_parity(path: &Path, odd: bool, after: bool) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let indices: Vec<usize> = (0..kids.len()).filter(|i| (*i % 2 == 0) == odd).collect();
    if indices.is_empty() {
        return Ok(0);
    }
    for &idx in indices.iter().rev() {
        let page_id = create_blank_page(&mut doc, pages_ref);
        let at = if after { idx + 1 } else { idx };
        kids.insert(at, Object::Reference(page_id));
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(indices.len() as u32)
}

pub fn duplicate_pages_by_parity_to_end(path: &Path, odd: bool) -> Result<u32, String> {
    let path_buf = PathBuf::from(path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    let indices: Vec<u32> = (0..total).filter(|i| (i % 2 == 0) == odd).collect();
    if indices.is_empty() {
        return Ok(0);
    }
    let copied = indices.len() as u32;
    for &idx in indices.iter().rev() {
        let new_idx = duplicate_page(path, idx)?;
        move_page_to_last(path, new_idx)?;
    }
    Ok(copied)
}

pub fn duplicate_pages_by_parity_to_start(path: &Path, odd: bool) -> Result<u32, String> {
    let path_buf = PathBuf::from(path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    let indices: Vec<u32> = (0..total).filter(|i| (i % 2 == 0) == odd).collect();
    if indices.is_empty() {
        return Ok(0);
    }
    let copied = indices.len() as u32;
    for (offset, &idx) in indices.iter().rev().enumerate() {
        let offset = offset as u32;
        let source = idx + offset;
        insert_pdf(path, path, offset, source, source)?;
    }
    Ok(copied)
}

pub fn export_pages_by_parity_as_pdf(path: &Path, output_dir: &Path, odd: bool) -> Result<Vec<String>, String> {
    if !path.is_file() {
        return Err("File not found".to_string());
    }
    let total = Document::load(path).map_err(|e| e.to_string())?.get_pages().len() as u32;
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    let mut written = Vec::new();
    for page_index in 0..total {
        if (page_index % 2 == 0) != odd {
            continue;
        }
        let file_name = format!("page-{:03}.pdf", page_index + 1);
        let output_path = output_dir.join(file_name);
        let out = extract_pdf_pages(path, &output_path, page_index, page_index)?;
        written.push(out);
    }
    Ok(written)
}
