use crate::pdf::coords::page_media_box;
use crate::pdf::crop::apply_crop_margins;
use crate::pdf::page_decor::create_blank_page;
use crate::pdf::page_tree::{flatten_pages, get_pages_kids, set_pages_kids};
use crate::pdf::rotation::{page_rotation, set_page_rotation};
use lopdf::{Document, Object};
use std::path::Path;

pub fn reset_rotation_range(path: &Path, start_page: u32, end_page: u32) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let mut reset = 0u32;
    for page_index in start_page..=end_page {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        set_page_rotation(&mut doc, page_id, 0)?;
        reset += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(reset)
}

pub fn rotate_page_180_range(path: &Path, start_page: u32, end_page: u32) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let mut rotated = 0u32;
    for page_index in start_page..=end_page {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let current = page_rotation(&doc, page_id);
        set_page_rotation(&mut doc, page_id, current + 180)?;
        rotated += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(rotated)
}

pub fn reverse_page_range(path: &Path, start_page: u32, end_page: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let start = start_page as usize;
    let end = end_page as usize;
    if start > end || end >= kids.len() {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let mut segment: Vec<Object> = kids.drain(start..=end).collect();
    segment.reverse();
    for (offset, kid) in segment.into_iter().enumerate() {
        kids.insert(start + offset, kid);
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn insert_blank_pages(path: &Path, at_index: u32, count: u32) -> Result<u32, String> {
    if count == 0 {
        return Err("Count must be at least 1".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let at = at_index as usize;
    if at > kids.len() {
        return Err("Insert index out of bounds".to_string());
    }
    for offset in 0..count {
        let page_id = create_blank_page(&mut doc, pages_ref);
        kids.insert(at + offset as usize, Object::Reference(page_id));
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(count)
}

pub fn crop_page_range(
    path: &Path,
    start_page: u32,
    end_page: u32,
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
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let mut cropped = 0u32;
    for page_index in start_page..=end_page {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        apply_crop_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
        cropped += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(cropped)
}

pub fn sort_pages_by_size(path: &Path, descending: bool) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (kids, _) = get_pages_kids(&doc)?;
    let mut indexed: Vec<(usize, f64, Object)> = kids
        .into_iter()
        .enumerate()
        .map(|(i, kid)| {
            let area = kid
                .as_reference()
                .ok()
                .and_then(|id| page_media_box(&doc, id).ok())
                .map(|m| (m[2] - m[0]) * (m[3] - m[1]))
                .unwrap_or(0.0);
            (i, area, kid)
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
    let sorted: Vec<Object> = indexed.into_iter().map(|(_, _, kid)| kid).collect();
    set_pages_kids(&mut doc, pages_ref, sorted)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

/// Set rotation on every odd (0-based even index) or even page: add `Some(delta)`
/// to the current rotation, or clear it with `None`.
pub fn rotate_pages_by_parity(path: &Path, odd: bool, delta: Option<i64>) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let target_rem = if odd { 0 } else { 1 };
    let mut changed = 0u32;
    for page_index in 0..total {
        if page_index % 2 != target_rem {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let rotation = match delta {
            Some(delta) => page_rotation(&doc, page_id) + delta,
            None => 0,
        };
        set_page_rotation(&mut doc, page_id, rotation)?;
        changed += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(changed)
}

pub fn rotate_odd_pages(path: &Path) -> Result<u32, String> {
    rotate_pages_by_parity(path, true, Some(90))
}

pub fn rotate_even_pages(path: &Path) -> Result<u32, String> {
    rotate_pages_by_parity(path, false, Some(90))
}

pub fn delete_every_nth_page(path: &Path, nth: u32) -> Result<u32, String> {
    if nth < 2 {
        return Err("Nth must be at least 2".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let total = kids.len();
    let mut to_delete: Vec<usize> = (0..total).filter(|i| (i + 1) % nth as usize == 0).collect();
    if to_delete.is_empty() {
        return Ok(0);
    }
    if to_delete.len() >= total {
        return Err("Cannot delete every page in the document".to_string());
    }
    to_delete.sort_by(|a, b| b.cmp(a));
    let deleted = to_delete.len() as u32;
    for idx in to_delete {
        kids.remove(idx);
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    doc.prune_objects();
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(deleted)
}

pub fn move_page_range_to_end(path: &Path, start_page: u32, end_page: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let start = start_page as usize;
    let end = end_page as usize;
    if start > end || end >= kids.len() {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let segment: Vec<Object> = kids.drain(start..=end).collect();
    let insert_at = kids.len();
    for (offset, kid) in segment.into_iter().enumerate() {
        kids.insert(insert_at + offset, kid);
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn duplicate_page_range_to_start(path: &Path, start_page: u32, end_page: u32) -> Result<u32, String> {
    let path_buf = path.to_path_buf();
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let count = end_page - start_page + 1;
    crate::pdf::merge_split::insert_pdf(&path_buf, &path_buf, 0, start_page, end_page)?;
    Ok(count)
}
