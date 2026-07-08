use crate::pdf::annotations::{annot_is_redaction, annot_panda_stamp_kind, append_page_annotation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use serde::Serialize;
use std::path::Path;

pub fn ink_bbox(points: &[f64]) -> [f64; 4] {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for chunk in points.chunks(2) {
        if chunk.len() == 2 {
            min_x = min_x.min(chunk[0]);
            min_y = min_y.min(chunk[1]);
            max_x = max_x.max(chunk[0]);
            max_y = max_y.max(chunk[1]);
        }
    }
    [min_x, min_y, max_x, max_y]
}

pub fn add_ink_stroke(path: &Path, page_index: u32, points: Vec<f64>) -> Result<(), String> {
    if points.len() < 4 || !points.len().is_multiple_of(2) {
        return Err("Ink stroke needs at least two points".to_string());
    }
    for (i, v) in points.iter().enumerate() {
        crate::pdf::coords::finite_f64(*v, &format!("point[{i}]"))?;
    }

    let mut doc = Document::load(path).map_err(|e| e.to_string())?;

    let pages = doc.get_pages();
    let page_id = pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let bbox = ink_bbox(&points);
    let ink_coords: Vec<Object> = points.iter().map(|p| Object::Real(*p as f32)).collect();
    let annot = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"Ink".to_vec())),
        (
            b"Rect".to_vec(),
            Object::Array(vec![
                Object::Real(bbox[0] as f32),
                Object::Real(bbox[1] as f32),
                Object::Real(bbox[2] as f32),
                Object::Real(bbox[3] as f32),
            ]),
        ),
        (b"InkList".to_vec(), Object::Array(vec![Object::Array(ink_coords)])),
        (b"C".to_vec(), Object::Array(vec![Object::Real(0.0), Object::Real(0.0), Object::Real(1.0)])),
    ])));

    let annots = doc.get_dictionary_mut(*page_id).map_err(|e| e.to_string())?.get_mut(b"Annots");

    match annots {
        Ok(Object::Array(ref mut arr)) => {
            arr.push(Object::Reference(annot));
        }
        _ => {
            doc.get_dictionary_mut(*page_id)
                .map_err(|e| e.to_string())?
                .set(b"Annots", Object::Array(vec![Object::Reference(annot)]));
        }
    }

    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

/// Remove the `index`-th ink annotation (0-based among `Ink` subtypes).
pub fn remove_ink_stroke(path: &Path, page_index: u32, index: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;

    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let annots = match doc.get_dictionary(page_id).map_err(|e| e.to_string())?.get(b"Annots") {
        Ok(Object::Array(arr)) => arr.clone(),
        _ => return Err("No annotations on this page".to_string()),
    };

    let mut ink_count = 0u32;
    let mut target_pos: Option<usize> = None;
    for (pos, annot_ref) in annots.iter().enumerate() {
        let Object::Reference(id) = annot_ref else {
            continue;
        };
        let is_ink = doc
            .get_object(*id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(b"Subtype").ok())
            .and_then(|o| o.as_name().ok())
            .map(|n| String::from_utf8_lossy(n) == "Ink")
            .unwrap_or(false);
        if is_ink {
            if ink_count == index {
                target_pos = Some(pos);
                break;
            }
            ink_count += 1;
        }
    }

    let pos = target_pos.ok_or("Ink stroke not found".to_string())?;
    let mut new_annots = annots;
    new_annots.remove(pos);
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(b"Annots", Object::Array(new_annots));

    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn shape_rect_object(x1: f64, y1: f64, x2: f64, y2: f64) -> Object {
    Object::Array(vec![
        Object::Real(x1 as f32),
        Object::Real(y1 as f32),
        Object::Real(x2 as f32),
        Object::Real(y2 as f32),
    ])
}

pub fn shape_outline_fields(x1: f64, y1: f64, x2: f64, y2: f64) -> Vec<(Vec<u8>, Object)> {
    vec![
        (b"Rect".to_vec(), shape_rect_object(x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2))),
        (b"C".to_vec(), Object::Array(vec![Object::Real(1.0), Object::Real(0.0), Object::Real(0.0)])),
        (b"Border".to_vec(), Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Real(2.0)])),
    ]
}

