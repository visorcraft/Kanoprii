use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PdfBookmarkEntry {
    pub title: String,
    pub depth: u32,
    pub page_index: Option<u32>,
}

pub fn page_index_for_object(doc: &Document, object_id: ObjectId) -> Option<u32> {
    doc.get_pages().iter().find(|(_, id)| **id == object_id).map(|(num, _)| num - 1)
}

pub fn outline_title(dict: &Dictionary) -> String {
    dict.get(b"Title")
        .ok()
        .and_then(|value| value.as_str().ok())
        .map(|value| String::from_utf8_lossy(value).into_owned())
        .unwrap_or_else(|| "Untitled".to_string())
}

pub fn resolve_dest_object(doc: &Document, dest: &Object) -> Option<u32> {
    match dest {
        Object::Array(items) if !items.is_empty() => {
            items[0].as_reference().ok().and_then(|id| page_index_for_object(doc, id))
        }
        Object::String(name, _) | Object::Name(name) => resolve_named_dest(doc, name.as_slice()),
        Object::Reference(id) => page_index_for_object(doc, *id),
        _ => None,
    }
}

pub fn resolve_named_dest(doc: &Document, name: &[u8]) -> Option<u32> {
    let catalog = doc.catalog().ok()?;
    let dests_id = catalog.get(b"Dests").ok()?.as_reference().ok()?;
    let dests = doc.get_dictionary(dests_id).ok()?;
    let names = dests.get(b"Names").ok()?.as_array().ok()?;
    let mut index = 0usize;
    while index + 1 < names.len() {
        let matches = names[index].as_str().ok().is_some_and(|value| value == name);
        if matches {
            return resolve_dest_object(doc, &names[index + 1]);
        }
        index += 2;
    }
    None
}

pub fn resolve_outline_destination(doc: &Document, dict: &Dictionary) -> Option<u32> {
    if let Ok(dest) = dict.get(b"Dest") {
        if let Some(page_index) = resolve_dest_object(doc, dest) {
            return Some(page_index);
        }
    }
    let action = dict.get(b"A").ok()?.as_dict().ok()?;
    let subtype = action.get(b"S").ok().and_then(|value| value.as_name().ok());
    if subtype != Some(b"GoTo".as_slice()) {
        return None;
    }
    resolve_dest_object(doc, action.get(b"D").ok()?)
}

pub fn collect_outline_items(doc: &Document, item_id: ObjectId, depth: u32, entries: &mut Vec<PdfBookmarkEntry>) {
    let mut current = Some(item_id);
    let mut visited = std::collections::HashSet::<ObjectId>::new();
    while let Some(id) = current {
        if !visited.insert(id) {
            break;
        }
        let dict = match doc.get_dictionary(id) {
            Ok(dict) => dict,
            Err(_) => break,
        };
        entries.push(PdfBookmarkEntry {
            title: outline_title(dict),
            depth,
            page_index: resolve_outline_destination(doc, dict),
        });
        if depth < 32 {
            if let Ok(first) = dict.get(b"First") {
                if let Ok(child_id) = first.as_reference() {
                    collect_outline_items(doc, child_id, depth + 1, entries);
                }
            }
        }
        current = dict.get(b"Next").ok().and_then(|value| value.as_reference().ok());
    }
}

pub fn collect_outline_ids(doc: &Document, item_id: ObjectId, depth: u32, ids: &mut Vec<ObjectId>) {
    if depth >= 32 {
        return;
    }
    let mut current = Some(item_id);
    let mut visited = std::collections::HashSet::<ObjectId>::new();
    while let Some(id) = current {
        if !visited.insert(id) {
            break;
        }
        ids.push(id);
        if let Ok(dict) = doc.get_dictionary(id) {
            if let Ok(first) = dict.get(b"First") {
                if let Ok(child_id) = first.as_reference() {
                    collect_outline_ids(doc, child_id, depth + 1, ids);
                }
            }
            current = dict.get(b"Next").ok().and_then(|value| value.as_reference().ok());
        } else {
            break;
        }
    }
}

pub fn flat_outline_ids(doc: &Document) -> Result<Vec<ObjectId>, String> {
    let catalog = doc.catalog().map_err(|e| e.to_string())?;
    let outlines_id = match catalog.get(b"Outlines") {
        Ok(Object::Reference(id)) => *id,
        _ => return Ok(Vec::new()),
    };
    let outlines = doc.get_dictionary(outlines_id).map_err(|e| e.to_string())?;
    let first_id = match outlines.get(b"First") {
        Ok(Object::Reference(id)) => *id,
        _ => return Ok(Vec::new()),
    };
    let mut ids = Vec::new();
    collect_outline_ids(doc, first_id, 0, &mut ids);
    Ok(ids)
}

