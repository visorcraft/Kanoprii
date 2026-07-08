use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

pub fn next_image_xobject_name(xobjects: &Dictionary) -> String {
    for n in 1..=9999 {
        let name = format!("Im{n}");
        if xobjects.get(name.as_bytes()).is_err() {
            return name;
        }
    }
    "Im9999".to_string()
}

pub fn append_page_content(doc: &mut Document, page_id: ObjectId, ops: &[u8]) -> Result<(), String> {
    let contents = doc.get_dictionary(page_id).map_err(|e| e.to_string())?.get(b"Contents").ok().cloned();
    match contents {
        Some(Object::Reference(id)) => {
            let obj = doc.get_object_mut(id).map_err(|e| e.to_string())?;
            if let Object::Stream(stream) = obj {
                let mut body = stream.get_plain_content().map_err(|e| e.to_string())?;
                body.extend_from_slice(ops);
                stream.set_plain_content(body);
            } else {
                return Err("Bad page Contents".to_string());
            }
        }
        Some(Object::Array(mut arr)) => {
            let new_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), ops.to_vec())));
            arr.push(Object::Reference(new_id));
            doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(b"Contents", Object::Array(arr));
        }
        _ => {
            let stream_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), ops.to_vec())));
            doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(b"Contents", Object::Reference(stream_id));
        }
    }
    Ok(())
}

pub fn embed_jpeg_xobject(doc: &mut Document, jpeg: Vec<u8>, width: u32, height: u32) -> ObjectId {
    doc.add_object(Object::Stream(Stream::new(
        Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Image".to_vec())),
            (b"Width".to_vec(), Object::Integer(width as i64)),
            (b"Height".to_vec(), Object::Integer(height as i64)),
            (b"ColorSpace".to_vec(), Object::Name(b"DeviceRGB".to_vec())),
            (b"BitsPerComponent".to_vec(), Object::Integer(8)),
            (b"Filter".to_vec(), Object::Name(b"DCTDecode".to_vec())),
        ]),
        jpeg,
    )))
}