pub fn remove_annotation_by_subtype(
    doc: &mut Document,
    page_id: ObjectId,
    subtype: &str,
    index: u32,
    not_found_msg: &str,
) -> Result<(), String> {
    let annots = match doc.get_dictionary(page_id).map_err(|e| e.to_string())?.get(b"Annots") {
        Ok(Object::Array(arr)) => arr.clone(),
        _ => return Err("No annotations on this page".to_string()),
    };

    let mut match_count = 0u32;
    let mut target_pos: Option<usize> = None;
    for (pos, annot_ref) in annots.iter().enumerate() {
        let Object::Reference(id) = annot_ref else {
            continue;
        };
        let matches = doc
            .get_object(*id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(b"Subtype").ok())
            .and_then(|o| o.as_name().ok())
            .map(|n| String::from_utf8_lossy(n) == subtype)
            .unwrap_or(false);
        if matches {
            if match_count == index {
                target_pos = Some(pos);
                break;
            }
            match_count += 1;
        }
    }

    let pos = target_pos.ok_or_else(|| not_found_msg.to_string())?;
    let mut new_annots = annots;
    new_annots.remove(pos);
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(b"Annots", Object::Array(new_annots));
    Ok(())
}

pub fn add_square(path: &Path, page_index: u32, x1: f64, y1: f64, x2: f64, y2: f64) -> Result<(), String> {
    for (name, v) in [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)] {
        crate::pdf::coords::finite_f64(v, name)?;
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let mut fields = vec![
        (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"Square".to_vec())),
    ];
    fields.extend(shape_outline_fields(x1, y1, x2, y2));
    let annot = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(fields)));
    append_page_annotation(&mut doc, page_id, annot)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn add_circle(path: &Path, page_index: u32, x1: f64, y1: f64, x2: f64, y2: f64) -> Result<(), String> {
    for (name, v) in [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)] {
        crate::pdf::coords::finite_f64(v, name)?;
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let mut fields = vec![
        (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"Circle".to_vec())),
    ];
    fields.extend(shape_outline_fields(x1, y1, x2, y2));
    let annot = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(fields)));
    append_page_annotation(&mut doc, page_id, annot)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn add_line(path: &Path, page_index: u32, x1: f64, y1: f64, x2: f64, y2: f64) -> Result<(), String> {
    for (name, v) in [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)] {
        crate::pdf::coords::finite_f64(v, name)?;
    }
    if (x2 - x1).hypot(y2 - y1) < 5.0 {
        return Err("Line is too short".to_string());
    }

    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let mut fields = vec![
        (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"Line".to_vec())),
        (
            b"L".to_vec(),
            Object::Array(vec![
                Object::Real(x1 as f32),
                Object::Real(y1 as f32),
                Object::Real(x2 as f32),
                Object::Real(y2 as f32),
            ]),
        ),
    ];
    fields.extend(shape_outline_fields(x1, y1, x2, y2));
    let annot = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(fields)));
    append_page_annotation(&mut doc, page_id, annot)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn remove_square(path: &Path, page_index: u32, index: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    remove_annotation_by_subtype(&mut doc, page_id, "Square", index, "Square shape not found")?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn remove_circle(path: &Path, page_index: u32, index: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    remove_annotation_by_subtype(&mut doc, page_id, "Circle", index, "Circle shape not found")?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn remove_line(path: &Path, page_index: u32, index: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    remove_annotation_by_subtype(&mut doc, page_id, "Line", index, "Line shape not found")?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

const STAMP_PRESETS: &[(&str, &str)] =
    &[("approved", "APPROVED"), ("draft", "DRAFT"), ("confidential", "CONFIDENTIAL"), ("reviewed", "REVIEWED")];

const TEXT_STAMP_WIDTH: f64 = 132.0;
const TEXT_STAMP_HEIGHT: f64 = 32.0;
const IMAGE_STAMP_SIZE: f64 = 72.0;

#[derive(Serialize)]
pub struct StampPresetInfo {
    pub id: String,
    pub label: String,
    pub color: [u8; 3],
}

pub fn stamp_preset_label(preset: &str) -> Result<&'static str, String> {
    STAMP_PRESETS
        .iter()
        .find(|(id, _)| *id == preset)
        .map(|(_, label)| *label)
        .ok_or_else(|| format!("Unknown stamp preset: {preset}"))
}

pub fn stamp_preset_color(preset: &str) -> [u8; 3] {
    match preset {
        "approved" => [34, 139, 34],
        "draft" => [120, 120, 120],
        "confidential" => [178, 34, 34],
        "reviewed" => [30, 90, 160],
        _ => [100, 100, 100],
    }
}

pub fn stamp_text_default_appearance(preset: &str) -> &'static str {
    match preset {
        "approved" => "/Helvetica-Bold 14 Tf 0.0 0.55 0.0 rg",
        "draft" => "/Helvetica-Bold 14 Tf 0.35 0.35 0.35 rg",
        "confidential" => "/Helvetica-Bold 14 Tf 0.7 0.1 0.1 rg",
        "reviewed" => "/Helvetica-Bold 14 Tf 0.12 0.35 0.63 rg",
        _ => "/Helvetica-Bold 14 Tf 0.0 0.0 0.0 rg",
    }
}

pub fn remove_panda_stamp(doc: &mut Document, page_id: ObjectId, kind: &str, index: u32) -> Result<(), String> {
    let annots = match doc.get_dictionary(page_id).map_err(|e| e.to_string())?.get(b"Annots") {
        Ok(Object::Array(arr)) => arr.clone(),
        _ => return Err("No annotations on this page".to_string()),
    };

    let mut match_count = 0u32;
    let mut target_pos: Option<usize> = None;
    for (pos, annot_ref) in annots.iter().enumerate() {
        let Object::Reference(id) = annot_ref else {
            continue;
        };
        let is_match = doc
            .get_object(*id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(annot_panda_stamp_kind)
            .map(|k| k == kind)
            .unwrap_or(false);
        if is_match {
            if match_count == index {
                target_pos = Some(pos);
                break;
            }
            match_count += 1;
        }
    }

    let pos = target_pos.ok_or_else(|| format!("{kind} stamp not found"))?;
    let mut new_annots = annots;
    new_annots.remove(pos);
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(b"Annots", Object::Array(new_annots));
    Ok(())
}

pub fn list_stamp_presets() -> Vec<StampPresetInfo> {
    STAMP_PRESETS
        .iter()
        .map(|(id, label)| StampPresetInfo {
            id: (*id).to_string(),
            label: (*label).to_string(),
            color: stamp_preset_color(id),
        })
        .collect()
}

pub fn add_text_stamp(path: &Path, page_index: u32, x: f64, y: f64, preset: String) -> Result<(), String> {
    crate::pdf::coords::finite_f64(x, "x")?;
    crate::pdf::coords::finite_f64(y, "y")?;
    let label = stamp_preset_label(&preset)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let x2 = x + TEXT_STAMP_WIDTH;
    let y2 = y + TEXT_STAMP_HEIGHT;
    let annot = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"FreeText".to_vec())),
        (b"Rect".to_vec(), shape_rect_object(x, y, x2, y2)),
        (b"Contents".to_vec(), Object::String(label.as_bytes().to_vec(), lopdf::StringFormat::Literal)),
        (
            b"DA".to_vec(),
            Object::String(stamp_text_default_appearance(&preset).as_bytes().to_vec(), lopdf::StringFormat::Literal),
        ),
        (b"F".to_vec(), Object::Integer(4)),
        (b"PandaStamp".to_vec(), Object::Name(preset.as_bytes().to_vec())),
        (b"PandaStampKind".to_vec(), Object::Name(b"text".to_vec())),
    ])));
    append_page_annotation(&mut doc, page_id, annot)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn embed_stamp_image_xobject(doc: &mut Document, preset: &str) -> Result<ObjectId, String> {
    let (r, g, b) = {
        let c = stamp_preset_color(preset);
        (c[0], c[1], c[2])
    };
    let width = 72u32;
    let height = 72u32;
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for py in 0..height {
        for px in 0..width {
            let edge = px < 2 || py < 2 || px >= width - 2 || py >= height - 2;
            let (pr, pg, pb) =
                if edge { (r.saturating_sub(40), g.saturating_sub(40), b.saturating_sub(40)) } else { (r, g, b) };
            rgb.extend_from_slice(&[pr, pg, pb]);
        }
    }
    let img_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Image".to_vec())),
            (b"Width".to_vec(), Object::Integer(width as i64)),
            (b"Height".to_vec(), Object::Integer(height as i64)),
            (b"ColorSpace".to_vec(), Object::Name(b"DeviceRGB".to_vec())),
            (b"BitsPerComponent".to_vec(), Object::Integer(8)),
        ]),
        rgb,
    )));
    Ok(img_id)
}

