use crate::pdf::content::append_page_content;
use crate::pdf::coords::{page_media_box, VIEWER_PAGE_H, VIEWER_PAGE_W};
use crate::pdf::page_text::{ensure_helvetica_font, escape_pdf_literal_string, viewer_point_to_pdf};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::path::Path;

pub fn create_blank_page(doc: &mut Document, pages_ref: ObjectId) -> ObjectId {
    let mut page = Dictionary::new();
    page.set("Type", Object::Name(b"Page".to_vec()));
    page.set("Parent", Object::Reference(pages_ref));
    page.set("Resources", Object::Dictionary(Dictionary::new()));
    page.set(
        "MediaBox",
        Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(612), Object::Integer(792)]),
    );
    let content_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), Vec::new())));
    page.set("Contents", Object::Reference(content_id));
    doc.add_object(Object::Dictionary(page))
}

pub(crate) fn append_outline_item(doc: &mut Document, title: &str, page_id: ObjectId) -> Result<(), String> {
    let catalog_id = doc.trailer.get(b"Root").map_err(|e| e.to_string())?.as_reference().map_err(|_| "Bad Root")?;
    let existing_outlines = doc
        .get_dictionary(catalog_id)
        .map_err(|e| e.to_string())?
        .get(b"Outlines")
        .ok()
        .and_then(|o| o.as_reference().ok());

    let mut item = Dictionary::new();
    item.set("Title", Object::String(title.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    item.set("Dest", Object::Array(vec![Object::Reference(page_id), Object::Name(b"Fit".to_vec())]));
    let item_id = doc.add_object(Object::Dictionary(item));

    if let Some(outlines_id) = existing_outlines {
        let (last_id, count) = {
            let outlines = doc.get_dictionary(outlines_id).map_err(|e| e.to_string())?;
            (
                outlines.get(b"Last").ok().and_then(|o| o.as_reference().ok()),
                outlines.get(b"Count").ok().and_then(|o| o.as_i64().ok()).unwrap_or(0),
            )
        };
        if let Some(last_id) = last_id {
            if let Ok(Object::Dictionary(last)) = doc.get_object_mut(last_id) {
                last.set("Next", Object::Reference(item_id));
            }
            if let Ok(Object::Dictionary(item)) = doc.get_object_mut(item_id) {
                item.set("Parent", Object::Reference(outlines_id));
                item.set("Prev", Object::Reference(last_id));
            }
        } else if let Ok(Object::Dictionary(item)) = doc.get_object_mut(item_id) {
            item.set("Parent", Object::Reference(outlines_id));
        }
        if let Ok(Object::Dictionary(outlines)) = doc.get_object_mut(outlines_id) {
            if last_id.is_none() {
                outlines.set("First", Object::Reference(item_id));
            }
            outlines.set("Last", Object::Reference(item_id));
            outlines.set("Count", Object::Integer(count + 1));
        }
    } else {
        let outlines_id = doc.new_object_id();
        if let Ok(Object::Dictionary(item)) = doc.get_object_mut(item_id) {
            item.set("Parent", Object::Reference(outlines_id));
        }
        let mut outlines = Dictionary::new();
        outlines.set("Type", Object::Name(b"Outlines".to_vec()));
        outlines.set("First", Object::Reference(item_id));
        outlines.set("Last", Object::Reference(item_id));
        outlines.set("Count", Object::Integer(1));
        doc.objects.insert(outlines_id, Object::Dictionary(outlines));
        doc.get_dictionary_mut(catalog_id).map_err(|e| e.to_string())?.set("Outlines", Object::Reference(outlines_id));
    }
    Ok(())
}

pub fn build_page_number_ops(font_name: &str, label: &str, px: f64, py: f64, font_size: f64) -> String {
    let escaped = escape_pdf_literal_string(label);
    format!(
        "\nBT /{font_name} {font_size} Tf 1 0 0 1 {px} {py} Tm ({escaped}) Tj ET\n",
        font_name = font_name,
        font_size = font_size,
        px = px,
        py = py,
        escaped = escaped
    )
}

fn viewer_point_for_stamp(position: &str) -> (f64, f64) {
    match position {
        "footer-left" => (40.0, 1100.0),
        "footer-center" => (380.0, 1100.0),
        "footer-right" => (760.0, 1100.0),
        "header-right" => (760.0, 40.0),
        "header-center" => (380.0, 40.0),
        _ => (380.0, 1100.0),
    }
}

fn stamp_page_range<L>(
    doc: &mut Document,
    start_page: u32,
    end_page: u32,
    position: &str,
    label_at: L,
) -> Result<u32, String>
where
    L: Fn(u32) -> String,
{
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let (vx, vy) = viewer_point_for_stamp(position);
    let mut stamped = 0u32;
    for page_index in start_page..=end_page {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let font_name = ensure_helvetica_font(doc, page_id)?;
        let (px, py) = viewer_point_to_pdf(doc, page_id, vx, vy)?;
        let label = label_at(page_index);
        let ops = build_page_number_ops(&font_name, &label, px, py, 12.0);
        append_page_content(doc, page_id, ops.as_bytes())?;
        stamped += 1;
    }
    Ok(stamped)
}

pub fn bates_label(prefix: &str, n: u64, digits: usize) -> String {
    // ponytail: cap width. u64 is at most 20 digits, so anything beyond that
    // pads pointlessly; without the cap a hostile `digits` (e.g. 4e9 from the
    // command boundary) makes format! allocate gigabytes per page (OOM/hang).
    let digits = digits.min(20);
    format!("{prefix}{n:0digits$}")
}

pub fn add_bates_numbers(
    path: &Path,
    start_page: u32,
    end_page: u32,
    prefix: &str,
    start_number: u64,
    digits: usize,
    position: &str,
) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let prefix = prefix.to_string();
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let (vx, vy) = viewer_point_for_stamp(position);
    for (counter, page_index) in (start_number..).zip(start_page..=end_page) {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let font_name = ensure_helvetica_font(&mut doc, page_id)?;
        let (px, py) = viewer_point_to_pdf(&doc, page_id, vx, vy)?;
        let label = bates_label(&prefix, counter, digits);
        let ops = build_page_number_ops(&font_name, &label, px, py, 12.0);
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn add_page_numbers(path: &Path, start_page: u32, end_page: u32, prefix: Option<String>) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let prefix = prefix.unwrap_or_default();
    let stamped = stamp_page_range(&mut doc, start_page, end_page, "footer-center", |page_index| {
        format!("{prefix}{}", page_index + 1)
    })?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(stamped)
}

pub fn build_watermark_ops(font_name: &str, text: &str, cx: f64, cy: f64) -> String {
    let escaped = escape_pdf_literal_string(text);
    format!(
        "\nq 0.35 g BT /{font_name} 42 Tf 0.7071 0.7071 -0.7071 0.7071 {cx} {cy} Tm ({escaped}) Tj ET Q\n",
        font_name = font_name,
        cx = cx,
        cy = cy,
        escaped = escaped
    )
}

pub fn add_text_watermark(path: &Path, text: &str, start_page: u32, end_page: u32) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Watermark text cannot be empty".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let mut stamped = 0u32;
    for page_index in start_page..=end_page {
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

pub fn flatten_annotations(path: &Path, start_page: u32, end_page: u32) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let removed = flatten_annotations_in_doc(&mut doc, start_page, end_page)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(removed)
}

pub(crate) fn flatten_annotations_in_doc(doc: &mut Document, start_page: u32, end_page: u32) -> Result<u32, String> {
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let mut removed = 0u32;
    for page_index in start_page..=end_page {
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
    Ok(removed)
}

pub(crate) fn flatten_all_annotations_in_doc(doc: &mut Document) -> Result<u32, String> {
    let total = doc.get_pages().len() as u32;
    if total == 0 {
        return Ok(0);
    }
    flatten_annotations_in_doc(doc, 0, total - 1)
}

pub fn add_page_header(path: &Path, start_page: u32, end_page: u32, text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Header text cannot be empty".to_string());
    }
    let label = trimmed.to_string();
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let stamped = stamp_page_range(&mut doc, start_page, end_page, "header-center", |_| label.clone())?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(stamped)
}

pub fn add_page_footer(path: &Path, start_page: u32, end_page: u32, text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Footer text cannot be empty".to_string());
    }
    let label = trimmed.to_string();
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let stamped = stamp_page_range(&mut doc, start_page, end_page, "footer-center", |_| label.clone())?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(stamped)
}

pub fn build_page_border_ops(doc: &Document, page_id: ObjectId, inset: f64) -> Result<String, String> {
    let media = page_media_box(doc, page_id)?;
    let mw = media[2] - media[0];
    let mh = media[3] - media[1];
    if mw <= 0.0 || mh <= 0.0 {
        return Err("Invalid page size".to_string());
    }
    let pad_x = inset * mw / VIEWER_PAGE_W;
    let pad_y = inset * mh / VIEWER_PAGE_H;
    let x = media[0] + pad_x;
    let y = media[1] + pad_y;
    let w = mw - 2.0 * pad_x;
    let h = mh - 2.0 * pad_y;
    if w <= 0.0 || h <= 0.0 {
        return Err("Border inset is too large".to_string());
    }
    Ok(format!("\nq 1 w 0 0 0 RG {x} {y} {w} {h} re S Q\n", x = x, y = y, w = w, h = h))
}

pub fn add_page_border(path: &Path, start_page: u32, end_page: u32, inset: f64) -> Result<u32, String> {
    if inset < 0.0 {
        return Err("Inset must be non-negative".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let mut bordered = 0u32;
    for page_index in start_page..=end_page {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let ops = build_page_border_ops(&doc, page_id, inset)?;
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        bordered += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(bordered)
}

pub fn add_page_header_by_parity(path: &Path, odd: bool, text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Header text cannot be empty".to_string());
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
        let (px, py) = viewer_point_to_pdf(&doc, page_id, 380.0, 40.0)?;
        let ops = build_page_number_ops(&font_name, trimmed, px, py, 12.0);
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        stamped += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(stamped)
}

pub fn add_page_footer_by_parity(path: &Path, odd: bool, text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Footer text cannot be empty".to_string());
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
        let (px, py) = viewer_point_to_pdf(&doc, page_id, 380.0, 1100.0)?;
        let ops = build_page_number_ops(&font_name, trimmed, px, py, 12.0);
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        stamped += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(stamped)
}

pub fn add_page_border_by_parity(path: &Path, odd: bool, inset: f64) -> Result<u32, String> {
    if inset < 0.0 {
        return Err("Inset must be non-negative".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut bordered = 0u32;
    for page_index in 0..total {
        if (page_index % 2 == 0) != odd {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let ops = build_page_border_ops(&doc, page_id, inset)?;
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        bordered += 1;
    }
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(bordered)
}

#[cfg(test)]
mod bates_tests {
    use super::bates_label;

    #[test]
    fn bates_label_caps_width_to_prevent_oom() {
        // Hostile width must not allocate gigabytes.
        let s = bates_label("PFX-", 1, usize::MAX);
        assert!(s.len() <= 24, "expected capped width, got len {}", s.len());
        // Normal padding still works.
        assert_eq!(bates_label("A", 7, 4), "A0007");
    }
}
