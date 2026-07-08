use crate::pdf::coords::obj_to_f64;
use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::Serialize;
use std::path::Path;

const TEXT_NOTE_WIDTH: f64 = 140.0;
const TEXT_NOTE_HEIGHT: f64 = 80.0;

pub fn append_page_annotation(doc: &mut Document, page_id: ObjectId, annot_id: ObjectId) -> Result<(), String> {
    let page_dict = doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?;
    match page_dict.get_mut(b"Annots") {
        Ok(Object::Array(arr)) => arr.push(Object::Reference(annot_id)),
        _ => page_dict.set(b"Annots", Object::Array(vec![Object::Reference(annot_id)])),
    }
    Ok(())
}

pub fn add_highlight(path: &Path, page_index: u32, x1: f64, y1: f64, x2: f64, y2: f64) -> Result<(), String> {
    for (name, v) in [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)] {
        crate::pdf::coords::finite_f64(v, name)?;
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;

    let pages = doc.get_pages();
    let page_id = pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let annot = doc.add_object(Object::Dictionary(lopdf::Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"Highlight".to_vec())),
        (
            b"Rect".to_vec(),
            Object::Array(vec![
                Object::Real(x1 as f32),
                Object::Real(y1 as f32),
                Object::Real(x2 as f32),
                Object::Real(y2 as f32),
            ]),
        ),
        (
            b"QuadPoints".to_vec(),
            Object::Array(vec![
                Object::Real(x1 as f32),
                Object::Real(y2 as f32),
                Object::Real(x2 as f32),
                Object::Real(y2 as f32),
                Object::Real(x1 as f32),
                Object::Real(y1 as f32),
                Object::Real(x2 as f32),
                Object::Real(y1 as f32),
            ]),
        ),
        (b"C".to_vec(), Object::Array(vec![Object::Real(1.0), Object::Real(1.0), Object::Real(0.0)])),
    ])));

    append_page_annotation(&mut doc, *page_id, annot)?;

    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

/// Remove the `index`-th highlight annotation (0-based, in document order) from a
/// page. The index matches the order highlights are returned by
/// `get_annotations` after filtering to the `Highlight` subtype.
pub fn remove_highlight(path: &Path, page_index: u32, index: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;

    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let annots = match doc.get_dictionary(page_id).map_err(|e| e.to_string())?.get(b"Annots") {
        Ok(Object::Array(arr)) => arr.clone(),
        _ => return Err("No annotations on this page".to_string()),
    };

    let mut highlight_count = 0u32;
    let mut target_pos: Option<usize> = None;
    for (pos, annot_ref) in annots.iter().enumerate() {
        let Object::Reference(id) = annot_ref else {
            continue;
        };
        let is_highlight = doc
            .get_object(*id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(b"Subtype").ok())
            .and_then(|o| o.as_name().ok())
            .map(|n| String::from_utf8_lossy(n) == "Highlight")
            .unwrap_or(false);
        if is_highlight {
            if highlight_count == index {
                target_pos = Some(pos);
                break;
            }
            highlight_count += 1;
        }
    }

    let pos = target_pos.ok_or("Highlight not found".to_string())?;
    let mut new_annots = annots;
    new_annots.remove(pos);
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(b"Annots", Object::Array(new_annots));

    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn add_text_note(path: &Path, page_index: u32, x: f64, y: f64, content: String) -> Result<(), String> {
    crate::pdf::coords::finite_f64(x, "x")?;
    crate::pdf::coords::finite_f64(y, "y")?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;

    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let x2 = x + TEXT_NOTE_WIDTH;
    let y2 = y + TEXT_NOTE_HEIGHT;
    let annot = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"Text".to_vec())),
        (
            b"Rect".to_vec(),
            Object::Array(vec![
                Object::Real(x as f32),
                Object::Real(y as f32),
                Object::Real(x2 as f32),
                Object::Real(y2 as f32),
            ]),
        ),
        (b"Contents".to_vec(), Object::String(content.into_bytes(), lopdf::StringFormat::Literal)),
        (b"Open".to_vec(), Object::Boolean(false)),
        (b"C".to_vec(), Object::Array(vec![Object::Real(1.0), Object::Real(1.0), Object::Real(0.6)])),
    ])));

    append_page_annotation(&mut doc, page_id, annot)?;

    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

