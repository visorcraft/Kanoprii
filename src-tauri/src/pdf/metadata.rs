use lopdf::{Dictionary, Document, Object, ObjectId};

pub fn read_info_string(doc: &Document, key: &[u8]) -> Option<String> {
    let object = doc.trailer.get(b"Info").ok()?;
    let dict = match object {
        Object::Reference(id) => doc.get_dictionary(*id).ok()?,
        Object::Dictionary(dict) => dict,
        _ => return None,
    };
    dict.get(key).ok().and_then(|value| value.as_str().ok()).map(|bytes| String::from_utf8_lossy(bytes).into_owned())
}

pub fn ensure_info_dict_id(doc: &mut Document) -> Result<ObjectId, String> {
    match doc.trailer.get(b"Info") {
        Ok(Object::Reference(id)) => Ok(*id),
        Ok(Object::Dictionary(dict)) => {
            let id = doc.add_object(Object::Dictionary(dict.clone()));
            doc.trailer.set(b"Info", Object::Reference(id));
            Ok(id)
        }
        _ => {
            let id = doc.add_object(Object::Dictionary(Dictionary::new()));
            doc.trailer.set(b"Info", Object::Reference(id));
            Ok(id)
        }
    }
}

pub fn unix_seconds_to_utc_parts(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let time = secs.rem_euclid(86_400);
    let hour = (time / 3_600) as u32;
    let minute = ((time % 3_600) / 60) as u32;
    let second = (time % 60) as u32;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i32) + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let month = (5 * doy + 2) / 153;
    let day = (doy - (153 * month + 2) / 5 + 1) as u32;
    let month = ((month + 2) % 12 + 1) as u32;
    let year = y + i32::from(month <= 2);

    (year, month, day, hour, minute, second)
}

pub fn current_pdf_mod_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let (year, month, day, hour, minute, second) = unix_seconds_to_utc_parts(secs);
    format!("D:{year:04}{month:02}{day:02}{hour:02}{minute:02}{second:02}Z")
}

pub fn write_info_text_field(dict: &mut Dictionary, key: &[u8], value: Option<String>) {
    let Some(text) = value.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()) else {
        dict.remove(key);
        return;
    };
    dict.set(key, Object::String(text.into_bytes(), lopdf::StringFormat::Literal));
}
