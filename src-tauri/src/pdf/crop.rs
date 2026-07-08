use crate::pdf::coords::{page_media_box, VIEWER_PAGE_H, VIEWER_PAGE_W};
use lopdf::{Document, Object, ObjectId};
use std::path::Path;

pub fn apply_crop_margins(
    doc: &mut Document,
    page_id: ObjectId,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<(), String> {
    let media = page_media_box(doc, page_id)?;
    let mw = media[2] - media[0];
    let mh = media[3] - media[1];
    if mw <= 0.0 || mh <= 0.0 {
        return Err("Invalid page size".to_string());
    }
    let left = media[0] + margin_left * mw / VIEWER_PAGE_W;
    let bottom = media[1] + margin_bottom * mh / VIEWER_PAGE_H;
    let right = media[2] - margin_right * mw / VIEWER_PAGE_W;
    let top = media[3] - margin_top * mh / VIEWER_PAGE_H;
    if right <= left || top <= bottom {
        return Err("Crop margins are too large".to_string());
    }
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(
        b"CropBox",
        Object::Array(vec![
            Object::Real(left as f32),
            Object::Real(bottom as f32),
            Object::Real(right as f32),
            Object::Real(top as f32),
        ]),
    );
    Ok(())
}

pub fn crop_page(
    path: &Path,
    page_index: u32,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<(), String> {
    // Reject NaN/±Inf explicitly: `x < 0.0` is false for NaN so the
    // negative-margin check alone would let NaN slip through into CropBox.
    for (name, v) in [
        ("margin_top", margin_top),
        ("margin_right", margin_right),
        ("margin_bottom", margin_bottom),
        ("margin_left", margin_left),
    ] {
        crate::pdf::coords::finite_f64(v, name)?;
    }
    if margin_top < 0.0 || margin_right < 0.0 || margin_bottom < 0.0 || margin_left < 0.0 {
        return Err("Margins must be non-negative".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    apply_crop_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn crop_all_pages(
    path: &Path,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
    if margin_top < 0.0 || margin_right < 0.0 || margin_bottom < 0.0 || margin_left < 0.0 {
        return Err("Margins must be non-negative".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();
    for page_id in &page_ids {
        apply_crop_margins(&mut doc, *page_id, margin_top, margin_right, margin_bottom, margin_left)?;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(page_ids.len() as u32)
}

pub fn clear_page_crop(path: &Path, page_index: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.remove(b"CropBox");
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn clear_all_page_crops(path: &Path) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();
    let mut cleared = 0u32;
    for page_id in &page_ids {
        if doc.get_dictionary_mut(*page_id).map_err(|e| e.to_string())?.remove(b"CropBox").is_some() {
            cleared += 1;
        }
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(cleared)
}