/// Remove the `index`-th text-note annotation (0-based among `Text` subtypes).
pub fn remove_text_note(path: &Path, page_index: u32, index: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;

    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let annots = match doc.get_dictionary(page_id).map_err(|e| e.to_string())?.get(b"Annots") {
        Ok(Object::Array(arr)) => arr.clone(),
        _ => return Err("No annotations on this page".to_string()),
    };

    let mut note_count = 0u32;
    let mut target_pos: Option<usize> = None;
    for (pos, annot_ref) in annots.iter().enumerate() {
        let Object::Reference(id) = annot_ref else {
            continue;
        };
        let is_text = doc
            .get_object(*id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(b"Subtype").ok())
            .and_then(|o| o.as_name().ok())
            .map(|n| String::from_utf8_lossy(n) == "Text")
            .unwrap_or(false);
        if is_text {
            if note_count == index {
                target_pos = Some(pos);
                break;
            }
            note_count += 1;
        }
    }

    let pos = target_pos.ok_or("Text note not found".to_string())?;
    let mut new_annots = annots;
    new_annots.remove(pos);
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(b"Annots", Object::Array(new_annots));

    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

#[derive(Serialize)]
pub struct AnnotationData {
    pub subtype: String,
    pub rect: [f64; 4],
    pub color: Option<[f64; 3]>,
    pub contents: Option<String>,
    pub ink_points: Option<Vec<f64>>,
    pub line_endpoints: Option<[f64; 4]>,
    pub stamp_kind: Option<String>,
    pub stamp_preset: Option<String>,
    pub is_redaction: bool,
}

fn annot_contents(dict: &Dictionary) -> Option<String> {
    dict.get(b"Contents").ok().and_then(|o| match o {
        Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).to_string()),
        _ => None,
    })
}

pub(crate) fn annot_panda_stamp(dict: &Dictionary) -> Option<String> {
    dict.get(b"PandaStamp").ok().and_then(|o| o.as_name().ok()).map(|b| String::from_utf8_lossy(b).to_string())
}

pub(crate) fn annot_panda_stamp_kind(dict: &Dictionary) -> Option<String> {
    dict.get(b"PandaStampKind").ok().and_then(|o| o.as_name().ok()).map(|b| String::from_utf8_lossy(b).to_string())
}

pub(crate) fn annot_is_redaction(dict: &Dictionary) -> bool {
    dict.get(b"PandaRedact").ok().and_then(|o| o.as_bool().ok()).unwrap_or(false)
}

pub(crate) fn redaction_rects_on_page(doc: &Document, page_id: ObjectId) -> Vec<[f64; 4]> {
    let Ok(page_dict) = doc.get_dictionary(page_id) else {
        return Vec::new();
    };
    let Ok(Object::Array(arr)) = page_dict.get(b"Annots") else {
        return Vec::new();
    };
    let mut rects = Vec::new();
    for annot_ref in arr {
        let Object::Reference(id) = annot_ref else {
            continue;
        };
        let Some(annot_dict) = doc.get_object(*id).ok().and_then(|o| o.as_dict().ok()) else {
            continue;
        };
        if !annot_is_redaction(annot_dict) {
            continue;
        }
        if let Ok(Object::Array(rect_arr)) = annot_dict.get(b"Rect") {
            let get = |i: usize| rect_arr.get(i).map(obj_to_f64).unwrap_or(0.0);
            rects.push([get(0), get(1), get(2), get(3)]);
        }
    }
    rects
}

