use lopdf::{Document, Object, ObjectId};

pub fn page_rotation(doc: &Document, page_id: ObjectId) -> i64 {
    doc.get_dictionary(page_id).ok().and_then(|d| d.get(b"Rotate").ok()).and_then(|o| o.as_i64().ok()).unwrap_or(0)
}

pub fn set_page_rotation(doc: &mut Document, page_id: ObjectId, rotation: i64) -> Result<(), String> {
    let normalized = rotation.rem_euclid(360);
    if normalized == 0 {
        doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.remove(b"Rotate");
    } else {
        doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(b"Rotate", Object::Integer(normalized));
    }
    Ok(())
}

/// Rotate one page by `delta_degrees` (e.g. 90 for clockwise, 270 for counter-clockwise).
pub fn rotate_page_at(doc: &mut Document, page_index: u32, delta_degrees: i64) -> Result<(), String> {
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    let next = (page_rotation(doc, page_id) + delta_degrees).rem_euclid(360);
    set_page_rotation(doc, page_id, next)
}

/// Rotate every page by `delta_degrees`. Returns the number of pages rotated.
pub fn rotate_all_pages_by(doc: &mut Document, delta_degrees: i64) -> Result<u32, String> {
    let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();
    for page_id in &page_ids {
        let next = (page_rotation(doc, *page_id) + delta_degrees).rem_euclid(360);
        set_page_rotation(doc, *page_id, next)?;
    }
    Ok(page_ids.len() as u32)
}

/// Clear rotation on one page.
pub fn reset_page_rotation_at(doc: &mut Document, page_index: u32) -> Result<(), String> {
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    set_page_rotation(doc, page_id, 0)
}

/// Clear rotation on every page. Returns the number of pages cleared.
pub fn reset_all_page_rotations(doc: &mut Document) -> Result<u32, String> {
    let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();
    for page_id in &page_ids {
        set_page_rotation(doc, *page_id, 0)?;
    }
    Ok(page_ids.len() as u32)
}
