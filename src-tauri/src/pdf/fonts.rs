use std::collections::BTreeMap;

use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

static EMBEDDED_FONT_BYTES: &[u8] = include_bytes!("../../vendor/fonts/LiberationSans-Regular.ttf");
static FONT_RESOURCE_NAME: &str = "PPFullFont";
static FONT_BASE_NAME: &str = "LiberationSans";

/// Check whether every character in `text` has a glyph in the bundled font.
pub fn font_has_glyphs_for(text: &str) -> bool {
    let Ok(face) = ttf_parser::Face::parse(EMBEDDED_FONT_BYTES, 0) else {
        return false;
    };
    for ch in text.chars() {
        if face.glyph_index(ch).is_none() {
            return false;
        }
    }
    true
}

/// Ensure the page (and document) has the embedded full font available.
/// Returns the resource name to use in text operators (e.g. "/PPFullFont").
pub fn ensure_full_font(doc: &mut Document, page_id: ObjectId) -> Result<String, String> {
    // Check whether the font is already embedded in this document.
    if let Some(existing_id) = find_embedded_font_id(doc) {
        // Ensure the page's Resources / Font dict references it.
        add_font_to_page_resources(doc, page_id, existing_id)?;
        return Ok(FONT_RESOURCE_NAME.to_string());
    }

    let face =
        ttf_parser::Face::parse(EMBEDDED_FONT_BYTES, 0).map_err(|e| format!("Failed to parse embedded font: {e:?}"))?;

    let bbox = face.global_bounding_box();
    let ascent = face.ascender();
    let descent = face.descender();
    let cap_height = face.capital_height().unwrap_or(ascent);
    let stem_v = ((bbox.x_max - bbox.x_min) as f64 * 0.13).round() as i64;

    // Font stream
    let font_stream = Stream::new(
        Dictionary::from_iter(vec![(b"Length1".to_vec(), Object::Integer(EMBEDDED_FONT_BYTES.len() as i64))]),
        EMBEDDED_FONT_BYTES.to_vec(),
    );
    let font_file_id = doc.add_object(Object::Stream(font_stream));

    // Font descriptor
    let font_descriptor = Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"FontDescriptor".to_vec())),
        (b"FontName".to_vec(), Object::Name(FONT_BASE_NAME.as_bytes().to_vec())),
        (b"Flags".to_vec(), Object::Integer(32)),
        (
            b"FontBBox".to_vec(),
            Object::Array(vec![
                Object::Integer(bbox.x_min as i64),
                Object::Integer(bbox.y_min as i64),
                Object::Integer(bbox.x_max as i64),
                Object::Integer(bbox.y_max as i64),
            ]),
        ),
        (b"ItalicAngle".to_vec(), Object::Integer(0)),
        (b"Ascent".to_vec(), Object::Integer(ascent as i64)),
        (b"Descent".to_vec(), Object::Integer(descent as i64)),
        (b"CapHeight".to_vec(), Object::Integer(cap_height as i64)),
        (b"StemV".to_vec(), Object::Integer(stem_v)),
        (b"FontFile2".to_vec(), Object::Reference(font_file_id)),
    ]);
    let font_descriptor_id = doc.add_object(Object::Dictionary(font_descriptor));

    // Font dictionary
    let font_dict = Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Font".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"TrueType".to_vec())),
        (b"BaseFont".to_vec(), Object::Name(FONT_BASE_NAME.as_bytes().to_vec())),
        (b"FontDescriptor".to_vec(), Object::Reference(font_descriptor_id)),
        (b"Encoding".to_vec(), Object::Name(b"WinAnsiEncoding".to_vec())),
    ]);
    let font_id = doc.add_object(Object::Dictionary(font_dict));

    add_font_to_page_resources(doc, page_id, font_id)?;
    Ok(FONT_RESOURCE_NAME.to_string())
}

fn find_embedded_font_id(doc: &Document) -> Option<ObjectId> {
    for (id, obj) in &doc.objects {
        let Ok(dict) = obj.as_dict() else { continue };
        if dict.get(b"Type").ok()?.as_name().ok()? != b"Font" {
            continue;
        }
        if dict.get(b"Subtype").ok()?.as_name().ok()? != b"TrueType" {
            continue;
        }
        if let Ok(base) = dict.get(b"BaseFont").ok()?.as_name() {
            if base == FONT_BASE_NAME.as_bytes() {
                return Some(*id);
            }
        }
    }
    None
}

fn add_font_to_page_resources(doc: &mut Document, page_id: ObjectId, font_id: ObjectId) -> Result<(), String> {
    let mut resources = page_resources_for_edit(doc, page_id);
    let mut fonts =
        resources.get(b"Font").ok().and_then(|obj| dictionary_object_to_owned(doc, obj)).unwrap_or_default();
    fonts.set(FONT_RESOURCE_NAME.as_bytes(), Object::Reference(font_id));
    resources.set(b"Font", Object::Dictionary(fonts));
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(b"Resources", Object::Dictionary(resources));
    Ok(())
}

fn page_resources_for_edit(doc: &Document, page_id: ObjectId) -> Dictionary {
    if let Ok(page_dict) = doc.get_dictionary(page_id) {
        if let Ok(resources) = page_dict.get(b"Resources") {
            if let Some(dict) = dictionary_object_to_owned(doc, resources) {
                return dict;
            }
        }
    }
    crate::pdf::markdown_images::resolve_page_resources(doc, page_id).unwrap_or_default()
}

