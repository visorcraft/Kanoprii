use std::collections::BTreeMap;

use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

use crate::pdf::page_tree::{inherited_page_attr, is_page_dict, INHERITABLE_PAGE_KEYS};

pub fn import_object(
    dst: &mut Document,
    src: &Document,
    id: ObjectId,
    dst_root: ObjectId,
    remap: &mut BTreeMap<ObjectId, ObjectId>,
) -> ObjectId {
    if let Some(&new) = remap.get(&id) {
        return new;
    }
    let new_id = dst.new_object_id();
    remap.insert(id, new_id);
    let cloned = match src.get_object(id) {
        Ok(Object::Dictionary(d)) if is_page_dict(d) => {
            Object::Dictionary(import_page_dict(dst, src, id, d, dst_root, remap))
        }
        Ok(obj) => import_value(dst, src, obj, dst_root, remap),
        Err(_) => Object::Null,
    };
    dst.objects.insert(new_id, cloned);
    new_id
}

fn import_value(
    dst: &mut Document,
    src: &Document,
    value: &Object,
    dst_root: ObjectId,
    remap: &mut BTreeMap<ObjectId, ObjectId>,
) -> Object {
    match value {
        Object::Reference(rid) => Object::Reference(import_object(dst, src, *rid, dst_root, remap)),
        Object::Array(items) => {
            Object::Array(items.iter().map(|v| import_value(dst, src, v, dst_root, remap)).collect())
        }
        Object::Dictionary(d) => Object::Dictionary(import_dict(dst, src, d, dst_root, remap)),
        Object::Stream(s) => {
            Object::Stream(Stream::new(import_dict(dst, src, &s.dict, dst_root, remap), s.content.clone()))
        }
        other => other.clone(),
    }
}

pub fn import_dict(
    dst: &mut Document,
    src: &Document,
    dict: &Dictionary,
    dst_root: ObjectId,
    remap: &mut BTreeMap<ObjectId, ObjectId>,
) -> Dictionary {
    let mut out = Dictionary::new();
    for (key, val) in dict.iter() {
        out.set(key.clone(), import_value(dst, src, val, dst_root, remap));
    }
    out
}

/// Import a page detached from its source tree: resolve inherited attributes,
/// drop the upward /Parent link, deep-copy the remaining entries (Contents,
/// Resources, …), then point /Parent at the destination root.
fn import_page_dict(
    dst: &mut Document,
    src: &Document,
    src_page: ObjectId,
    dict: &Dictionary,
    dst_root: ObjectId,
    remap: &mut BTreeMap<ObjectId, ObjectId>,
) -> Dictionary {
    let mut page = dict.clone();
    page.remove(b"Parent");
    for key in INHERITABLE_PAGE_KEYS {
        if page.get(key).is_err() {
            if let Some(val) = inherited_page_attr(src, src_page, key) {
                page.set(key.to_vec(), val);
            }
        }
    }
    let mut out = import_dict(dst, src, &page, dst_root, remap);
    out.set("Parent", Object::Reference(dst_root));
    out
}