pub fn remove_outline_item(doc: &mut Document, outlines_id: ObjectId, item_id: ObjectId) -> Result<(), String> {
    let dict = doc.get_dictionary(item_id).map_err(|e| e.to_string())?;
    if dict.get(b"First").ok().and_then(|o| o.as_reference().ok()).is_some() {
        return Err("Remove child bookmarks first".to_string());
    }
    let prev_id = dict.get(b"Prev").ok().and_then(|o| o.as_reference().ok());
    let next_id = dict.get(b"Next").ok().and_then(|o| o.as_reference().ok());
    if let Some(prev_id) = prev_id {
        if let Ok(Object::Dictionary(prev)) = doc.get_object_mut(prev_id) {
            if let Some(next_id) = next_id {
                prev.set("Next", Object::Reference(next_id));
            } else {
                prev.remove(b"Next");
            }
        }
    }
    if let Some(next_id) = next_id {
        if let Ok(Object::Dictionary(next)) = doc.get_object_mut(next_id) {
            if let Some(prev_id) = prev_id {
                next.set("Prev", Object::Reference(prev_id));
            } else {
                next.remove(b"Prev");
            }
        }
    }
    let outlines = doc.get_dictionary_mut(outlines_id).map_err(|e| e.to_string())?;
    let first_id = outlines.get(b"First").ok().and_then(|o| o.as_reference().ok());
    let last_id = outlines.get(b"Last").ok().and_then(|o| o.as_reference().ok());
    if first_id == Some(item_id) {
        if let Some(next_id) = next_id {
            outlines.set("First", Object::Reference(next_id));
        } else {
            outlines.remove(b"First");
            outlines.remove(b"Last");
        }
    }
    if last_id == Some(item_id) {
        if let Some(prev_id) = prev_id {
            outlines.set("Last", Object::Reference(prev_id));
        }
    }
    let count = outlines.get(b"Count").ok().and_then(|o| o.as_i64().ok()).unwrap_or(1);
    if count <= 1 {
        outlines.remove(b"Count");
        outlines.remove(b"First");
        outlines.remove(b"Last");
        let catalog_id = doc.trailer.get(b"Root").map_err(|e| e.to_string())?.as_reference().map_err(|_| "Bad Root")?;
        doc.get_dictionary_mut(catalog_id).map_err(|e| e.to_string())?.remove(b"Outlines");
    } else {
        outlines.set("Count", Object::Integer(count - 1));
    }
    doc.objects.remove(&item_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Object, Stream};

    fn minimal_doc_with_outline() -> (Document, ObjectId, ObjectId) {
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();
        doc.objects.insert(content_id, Object::Stream(Stream::new(Dictionary::new(), b"BT ET".to_vec())));
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
        page.set("Contents", Object::Reference(content_id));
        doc.objects.insert(page_id, Object::Dictionary(page));
        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name(b"Pages".to_vec()));
        pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let catalog_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set("Root", Object::Reference(catalog_id));

        (doc, pages_id, catalog_id)
    }

    #[test]
    fn collect_outline_items_breaks_next_cycle() {
        let (mut doc, _pages_id, catalog_id) = minimal_doc_with_outline();

        let item_id = doc.add_object(Object::Dictionary(Dictionary::new()));
        if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(item_id) {
            dict.set(b"Title".to_vec(), Object::String(b"A".to_vec(), lopdf::StringFormat::Literal));
            dict.set(b"Next".to_vec(), Object::Reference(item_id));
        }
        let outlines_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Outlines".to_vec())),
            (b"First".to_vec(), Object::Reference(item_id)),
        ])));
        if let Ok(Object::Dictionary(catalog)) = doc.get_object_mut(catalog_id) {
            catalog.set("Outlines", Object::Reference(outlines_id));
        }

        let start = std::time::Instant::now();
        let mut entries = Vec::new();
        collect_outline_items(&doc, item_id, 0, &mut entries);
        assert!(start.elapsed() < std::time::Duration::from_secs(1), "outline collection cycled too long");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn collect_outline_ids_breaks_next_cycle() {
        let (mut doc, _pages_id, catalog_id) = minimal_doc_with_outline();

        let item_id = doc.add_object(Object::Dictionary(Dictionary::new()));
        if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(item_id) {
            dict.set(b"Title".to_vec(), Object::String(b"A".to_vec(), lopdf::StringFormat::Literal));
            dict.set(b"Next".to_vec(), Object::Reference(item_id));
        }
        let outlines_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Outlines".to_vec())),
            (b"First".to_vec(), Object::Reference(item_id)),
        ])));
        if let Ok(Object::Dictionary(catalog)) = doc.get_object_mut(catalog_id) {
            catalog.set("Outlines", Object::Reference(outlines_id));
        }

        let start = std::time::Instant::now();
        let mut ids = Vec::new();
        collect_outline_ids(&doc, item_id, 0, &mut ids);
        assert!(start.elapsed() < std::time::Duration::from_secs(1), "outline id collection cycled too long");
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn collect_outline_ids_caps_first_depth() {
        let (mut doc, _pages_id, catalog_id) = minimal_doc_with_outline();

        // Build a 35-level deep /First chain of distinct IDs.
        let mut ids_chain = Vec::new();
        for _ in 0..35 {
            let id = doc.add_object(Object::Dictionary(Dictionary::new()));
            ids_chain.push(id);
        }
        for (i, &id) in ids_chain.iter().enumerate() {
            if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(id) {
                dict.set(b"Title".to_vec(), Object::String(format!("{i}").into_bytes(), lopdf::StringFormat::Literal));
                if i + 1 < ids_chain.len() {
                    dict.set(b"First".to_vec(), Object::Reference(ids_chain[i + 1]));
                }
            }
        }
        let outlines_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Outlines".to_vec())),
            (b"First".to_vec(), Object::Reference(ids_chain[0])),
        ])));
        if let Ok(Object::Dictionary(catalog)) = doc.get_object_mut(catalog_id) {
            catalog.set("Outlines", Object::Reference(outlines_id));
        }

        let mut ids = Vec::new();
        collect_outline_ids(&doc, ids_chain[0], 0, &mut ids);
        assert_eq!(ids.len(), 32, "depth cap should stop the walk at 32 levels");
    }
}