fn dictionary_object_to_owned(doc: &Document, obj: &Object) -> Option<Dictionary> {
    match obj {
        Object::Dictionary(dict) => Some(dict.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    }
}

pub fn page_font_entries(doc: &Document, page_id: ObjectId) -> Vec<(Vec<u8>, ObjectId)> {
    let mut out = Vec::new();
    let Ok(page) = doc.get_dictionary(page_id) else { return out };
    let Ok(resources) = page.get(b"Resources").and_then(|o| o.as_dict()) else { return out };
    let Ok(fonts) = resources.get(b"Font").and_then(|o| o.as_dict()) else { return out };
    for (name, obj) in fonts.iter() {
        let id = match obj {
            Object::Reference(id) => *id,
            _ => continue,
        };
        out.push((name.clone(), id));
    }
    out
}

fn font_signature(doc: &Document, font_id: ObjectId) -> Option<String> {
    let dict = doc.get_dictionary(font_id).ok()?;
    let base = dict.get(b"BaseFont").ok()?.as_name().ok()?;
    let subtype = dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()).unwrap_or(b"");
    let mut sig = format!("{}:{}", String::from_utf8_lossy(subtype), String::from_utf8_lossy(base));
    if let Ok(Object::Reference(desc_id)) = dict.get(b"FontDescriptor") {
        if let Ok(Object::Dictionary(desc)) = doc.get_object(*desc_id) {
            if let Some(len) = desc.get(b"Length").ok().and_then(|o| o.as_i64().ok()) {
                sig.push_str(&format!(":len={len}"));
            }
            if let Some(name) = desc.get(b"FontName").ok().and_then(|o| o.as_name().ok()) {
                sig.push_str(&format!(":fn={}", String::from_utf8_lossy(name)));
            }
        }
    }
    Some(sig)
}

pub fn dedup_fonts_after_insert(doc: &mut Document, inserted_page_ids: &[ObjectId]) -> Result<u32, String> {
    let inserted: BTreeMap<ObjectId, ()> = inserted_page_ids.iter().copied().map(|id| (id, ())).collect();
    let mut known: BTreeMap<String, ObjectId> = BTreeMap::new();

    for &page_id in doc.get_pages().values() {
        if inserted.contains_key(&page_id) {
            continue;
        }
        for (_name, font_id) in page_font_entries(doc, page_id) {
            if let Some(sig) = font_signature(doc, font_id) {
                known.entry(sig).or_insert(font_id);
            }
        }
    }

    let mut deduped = 0u32;
    for &page_id in inserted_page_ids {
        let entries = page_font_entries(doc, page_id);
        for (res_name, font_id) in entries {
            let Some(sig) = font_signature(doc, font_id) else { continue };
            if let Some(&existing_id) = known.get(&sig) {
                if existing_id != font_id {
                    let page_dict = doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?;
                    let resources = page_dict
                        .get_mut(b"Resources")
                        .map_err(|e| e.to_string())?
                        .as_dict_mut()
                        .map_err(|_| "Bad Resources".to_string())?;
                    let fonts = resources
                        .get_mut(b"Font")
                        .map_err(|e| e.to_string())?
                        .as_dict_mut()
                        .map_err(|_| "Bad Font dict".to_string())?;
                    fonts.set(res_name, Object::Reference(existing_id));
                    deduped += 1;
                }
            } else {
                known.insert(sig, font_id);
            }
        }
    }
    Ok(deduped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_doc_with_inherited_resources() -> (Document, ObjectId) {
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let image_id = doc.add_object(Object::Stream(Stream::new(
            Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
                (b"Subtype".to_vec(), Object::Name(b"Image".to_vec())),
                (b"Width".to_vec(), Object::Integer(1)),
                (b"Height".to_vec(), Object::Integer(1)),
                (b"ColorSpace".to_vec(), Object::Name(b"DeviceRGB".to_vec())),
                (b"BitsPerComponent".to_vec(), Object::Integer(8)),
            ]),
            vec![0, 0, 0],
        )));

        let mut xobjects = Dictionary::new();
        xobjects.set(b"ImParent", Object::Reference(image_id));
        let mut resources = Dictionary::new();
        resources.set(b"XObject", Object::Dictionary(xobjects));

        doc.set_object(
            pages_id,
            Object::Dictionary(Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"Pages".to_vec())),
                (b"Kids".to_vec(), Object::Array(vec![Object::Reference(page_id)])),
                (b"Count".to_vec(), Object::Integer(1)),
                (b"Resources".to_vec(), Object::Dictionary(resources)),
            ])),
        );

        doc.set_object(
            page_id,
            Object::Dictionary(Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"Page".to_vec())),
                (b"Parent".to_vec(), Object::Reference(pages_id)),
                (
                    b"MediaBox".to_vec(),
                    Object::Array(vec![
                        Object::Integer(0),
                        Object::Integer(0),
                        Object::Integer(612),
                        Object::Integer(792),
                    ]),
                ),
            ])),
        );

        let catalog_id = doc.new_object_id();
        doc.set_object(
            catalog_id,
            Object::Dictionary(Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"Catalog".to_vec())),
                (b"Pages".to_vec(), Object::Reference(pages_id)),
            ])),
        );
        doc.trailer.set(b"Root", Object::Reference(catalog_id));
        (doc, page_id)
    }

    #[test]
    fn ensure_full_font_preserves_inherited_resources() {
        let (mut doc, page_id) = build_doc_with_inherited_resources();

        let name = ensure_full_font(&mut doc, page_id).unwrap();

        assert_eq!(name, "PPFullFont");
        let page = doc.get_dictionary(page_id).unwrap();
        let resources = page.get(b"Resources").unwrap().as_dict().unwrap();
        assert!(resources.get(b"XObject").unwrap().as_dict().unwrap().get(b"ImParent").is_ok());
        assert!(resources.get(b"Font").unwrap().as_dict().unwrap().get(b"PPFullFont").is_ok());
    }
}