fn parse_annotation_dict(annot_dict: &Dictionary) -> AnnotationData {
    let subtype = annot_dict
        .get(b"Subtype")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default();

    let rect = if let Ok(Object::Array(rect_arr)) = annot_dict.get(b"Rect") {
        let get = |i: usize| rect_arr.get(i).map(obj_to_f64).unwrap_or(0.0);
        [get(0), get(1), get(2), get(3)]
    } else {
        [0.0; 4]
    };

    let color = annot_dict.get(b"C").ok().and_then(|o| {
        o.as_array().ok().map(|arr| {
            let get = |i: usize| arr.get(i).map(obj_to_f64).unwrap_or(0.0);
            [get(0), get(1), get(2)]
        })
    });

    let contents = annot_contents(annot_dict);
    let ink_points = if subtype == "Ink" {
        annot_dict.get(b"InkList").ok().and_then(|o| o.as_array().ok()).and_then(|strokes| {
            strokes
                .first()
                .and_then(|stroke| stroke.as_array().ok())
                .map(|coords| coords.iter().map(obj_to_f64).collect::<Vec<_>>())
        })
    } else {
        None
    };
    let line_endpoints = if subtype == "Line" {
        annot_dict.get(b"L").ok().and_then(|o| o.as_array().ok()).map(|arr| {
            let get = |i: usize| arr.get(i).map(obj_to_f64).unwrap_or(0.0);
            [get(0), get(1), get(2), get(3)]
        })
    } else {
        None
    };
    let stamp_kind = annot_panda_stamp_kind(annot_dict);
    let stamp_preset = annot_panda_stamp(annot_dict);
    let is_redaction = annot_is_redaction(annot_dict);
    AnnotationData {
        subtype,
        rect,
        color,
        contents,
        ink_points,
        line_endpoints,
        stamp_kind,
        stamp_preset,
        is_redaction,
    }
}

fn walk_page_annotations(doc: &Document, page_id: ObjectId) -> Result<Vec<AnnotationData>, String> {
    let page_dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    if let Ok(Object::Array(arr)) = page_dict.get(b"Annots") {
        for annot_ref in arr {
            let id = match annot_ref {
                Object::Reference(id) => *id,
                _ => continue,
            };
            if let Ok(annot_obj) = doc.get_object(id) {
                if let Ok(annot_dict) = annot_obj.as_dict() {
                    result.push(parse_annotation_dict(annot_dict));
                }
            }
        }
    }
    Ok(result)
}

pub fn get_annotations(path: &Path, page_index: u32) -> Result<Vec<AnnotationData>, String> {
    let doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    walk_page_annotations(&doc, page_id)
}

#[derive(Serialize)]
pub struct DocAnnotation {
    pub page_index: u32,
    pub index: u32,
    pub data: AnnotationData,
}

pub fn list_document_annotations(path: &Path) -> Result<Vec<DocAnnotation>, String> {
    let doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut page_nums: Vec<u32> = doc.get_pages().keys().copied().collect();
    page_nums.sort_unstable();
    let mut out = Vec::new();
    for page_num in page_nums {
        let page_index = page_num.saturating_sub(1);
        let page_id = *doc.get_pages().get(&page_num).ok_or("Page not found".to_string())?;
        let annots = walk_page_annotations(&doc, page_id)?;
        let mut highlight_idx = 0u32;
        let mut note_idx = 0u32;
        let mut redact_idx = 0u32;
        for data in annots {
            let index = if data.is_redaction {
                let i = redact_idx;
                redact_idx += 1;
                i
            } else if data.subtype == "Highlight" {
                let i = highlight_idx;
                highlight_idx += 1;
                i
            } else if data.subtype == "Text" {
                let i = note_idx;
                note_idx += 1;
                i
            } else {
                0
            };
            out.push(DocAnnotation { page_index, index, data });
        }
    }
    Ok(out)
}
