use crate::pdf::coords::{page_media_box, VIEWER_PAGE_H, VIEWER_PAGE_W};
use lopdf::{Document, Object, ObjectId};
use std::path::Path;

pub fn page_size_preset_dims(preset: &str) -> Result<(f64, f64), String> {
    match preset.to_ascii_lowercase().as_str() {
        "letter" => Ok((612.0, 792.0)),
        "a4" => Ok((595.28, 841.89)),
        "legal" => Ok((612.0, 1008.0)),
        _ => Err(format!("Unknown page size preset: {preset} (use letter, a4, or legal)")),
    }
}

pub fn set_page_size(path: &Path, start_page: u32, end_page: u32, preset: &str) -> Result<u32, String> {
    let (w, h) = page_size_preset_dims(preset)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let mut resized = 0u32;
    for page_index in start_page..=end_page {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let page = doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?;
        page.set(
            "MediaBox",
            Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Real(w as f32), Object::Real(h as f32)]),
        );
        page.remove(b"CropBox");
        resized += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(resized)
}

pub fn apply_expand_margins(
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
    let left = media[0] - margin_left * mw / VIEWER_PAGE_W;
    let bottom = media[1] - margin_bottom * mh / VIEWER_PAGE_H;
    let right = media[2] + margin_right * mw / VIEWER_PAGE_W;
    let top = media[3] + margin_top * mh / VIEWER_PAGE_H;
    if right <= left || top <= bottom {
        return Err("Expand margins are too large".to_string());
    }
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(
        b"MediaBox",
        Object::Array(vec![
            Object::Real(left as f32),
            Object::Real(bottom as f32),
            Object::Real(right as f32),
            Object::Real(top as f32),
        ]),
    );
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.remove(b"CropBox");
    Ok(())
}

pub fn expand_page_margins(
    path: &Path,
    start_page: u32,
    end_page: u32,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
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
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let mut expanded = 0u32;
    for page_index in start_page..=end_page {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        apply_expand_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
        expanded += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(expanded)
}

pub fn apply_shrink_margins(
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
        return Err("Shrink margins are too large".to_string());
    }
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(
        b"MediaBox",
        Object::Array(vec![
            Object::Real(left as f32),
            Object::Real(bottom as f32),
            Object::Real(right as f32),
            Object::Real(top as f32),
        ]),
    );
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.remove(b"CropBox");
    Ok(())
}

pub fn shrink_page_margins(
    path: &Path,
    start_page: u32,
    end_page: u32,
    margin_top: f64,
    margin_right: f64,
    margin_bottom: f64,
    margin_left: f64,
) -> Result<u32, String> {
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
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let mut shrunk = 0u32;
    for page_index in start_page..=end_page {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        apply_shrink_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
        shrunk += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(shrunk)
}

pub fn set_page_size_by_parity(path: &Path, odd: bool, preset: &str) -> Result<u32, String> {
    let (w, h) = page_size_preset_dims(preset)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut resized = 0u32;
    for page_index in 0..total {
        if (page_index % 2 == 0) != odd {
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
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(resized)
}
