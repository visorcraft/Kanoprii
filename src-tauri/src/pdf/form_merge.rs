use std::collections::{BTreeMap, HashSet};

use lopdf::{Document, Object, ObjectId};

use crate::pdf::forms::{field_partial_name, mark_acroform_need_appearances, push_acroform_field, resolve_field_dict};

fn form_field_root_id(doc: &Document, mut id: ObjectId) -> Option<ObjectId> {
    for _ in 0..32 {
        let dict = resolve_field_dict(doc, id)?;
        if let Ok(Object::Reference(parent)) = dict.get(b"Parent") {
            id = *parent;
            continue;
        }
        return Some(id);
    }
    None
}

fn form_roots_on_pages(doc: &Document, page_ids: &[ObjectId]) -> Vec<ObjectId> {
    let mut roots = BTreeMap::new();
    for &page_id in page_ids {
        let Ok(page) = doc.get_dictionary(page_id) else { continue };
        let Ok(Object::Array(annots)) = page.get(b"Annots") else { continue };
        for annot in annots {
            let Object::Reference(id) = annot else { continue };
            let Some(dict) = resolve_field_dict(doc, *id) else { continue };
            let is_widget = dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) == Some(b"Widget");
            if dict.get(b"FT").is_ok() || (is_widget && dict.get(b"Parent").is_ok()) {
                if let Some(root) = form_field_root_id(doc, *id) {
                    roots.insert(root, ());
                }
            }
        }
    }
    roots.keys().copied().collect()
}

fn acroform_tree_contains(
    doc: &Document,
    field: &Object,
    target: ObjectId,
    depth: u32,
    visited: &mut HashSet<ObjectId>,
) -> bool {
    if depth >= 32 {
        return false;
    }
    match field {
        Object::Reference(id) => {
            if *id == target {
                return true;
            }
            if !visited.insert(*id) {
                return false;
            }
            if let Some(dict) = resolve_field_dict(doc, *id) {
                if let Some(arr) = dict.get(b"Kids").ok().and_then(|o| o.as_array().ok()) {
                    return arr.iter().any(|kid| acroform_tree_contains(doc, kid, target, depth + 1, visited));
                }
            }
            false
        }
        _ => false,
    }
}

fn acroform_already_has_field(doc: &Document, field_id: ObjectId) -> bool {
    let Ok(catalog) = doc.catalog() else { return false };
    let Ok(Object::Reference(af_id)) = catalog.get(b"AcroForm") else { return false };
    let Ok(af) = doc.get_dictionary(*af_id) else { return false };
    let Ok(Object::Array(fields)) = af.get(b"Fields") else { return false };
    let mut visited = HashSet::new();
    fields.iter().any(|entry| acroform_tree_contains(doc, entry, field_id, 0, &mut visited))
}