pub fn add_image_stamp(path: &Path, page_index: u32, x: f64, y: f64, preset: String) -> Result<(), String> {
    crate::pdf::coords::finite_f64(x, "x")?;
    crate::pdf::coords::finite_f64(y, "y")?;
    stamp_preset_label(&preset)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let img_id = embed_stamp_image_xobject(&mut doc, &preset)?;
    let width = IMAGE_STAMP_SIZE;
    let height = IMAGE_STAMP_SIZE;
    let mut xobject_dict = Dictionary::new();
    xobject_dict.set(b"Im1", Object::Reference(img_id));
    let mut resources = Dictionary::new();
    resources.set(b"XObject", Object::Dictionary(xobject_dict));
    let form_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Form".to_vec())),
            (
                b"BBox".to_vec(),
                Object::Array(vec![
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Real(width as f32),
                    Object::Real(height as f32),
                ]),
            ),
            (b"Resources".to_vec(), Object::Dictionary(resources)),
        ]),
        format!("q {width} 0 0 {height} 0 0 cm /Im1 Do Q\n").into_bytes(),
    )));
    let ap = Dictionary::from_iter(vec![(b"N".to_vec(), Object::Reference(form_id))]);
    let x2 = x + width;
    let y2 = y + height;
    let annot = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"Stamp".to_vec())),
        (b"Rect".to_vec(), shape_rect_object(x, y, x2, y2)),
        (b"AP".to_vec(), Object::Dictionary(ap)),
        (b"PandaStamp".to_vec(), Object::Name(preset.as_bytes().to_vec())),
        (b"PandaStampKind".to_vec(), Object::Name(b"image".to_vec())),
    ])));
    append_page_annotation(&mut doc, page_id, annot)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn remove_text_stamp(path: &Path, page_index: u32, index: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    remove_panda_stamp(&mut doc, page_id, "text", index)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn remove_image_stamp(path: &Path, page_index: u32, index: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    remove_panda_stamp(&mut doc, page_id, "image", index)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn remove_redaction_at_index(doc: &mut Document, page_id: ObjectId, index: u32) -> Result<(), String> {
    let annots = match doc.get_dictionary(page_id).map_err(|e| e.to_string())?.get(b"Annots") {
        Ok(Object::Array(arr)) => arr.clone(),
        _ => return Err("No annotations on this page".to_string()),
    };

    let mut redaction_count = 0u32;
    let mut target_pos: Option<usize> = None;
    for (pos, annot_ref) in annots.iter().enumerate() {
        let Object::Reference(id) = annot_ref else {
            continue;
        };
        let is_redaction =
            doc.get_object(*id).ok().and_then(|o| o.as_dict().ok()).map(annot_is_redaction).unwrap_or(false);
        if is_redaction {
            if redaction_count == index {
                target_pos = Some(pos);
                break;
            }
            redaction_count += 1;
        }
    }

    let pos = target_pos.ok_or("Redaction not found".to_string())?;
    let mut new_annots = annots;
    new_annots.remove(pos);
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(b"Annots", Object::Array(new_annots));
    Ok(())
}

pub fn add_redaction(path: &Path, page_index: u32, x1: f64, y1: f64, x2: f64, y2: f64) -> Result<(), String> {
    for (name, v) in [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)] {
        crate::pdf::coords::finite_f64(v, name)?;
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let annot = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"Square".to_vec())),
        (b"Rect".to_vec(), shape_rect_object(x1, y1, x2, y2)),
        (b"C".to_vec(), Object::Array(vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)])),
        (b"IC".to_vec(), Object::Array(vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)])),
        (b"Border".to_vec(), Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Real(0.0)])),
        (b"PandaRedact".to_vec(), Object::Boolean(true)),
    ])));
    append_page_annotation(&mut doc, page_id, annot)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn remove_redaction(path: &Path, page_index: u32, index: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    remove_redaction_at_index(&mut doc, page_id, index)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}