fn rename_form_field_title(doc: &mut Document, field_id: ObjectId, new_name: &str) -> Result<(), String> {
    doc.get_dictionary_mut(field_id)
        .map_err(|e| e.to_string())?
        .set(b"T", Object::String(new_name.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    Ok(())
}

fn resolve_imported_form_name_conflict(doc: &mut Document, field_id: ObjectId) -> Result<(), String> {
    let Some(root) = form_field_root_id(doc, field_id) else {
        return Ok(());
    };
    let Some(name) = resolve_field_dict(doc, root).and_then(field_partial_name) else {
        return Ok(());
    };
    let mut clash = false;
    for &page_id in doc.get_pages().values() {
        let Ok(page) = doc.get_dictionary(page_id) else { continue };
        let Ok(Object::Array(annots)) = page.get(b"Annots") else { continue };
        for annot in annots {
            let Object::Reference(id) = annot else { continue };
            let Some(other_root) = form_field_root_id(doc, *id) else { continue };
            if other_root == root {
                continue;
            }
            if resolve_field_dict(doc, other_root).and_then(field_partial_name).as_deref() == Some(name.as_str()) {
                clash = true;
                break;
            }
        }
        if clash {
            break;
        }
    }
    if !clash {
        return Ok(());
    }
    let mut candidate = format!("imported_{name}");
    let mut suffix = 1u32;
    loop {
        let mut available = true;
        for &page_id in doc.get_pages().values() {
            let Ok(page) = doc.get_dictionary(page_id) else { continue };
            let Ok(Object::Array(annots)) = page.get(b"Annots") else { continue };
            for annot in annots {
                let Object::Reference(id) = annot else { continue };
                let Some(other_root) = form_field_root_id(doc, *id) else { continue };
                if other_root == root {
                    continue;
                }
                if resolve_field_dict(doc, other_root).and_then(field_partial_name).as_deref()
                    == Some(candidate.as_str())
                {
                    available = false;
                    break;
                }
            }
            if !available {
                break;
            }
        }
        if available {
            break;
        }
        candidate = format!("imported_{name}_{suffix}");
        suffix += 1;
    }
    rename_form_field_title(doc, root, &candidate)
}

fn register_acroform_field(doc: &mut Document, field_id: ObjectId) -> Result<(), String> {
    if acroform_already_has_field(doc, field_id) {
        return Ok(());
    }
    resolve_imported_form_name_conflict(doc, field_id)?;
    push_acroform_field(doc, field_id)
}

pub fn merge_acroform_after_insert(
    doc: &mut Document,
    insert_doc: &Document,
    inserted_page_ids: &[ObjectId],
    remap: &BTreeMap<ObjectId, ObjectId>,
) -> Result<(), String> {
    let mut roots = BTreeMap::<ObjectId, ()>::new();
    for root in form_roots_on_pages(doc, inserted_page_ids) {
        roots.insert(root, ());
    }
    if let Ok(catalog) = insert_doc.catalog() {
        if let Ok(Object::Reference(af_id)) = catalog.get(b"AcroForm") {
            if let Ok(af) = insert_doc.get_dictionary(*af_id) {
                if let Ok(Object::Array(fields)) = af.get(b"Fields") {
                    for field in fields {
                        let Object::Reference(src_id) = field else { continue };
                        if let Some(&dst_id) = remap.get(src_id) {
                            roots.insert(dst_id, ());
                        }
                    }
                }
            }
        }
    }
    for root in roots.keys().copied() {
        register_acroform_field(doc, root)?;
    }
    if !roots.is_empty() {
        mark_acroform_need_appearances(doc)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Object};

    #[test]
    fn acroform_tree_contains_breaks_kids_cycle() {
        let mut doc = Document::with_version("1.4");
        let field_id = doc.add_object(Object::Dictionary(Dictionary::new()));
        if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(field_id) {
            dict.set(b"FT".to_vec(), Object::Name(b"Tx".to_vec()));
            dict.set(b"Kids".to_vec(), Object::Array(vec![Object::Reference(field_id)]));
        }

        let mut visited = HashSet::new();
        let start = std::time::Instant::now();
        let contains = acroform_tree_contains(&doc, &Object::Reference(field_id), (9999, 0), 0, &mut visited);
        assert!(start.elapsed() < std::time::Duration::from_secs(1), "acroform tree walk cycled too long");
        assert!(!contains);
    }

    #[test]
    fn acroform_tree_contains_caps_kids_depth() {
        let mut doc = Document::with_version("1.4");
        // Build a 35-level deep /Kids chain of distinct field IDs.
        let mut chain = Vec::new();
        for _ in 0..35 {
            let id = doc.add_object(Object::Dictionary(Dictionary::new()));
            chain.push(id);
        }
        for (i, &id) in chain.iter().enumerate() {
            if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(id) {
                dict.set(b"FT".to_vec(), Object::Name(b"Tx".to_vec()));
                if i + 1 < chain.len() {
                    dict.set(b"Kids".to_vec(), Object::Array(vec![Object::Reference(chain[i + 1])]));
                }
            }
        }

        let mut visited = HashSet::new();
        let start = std::time::Instant::now();
        let contains = acroform_tree_contains(&doc, &Object::Reference(chain[0]), chain[34], 0, &mut visited);
        assert!(start.elapsed() < std::time::Duration::from_secs(1), "acroform tree walk exceeded depth cap");
        assert!(!contains, "depth cap should prevent reaching the 35th-level field");
    }
}
