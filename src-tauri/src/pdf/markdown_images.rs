use crate::pdf::import::{import_dict, import_object};
use crate::pdf::markdown_heuristic::MarkdownTextLine;
use crate::pdf::markdown_tagged::plain_text_to_markdown;
use crate::pdf::ocr::{resolve_tesseract, OcrExportStats, OCR_RENDER_H, OCR_RENDER_W};
use crate::pdf::page_tree::inherited_page_attr;
use crate::pdf::pdfium_bind::render_page_image;
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub struct MarkdownImageSink<'a> {
    pub assets_dir: &'a Path,
    pub rel_prefix: &'a str,
}

/// Pages with little extractable text may still be scanned or image-heavy layouts.
pub const PAGE_OCR_MIN_CHARS: usize = 120;

fn page_visible_char_count(lines: &[MarkdownTextLine], plain_fallback: &str) -> usize {
    if lines.is_empty() {
        plain_fallback.chars().filter(|c| !c.is_whitespace()).count()
    } else {
        lines.iter().map(|l| l.text.chars().filter(|c| !c.is_whitespace()).count()).sum()
    }
}

pub fn page_needs_ocr_supplement(lines: &[MarkdownTextLine], plain_fallback: &str) -> bool {
    page_visible_char_count(lines, plain_fallback) < PAGE_OCR_MIN_CHARS
}

pub fn try_ocr_image_bytes(bytes: &[u8], ext: &str) -> Result<Option<String>, String> {
    let png = match ext {
        "png" => bytes.to_vec(),
        "jpg" | "jpeg" => {
            let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
            let mut buf = Vec::new();
            img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png).map_err(|e| e.to_string())?;
            buf
        }
        _ => return Ok(None),
    };
    crate::pdf::ocr::try_ocr_png_bytes(&png)
}

pub(crate) fn append_page_ocr_supplement(
    markdown: &mut String,
    path: &Path,
    page_index: u32,
    stats: &mut OcrExportStats,
) -> Result<(), String> {
    let png = render_page_image(path, page_index, OCR_RENDER_W, OCR_RENDER_H, image::ImageFormat::Png)?;
    stats.sparse_supplements += 1;
    if let Some(text) = crate::pdf::ocr::try_ocr_png_bytes(&png)? {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            markdown.push_str("#### OCR (page render)\n\n");
            markdown.push_str(&plain_text_to_markdown(trimmed));
            stats.text_blocks += 1;
        }
    } else if resolve_tesseract().is_none() {
        markdown.push_str(&crate::pdf::ocr::ocr_missing_hint("Sparse page"));
        stats.missing_install_hints += 1;
    }
    Ok(())
}

pub(crate) fn append_scanned_page_markdown(
    markdown: &mut String,
    path: &Path,
    page_index: u32,
    image_sink: Option<&MarkdownImageSink<'_>>,
    stats: &mut OcrExportStats,
) -> Result<(), String> {
    let png = render_page_image(path, page_index, OCR_RENDER_W, OCR_RENDER_H, image::ImageFormat::Png)?;
    let ocr_text = crate::pdf::ocr::try_ocr_png_bytes(&png)?;
    stats.scanned_pages += 1;

    if let Some(sink) = image_sink {
        fs::create_dir_all(sink.assets_dir).map_err(|e| e.to_string())?;
        let file_name = format!("page-{}.png", page_index + 1);
        fs::write(sink.assets_dir.join(&file_name), &png).map_err(|e| e.to_string())?;
        markdown.push_str(&format!("![Page {}]({}/{})\n\n", page_index + 1, sink.rel_prefix, file_name));
    }

    if let Some(text) = ocr_text {
        markdown.push_str(&plain_text_to_markdown(&text));
        stats.text_blocks += 1;
    } else if resolve_tesseract().is_none() {
        markdown.push_str(&crate::pdf::ocr::ocr_missing_hint("Scanned page"));
        stats.missing_install_hints += 1;
    }
    Ok(())
}

fn pdf_filter_has_name(filter: &Object, target: &[u8]) -> bool {
    match filter {
        Object::Name(name) => name == target,
        Object::Array(items) => items.iter().any(|item| matches!(item, Object::Name(name) if name == target)),
        _ => false,
    }
}

fn pdf_filter_is_dctdecode(filter: &Object) -> bool {
    pdf_filter_has_name(filter, b"DCTDecode")
}

fn pdf_numeric_i64(obj: &Object) -> Option<i64> {
    match obj {
        Object::Integer(v) => Some(*v),
        Object::Real(v) => Some(*v as i64),
        _ => None,
    }
}

fn pdf_colorspace_name(colorspace: &Object) -> Option<Vec<u8>> {
    match colorspace {
        Object::Name(name) => Some(name.to_vec()),
        Object::Array(items) => items.first().and_then(|o| o.as_name().ok()).map(|n| n.to_vec()),
        _ => None,
    }
}

fn pdf_colorspace_is(colorspace: &Object, target: &[u8]) -> bool {
    pdf_colorspace_name(colorspace).as_deref() == Some(target)
}

fn cmyk_pixel_to_rgb(c: u8, m: u8, y: u8, k: u8) -> [u8; 3] {
    let c = c as f32 / 255.0;
    let m = m as f32 / 255.0;
    let y = y as f32 / 255.0;
    let k = k as f32 / 255.0;
    [
        (255.0 * (1.0 - c) * (1.0 - k)).round() as u8,
        (255.0 * (1.0 - m) * (1.0 - k)).round() as u8,
        (255.0 * (1.0 - y) * (1.0 - k)).round() as u8,
    ]
}

fn indexed_palette_rgb(colorspace: &Object) -> Option<Vec<u8>> {
    let items = colorspace.as_array().ok()?;
    if !pdf_colorspace_is(colorspace, b"Indexed") {
        return None;
    }
    let lookup = items.get(3)?;
    let Object::String(bytes, _) = lookup else {
        return None;
    };
    Some(bytes.clone())
}

fn raw_image_to_png(width: u32, height: u32, rgb: Vec<u8>) -> Option<Vec<u8>> {
    use image::{ImageBuffer, Rgb};
    let img: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_raw(width, height, rgb)?;
    let mut png = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).ok()?;
    Some(png)
}

fn gray_samples_to_png(width: u32, height: u32, bytes: &[u8]) -> Option<Vec<u8>> {
    let expected = (width as u64 * height as u64) as usize;
    if bytes.len() < expected {
        return None;
    }
    let mut rgb = Vec::with_capacity(expected * 3);
    for sample in &bytes[..expected] {
        rgb.extend_from_slice(&[*sample, *sample, *sample]);
    }
    raw_image_to_png(width, height, rgb)
}

fn rgb_samples_to_png(width: u32, height: u32, bytes: &[u8], components: usize) -> Option<Vec<u8>> {
    let expected = (width as u64 * height as u64 * components as u64) as usize;
    if bytes.len() < expected {
        return None;
    }
    if components == 3 {
        return raw_image_to_png(width, height, bytes[..expected].to_vec());
    }
    None
}

fn cmyk_samples_to_png(width: u32, height: u32, bytes: &[u8]) -> Option<Vec<u8>> {
    let expected = (width as u64 * height as u64 * 4) as usize;
    if bytes.len() < expected {
        return None;
    }
    let mut rgb = Vec::with_capacity((width as usize * height as usize) * 3);
    for chunk in bytes[..expected].chunks_exact(4) {
        rgb.extend_from_slice(&cmyk_pixel_to_rgb(chunk[0], chunk[1], chunk[2], chunk[3]));
    }
    raw_image_to_png(width, height, rgb)
}

fn indexed_samples_to_png(width: u32, height: u32, bytes: &[u8], palette: &[u8]) -> Option<Vec<u8>> {
    let expected = (width as u64 * height as u64) as usize;
    if bytes.len() < expected || palette.len() < 3 {
        return None;
    }
    let max_index = (palette.len() / 3).saturating_sub(1);
    let mut rgb = Vec::with_capacity(expected * 3);
    for &sample in &bytes[..expected] {
        let idx = (sample as usize).min(max_index) * 3;
        rgb.extend_from_slice(&palette[idx..idx + 3]);
    }
    raw_image_to_png(width, height, rgb)
}

fn pdf_decode_parms_dict(stream: &Stream) -> Option<&Dictionary> {
    match stream.dict.get(b"DecodeParms").ok()? {
        Object::Dictionary(dict) => Some(dict),
        Object::Array(items) => items.first().and_then(|item| item.as_dict().ok()),
        _ => None,
    }
}

fn pdf_bool_object(obj: &Object) -> Option<bool> {
    match obj {
        Object::Boolean(value) => Some(*value),
        Object::Integer(value) => Some(*value != 0),
        _ => None,
    }
}

fn pdf_ccitt_parms(stream: &Stream) -> Option<(u16, Option<u16>, i64, bool)> {
    let image_width = pdf_numeric_i64(stream.dict.get(b"Width").ok()?)? as u16;
    let image_height = stream.dict.get(b"Height").ok().and_then(pdf_numeric_i64).map(|v| v as u16);
    let decode_parms = pdf_decode_parms_dict(stream);
    let columns = decode_parms
        .and_then(|dict| dict.get(b"Columns").ok().and_then(pdf_numeric_i64))
        .map(|v| v as u16)
        .unwrap_or(image_width);
    let rows = decode_parms
        .and_then(|dict| dict.get(b"Rows").ok().and_then(pdf_numeric_i64))
        .map(|v| v as u16)
        .or(image_height);
    let k = decode_parms.and_then(|dict| dict.get(b"K").ok().and_then(pdf_numeric_i64)).unwrap_or(0);
    let black_is_1 =
        decode_parms.and_then(|dict| dict.get(b"BlackIs1").ok().and_then(pdf_bool_object)).unwrap_or(false);
    Some((columns.max(1), rows, k, black_is_1))
}

fn ccitt_pixel_value(color: fax::Color, black_is_1: bool) -> u8 {
    match (color, black_is_1) {
        (fax::Color::Black, false) | (fax::Color::White, true) => 0,
        (fax::Color::White, false) | (fax::Color::Black, true) => 255,
    }
}

fn ccitt_samples_to_png(columns: u16, rows: Option<u16>, k: i64, black_is_1: bool, bytes: &[u8]) -> Option<Vec<u8>> {
    use fax::decoder::{self, pels};

    let width = columns;
    let mut gray_rows: Vec<Vec<u8>> = Vec::new();
    if k < 0 {
        let height_limit = rows;
        decoder::decode_g4(bytes.iter().copied(), width, height_limit, |transitions| {
            let row = pels(transitions, width).map(|color| ccitt_pixel_value(color, black_is_1)).collect::<Vec<_>>();
            gray_rows.push(row);
        })?;
        if let Some(expected_rows) = rows {
            gray_rows.truncate(expected_rows as usize);
        }
        if gray_rows.is_empty() {
            return None;
        }
    } else if k == 0 {
        decoder::decode_g3(bytes.iter().copied(), |transitions| {
            let row = pels(transitions, width).map(|color| ccitt_pixel_value(color, black_is_1)).collect::<Vec<_>>();
            gray_rows.push(row);
        })?;
        if let Some(expected_rows) = rows {
            gray_rows.truncate(expected_rows as usize);
        }
        if gray_rows.is_empty() {
            return None;
        }
    } else {
        return None;
    }
    let height = gray_rows.len() as u32;
    let flat: Vec<u8> = gray_rows.into_iter().flatten().collect();
    gray_samples_to_png(width as u32, height, &flat)
}

fn run_length_decode(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        let length = bytes[index] as i8;
        index += 1;
        if length == -128 {
            continue;
        }
        if length >= 0 {
            let count = length as usize + 1;
            if index >= bytes.len() {
                break;
            }
            let value = bytes[index];
            index += 1;
            out.extend(std::iter::repeat_n(value, count));
        } else {
            let count = (-length) as usize + 1;
            if index + count > bytes.len() {
                break;
            }
            out.extend_from_slice(&bytes[index..index + count]);
            index += count;
        }
    }
    out
}

fn unpack_1bit_samples(bytes: &[u8], width: u32, height: u32, black_is_1: bool) -> Option<Vec<u8>> {
    let pixel_count = (width as u64).checked_mul(height as u64)? as usize;
    let mut gray = Vec::with_capacity(pixel_count);
    for bit_idx in 0..pixel_count {
        let byte = bytes.get(bit_idx / 8)?;
        let bit = (byte >> (7 - (bit_idx % 8))) & 1;
        let value = if (bit == 1) == black_is_1 { 0 } else { 255 };
        gray.push(value);
    }
    gray_samples_to_png(width, height, &gray)
}

fn pdf_filter_chain(filter: &Object) -> Vec<Vec<u8>> {
    match filter {
        Object::Name(name) => vec![name.to_vec()],
        Object::Array(items) => items.iter().filter_map(|item| item.as_name().ok().map(|name| name.to_vec())).collect(),
        _ => Vec::new(),
    }
}

/// Raw or decompressed stream bytes for image extraction. lopdf does not decode
/// CCITTFaxDecode or RunLengthDecode, so those filters use `stream.content`
/// (after any leading Flate/LZW/ASCII85 steps we can peel off).
fn pdf_image_encoded_bytes(stream: &Stream) -> Option<Vec<u8>> {
    let filters = stream.dict.get(b"Filter").ok().map(pdf_filter_chain).unwrap_or_default();
    if filters.is_empty() {
        return Some(stream.content.clone());
    }
    if filters.last().is_some_and(|name| name.as_slice() == b"CCITTFaxDecode" || name.as_slice() == b"RunLengthDecode")
    {
        if filters.len() == 1 {
            return Some(stream.content.clone());
        }
        let mut wrapper = stream.clone();
        wrapper.dict.set(
            b"Filter",
            Object::Array(filters[..filters.len() - 1].iter().map(|name| Object::Name(name.clone())).collect()),
        );
        return wrapper.decompressed_content().ok().or_else(|| Some(stream.content.clone()));
    }
    stream.decompressed_content().ok()
}

pub fn pdf_image_stream_bytes(stream: &Stream) -> Option<(Vec<u8>, &'static str)> {
    let filter = stream.dict.get(b"Filter").ok();
    let bytes = pdf_image_encoded_bytes(stream)?;
    if filter.is_some_and(pdf_filter_is_dctdecode) {
        return Some((bytes, "jpg"));
    }
    if filter.is_some_and(|f| pdf_filter_has_name(f, b"JPXDecode")) {
        if let Ok(img) = image::load_from_memory(&bytes) {
            let mut png = Vec::new();
            if img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).is_ok() {
                return Some((png, "png"));
            }
        }
    }
    if filter.is_some_and(|f| pdf_filter_has_name(f, b"CCITTFaxDecode")) {
        let (columns, rows, k, black_is_1) = pdf_ccitt_parms(stream)?;
        if let Some(png) = ccitt_samples_to_png(columns, rows, k, black_is_1, &bytes) {
            return Some((png, "png"));
        }
        return None;
    }

    let width = pdf_numeric_i64(stream.dict.get(b"Width").ok()?)? as u32;
    let height = pdf_numeric_i64(stream.dict.get(b"Height").ok()?)? as u32;
    if width == 0 || height == 0 {
        return None;
    }
    let bits = stream.dict.get(b"BitsPerComponent").ok().and_then(pdf_numeric_i64).unwrap_or(8) as u32;
    let colorspace = stream.dict.get(b"ColorSpace").ok()?;
    let decode = stream.dict.get(b"Decode").ok();
    let black_is_1 = decode
        .and_then(|obj| obj.as_array().ok())
        .and_then(|items| items.first())
        .and_then(pdf_numeric_f32)
        .is_some_and(|value| value > 0.5);

    if bits == 1 {
        let samples = if filter.is_some_and(|f| pdf_filter_has_name(f, b"RunLengthDecode")) {
            run_length_decode(&bytes)
        } else {
            bytes
        };
        if let Some(png) = unpack_1bit_samples(&samples, width, height, black_is_1) {
            return Some((png, "png"));
        }
        return None;
    }
    if bits != 8 {
        return None;
    }

    let png = if pdf_colorspace_is(colorspace, b"Indexed") {
        let palette = indexed_palette_rgb(colorspace)?;
        indexed_samples_to_png(width, height, &bytes, &palette)?
    } else if pdf_colorspace_is(colorspace, b"DeviceRGB") {
        rgb_samples_to_png(width, height, &bytes, 3)?
    } else if pdf_colorspace_is(colorspace, b"DeviceGray") {
        gray_samples_to_png(width, height, &bytes)?
    } else if pdf_colorspace_is(colorspace, b"DeviceCMYK") {
        cmyk_samples_to_png(width, height, &bytes)?
    } else {
        rgb_samples_to_png(width, height, &bytes, 3).or_else(|| gray_samples_to_png(width, height, &bytes))?
    };
    Some((png, "png"))
}

fn pdf_numeric_f32(obj: &Object) -> Option<f32> {
    match obj {
        Object::Integer(v) => Some(*v as f32),
        Object::Real(v) => Some(*v),
        _ => None,
    }
}

fn pdf_content_stream_bytes(doc: &Document, contents: &Object) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    match contents {
        Object::Reference(id) => {
            if let Ok(Object::Stream(stream)) = doc.get_object(*id) {
                if let Ok(bytes) = stream.decompressed_content() {
                    out.push(bytes);
                }
            }
        }
        Object::Array(items) => {
            for item in items {
                out.extend(pdf_content_stream_bytes(doc, item));
            }
        }
        Object::Stream(stream) => {
            if let Ok(bytes) = stream.decompressed_content() {
                out.push(bytes);
            }
        }
        _ => {}
    }
    out
}

fn pdf_name_token_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

/// Collect `/Name Do` XObject invocations from a page or form content stream.
pub fn parse_xobject_do_names(bytes: &[u8]) -> BTreeSet<Vec<u8>> {
    let mut names = BTreeSet::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'/' {
            index += 1;
            continue;
        }
        let start = index + 1;
        index += 1;
        while index < bytes.len() && pdf_name_token_char(bytes[index]) {
            index += 1;
        }
        if start == index {
            continue;
        }
        let mut cursor = index;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if bytes[cursor..].starts_with(b"Do") && !bytes.get(cursor + 2).copied().is_some_and(pdf_name_token_char) {
            names.insert(bytes[start..index].to_vec());
        }
    }
    names
}

pub fn resolve_page_resources(doc: &Document, page_id: ObjectId) -> Option<Dictionary> {
    let page = doc.get_dictionary(page_id).ok()?;
    if let Ok(resources) = page.get(b"Resources") {
        if let Ok(dict) = resources.as_dict() {
            return Some(dict.clone());
        }
    }
    inherited_page_attr(doc, page_id, b"Resources").and_then(|obj| obj.as_dict().ok().cloned())
}

pub fn collect_xobject_do_names_recursive(
    doc: &Document,
    resources: &Dictionary,
    content_bytes: &[u8],
    visited: &mut BTreeSet<ObjectId>,
) -> BTreeSet<Vec<u8>> {
    let mut used = parse_xobject_do_names(content_bytes);
    let Some(xobjects) = resources.get(b"XObject").ok().and_then(|obj| obj.as_dict().ok()) else {
        return used;
    };

    let nested_names: Vec<Vec<u8>> = used.iter().cloned().collect();
    for name in nested_names {
        let Some(form_id) = xobjects.get(&name).ok().and_then(|obj| obj.as_reference().ok()) else {
            continue;
        };
        if !visited.insert(form_id) {
            continue;
        }
        let Ok(Object::Stream(stream)) = doc.get_object(form_id) else {
            continue;
        };
        if stream.dict.get(b"Subtype").ok().and_then(|obj| obj.as_name().ok()) != Some(b"Form") {
            continue;
        }
        let form_resources = stream
            .dict
            .get(b"Resources")
            .ok()
            .and_then(|obj| obj.as_dict().ok())
            .map(|dict| merge_pdf_resource_dicts(resources.clone(), dict))
            .unwrap_or_else(|| resources.clone());
        let form_bytes = stream.decompressed_content().unwrap_or_default();
        used.extend(collect_xobject_do_names_recursive(doc, &form_resources, &form_bytes, visited));
    }
    used
}

pub fn xobject_names_used_on_page(doc: &Document, page_id: ObjectId) -> BTreeSet<Vec<u8>> {
    let page = match doc.get_dictionary(page_id) {
        Ok(page) => page,
        Err(_) => return BTreeSet::new(),
    };
    let contents = match page.get(b"Contents") {
        Ok(contents) => contents,
        Err(_) => return BTreeSet::new(),
    };
    let Some(resources) = resolve_page_resources(doc, page_id) else {
        let mut used = BTreeSet::new();
        for bytes in pdf_content_stream_bytes(doc, contents) {
            used.extend(parse_xobject_do_names(&bytes));
        }
        return used;
    };
    let mut visited = BTreeSet::new();
    let mut used = BTreeSet::new();
    for bytes in pdf_content_stream_bytes(doc, contents) {
        used.extend(collect_xobject_do_names_recursive(doc, &resources, &bytes, &mut visited));
    }
    used
}

fn merge_pdf_resource_dicts(mut base: Dictionary, overlay: &Dictionary) -> Dictionary {
    for (key, val) in overlay.iter() {
        let mergeable = matches!(key.as_slice(), b"XObject" | b"Font" | b"ExtGState" | b"ColorSpace" | b"Pattern");
        if mergeable {
            let base_sub = base.get(key).ok().and_then(|o| o.as_dict().ok());
            let overlay_sub = val.as_dict().ok();
            if let (Some(base_sub), Some(overlay_sub)) = (base_sub, overlay_sub) {
                let mut merged = base_sub.clone();
                for (sub_key, sub_val) in overlay_sub.iter() {
                    merged.set(sub_key.clone(), sub_val.clone());
                }
                base.set(key.clone(), Object::Dictionary(merged));
                continue;
            }
        }
        if base.get(key).is_err() {
            base.set(key.clone(), val.clone());
        }
    }
    base
}

fn pdf_form_bbox(stream: &Stream) -> Option<[f32; 4]> {
    let arr = stream.dict.get(b"BBox").ok()?.as_array().ok()?;
    if arr.len() < 4 {
        return None;
    }
    Some([pdf_numeric_f32(&arr[0])?, pdf_numeric_f32(&arr[1])?, pdf_numeric_f32(&arr[2])?, pdf_numeric_f32(&arr[3])?])
}

fn pdf_form_matrix(stream: &Stream) -> Option<[f32; 6]> {
    let arr = stream.dict.get(b"Matrix").ok()?.as_array().ok()?;
    if arr.len() < 6 {
        return None;
    }
    Some([
        pdf_numeric_f32(&arr[0])?,
        pdf_numeric_f32(&arr[1])?,
        pdf_numeric_f32(&arr[2])?,
        pdf_numeric_f32(&arr[3])?,
        pdf_numeric_f32(&arr[4])?,
        pdf_numeric_f32(&arr[5])?,
    ])
}

fn pdf_name_for_content_stream(name: &[u8]) -> String {
    if name.is_empty() {
        return "X".to_string();
    }
    let mut out = String::new();
    for &byte in name {
        if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' {
            out.push(byte as char);
        } else {
            out.push_str(&format!("#{:02X}", byte));
        }
    }
    out
}

const FORM_RENDER_MAX_PX: i32 = 2400;

fn form_render_pixel_size(width_pt: f32, height_pt: f32) -> (i32, i32) {
    let scale = (OCR_RENDER_W as f32 / width_pt.max(1.0)).max(1.0);
    let mut w = (width_pt * scale).round().max(1.0) as i32;
    let mut h = (height_pt * scale).round().max(1.0) as i32;
    let max_dim = w.max(h);
    if max_dim > FORM_RENDER_MAX_PX {
        let downscale = FORM_RENDER_MAX_PX as f32 / max_dim as f32;
        w = (w as f32 * downscale).round().max(1.0) as i32;
        h = (h as f32 * downscale).round().max(1.0) as i32;
    }
    (w, h)
}

fn pdf_image_dimensions(stream: &Stream) -> Option<(f32, f32)> {
    let width = pdf_numeric_f32(stream.dict.get(b"Width").ok()?)?;
    let height = pdf_numeric_f32(stream.dict.get(b"Height").ok()?)?;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    Some((width, height))
}

fn finish_single_page_render_pdf(
    dst: &mut Document,
    pages_id: ObjectId,
    page_resources_dst: Dictionary,
    media_width: f32,
    media_height: f32,
    content: String,
) -> Result<(), String> {
    let content_id = dst.add_object(Stream::new(Dictionary::new(), content.into_bytes()));
    let mut page = Dictionary::new();
    page.set("Type", Object::Name(b"Page".to_vec()));
    page.set("Parent", Object::Reference(pages_id));
    page.set("Resources", Object::Dictionary(page_resources_dst));
    page.set(
        "MediaBox",
        Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(media_width),
            Object::Real(media_height),
        ]),
    );
    page.set("Contents", Object::Reference(content_id));
    let page_obj_id = dst.add_object(Object::Dictionary(page));

    let mut pages = Dictionary::new();
    pages.set("Type", Object::Name(b"Pages".to_vec()));
    pages.set("Count", Object::Integer(1));
    pages.set("Kids", Object::Array(vec![Object::Reference(page_obj_id)]));
    dst.objects.insert(pages_id, Object::Dictionary(pages));

    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = dst.add_object(Object::Dictionary(catalog));
    dst.trailer.set("Root", Object::Reference(catalog_id));
    Ok(())
}

/// Build a one-page PDF that draws a single Form XObject at its natural BBox so
/// PDFium can rasterize vector charts and other non-image XObjects.
pub fn build_form_render_pdf(
    src: &Document,
    page_id: ObjectId,
    form_src_id: ObjectId,
    xobject_name: &[u8],
) -> Result<Document, String> {
    let bbox = src
        .get_object(form_src_id)
        .ok()
        .and_then(|o| o.as_stream().ok())
        .and_then(pdf_form_bbox)
        .unwrap_or([0.0, 0.0, 612.0, 792.0]);
    let llx = bbox[0];
    let lly = bbox[1];
    let width = (bbox[2] - llx).max(1.0);
    let height = (bbox[3] - lly).max(1.0);

    let mut dst = Document::with_version("1.5");
    let pages_id = dst.new_object_id();
    let mut remap = BTreeMap::new();

    let form_dst_id = import_object(&mut dst, src, form_src_id, pages_id, &mut remap);
    let form_stream = src.get_object(form_src_id).ok().and_then(|o| o.as_stream().ok());

    let mut page_resources_dst = resolve_page_resources(src, page_id)
        .map(|dict| import_dict(&mut dst, src, &dict, pages_id, &mut remap))
        .unwrap_or_default();
    if let Some(form_stream) = form_stream {
        if let Ok(form_resources) = form_stream.dict.get(b"Resources") {
            if let Ok(form_resources) = form_resources.as_dict() {
                let imported = import_dict(&mut dst, src, form_resources, pages_id, &mut remap);
                page_resources_dst = merge_pdf_resource_dicts(page_resources_dst, &imported);
            }
        }
    }

    let mut xobjects = page_resources_dst
        .get(b"XObject")
        .ok()
        .and_then(|obj| obj.as_dict().ok())
        .cloned()
        .unwrap_or_else(Dictionary::new);
    xobjects.set(xobject_name.to_vec(), Object::Reference(form_dst_id));
    page_resources_dst.set(b"XObject", Object::Dictionary(xobjects));

    let pdf_name = pdf_name_for_content_stream(xobject_name);
    let matrix_cm = form_stream
        .and_then(pdf_form_matrix)
        .map(|matrix| {
            format!("{} {} {} {} {} {} cm ", matrix[0], matrix[1], matrix[2], matrix[3], matrix[4], matrix[5])
        })
        .unwrap_or_default();
    let content = format!("q {matrix_cm}1 0 0 1 {} {} cm /{pdf_name} Do Q\n", -llx, -lly);
    finish_single_page_render_pdf(&mut dst, pages_id, page_resources_dst, width, height, content)?;
    Ok(dst)
}

pub fn build_image_render_pdf(
    src: &Document,
    page_id: ObjectId,
    image_src_id: ObjectId,
    xobject_name: &[u8],
) -> Result<Document, String> {
    let (width, height) = src
        .get_object(image_src_id)
        .ok()
        .and_then(|o| o.as_stream().ok())
        .and_then(pdf_image_dimensions)
        .unwrap_or((612.0, 792.0));

    let mut dst = Document::with_version("1.5");
    let pages_id = dst.new_object_id();
    let mut remap = BTreeMap::new();
    let image_dst_id = import_object(&mut dst, src, image_src_id, pages_id, &mut remap);

    let mut page_resources_dst = resolve_page_resources(src, page_id)
        .map(|dict| import_dict(&mut dst, src, &dict, pages_id, &mut remap))
        .unwrap_or_default();
    let mut xobjects = page_resources_dst
        .get(b"XObject")
        .ok()
        .and_then(|obj| obj.as_dict().ok())
        .cloned()
        .unwrap_or_else(Dictionary::new);
    xobjects.set(xobject_name.to_vec(), Object::Reference(image_dst_id));
    page_resources_dst.set(b"XObject", Object::Dictionary(xobjects));

    let pdf_name = pdf_name_for_content_stream(xobject_name);
    let content = format!("q {width} 0 0 {height} 0 0 cm /{pdf_name} Do Q\n");
    finish_single_page_render_pdf(&mut dst, pages_id, page_resources_dst, width, height, content)?;
    Ok(dst)
}

fn render_xobject_wrapper_png(
    wrapper: &mut Document,
    object_id: ObjectId,
    width_pt: f32,
    height_pt: f32,
    temp_prefix: &str,
) -> Result<Option<Vec<u8>>, String> {
    let (render_w, render_h) = form_render_pixel_size(width_pt, height_pt);
    let temp_path = std::env::temp_dir().join(format!(
        "pp_{temp_prefix}_{}_{}_{}.pdf",
        std::process::id(),
        object_id.0,
        object_id.1
    ));
    crate::pdf::io::save_atomic(&mut *wrapper, &temp_path)?;
    let png = match render_page_image(&temp_path, 0, render_w, render_h, image::ImageFormat::Png) {
        Ok(bytes) if !bytes.is_empty() => Some(bytes),
        Ok(_) => None,
        Err(_) => None,
    };
    let _ = fs::remove_file(&temp_path);
    Ok(png)
}

fn render_imported_form_xobject_png(
    src: &Document,
    page_id: ObjectId,
    form_src_id: ObjectId,
    xobject_name: &[u8],
) -> Result<Option<Vec<u8>>, String> {
    let bbox = src
        .get_object(form_src_id)
        .ok()
        .and_then(|o| o.as_stream().ok())
        .and_then(pdf_form_bbox)
        .unwrap_or([0.0, 0.0, 612.0, 792.0]);
    let width_pt = (bbox[2] - bbox[0]).max(1.0);
    let height_pt = (bbox[3] - bbox[1]).max(1.0);
    let mut wrapper = build_form_render_pdf(src, page_id, form_src_id, xobject_name)?;
    render_xobject_wrapper_png(&mut wrapper, form_src_id, width_pt, height_pt, "form")
}

fn render_imported_image_xobject_png(
    src: &Document,
    page_id: ObjectId,
    image_src_id: ObjectId,
    xobject_name: &[u8],
) -> Result<Option<Vec<u8>>, String> {
    let (width_pt, height_pt) = src
        .get_object(image_src_id)
        .ok()
        .and_then(|o| o.as_stream().ok())
        .and_then(pdf_image_dimensions)
        .unwrap_or((612.0, 792.0));
    let mut wrapper = build_image_render_pdf(src, page_id, image_src_id, xobject_name)?;
    render_xobject_wrapper_png(&mut wrapper, image_src_id, width_pt, height_pt, "image")
}

pub fn append_page_embedded_images(
    doc: &Document,
    page_number: u32,
    sink: &MarkdownImageSink<'_>,
    image_seq: &mut u32,
    stats: &mut OcrExportStats,
) -> Result<String, String> {
    fs::create_dir_all(sink.assets_dir).map_err(|e| e.to_string())?;
    let page_id = match doc.get_pages().get(&page_number) {
        Some(id) => *id,
        None => return Ok(String::new()),
    };
    let resources = resolve_page_resources(doc, page_id);
    let Some(resources) = resources else {
        return Ok(String::new());
    };
    let xobjects = resources.get(b"XObject").ok().and_then(|obj| obj.as_dict().ok());
    let Some(xobjects) = xobjects else {
        return Ok(String::new());
    };
    let used_on_page = xobject_names_used_on_page(doc, page_id);

    let mut block = String::new();
    for (name, obj) in xobjects.iter() {
        if !used_on_page.is_empty() && !used_on_page.contains(name) {
            continue;
        }
        let id = match obj {
            Object::Reference(id) => id,
            _ => continue,
        };
        let stream = match doc.get_object(*id).ok().and_then(|o| o.as_stream().ok()) {
            Some(stream) => stream,
            None => continue,
        };
        let subtype = stream.dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok());
        if subtype == Some(b"Form") {
            if let Some(png) =
                render_imported_form_xobject_png(doc, page_id, *id, name)?.filter(|bytes| !bytes.is_empty())
            {
                *image_seq += 1;
                let file_name = format!("page-{page_number}-form-{image_seq}.png");
                let ocr_text = crate::pdf::ocr::try_ocr_png_bytes(&png)?;
                fs::write(sink.assets_dir.join(&file_name), &png).map_err(|e| e.to_string())?;
                block.push_str(&format!(
                    "![Page {page_number} embedded form (vector chart)]({}/{file_name})\n\n",
                    sink.rel_prefix
                ));
                stats.embedded_form_blocks += 1;
                if let Some(text) = ocr_text {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        block.push_str("#### OCR (embedded form)\n\n");
                        block.push_str(&plain_text_to_markdown(trimmed));
                        stats.text_blocks += 1;
                    }
                }
            }
            continue;
        }
        if subtype != Some(b"Image") {
            continue;
        }
        let image_bytes = pdf_image_stream_bytes(stream).or_else(|| {
            render_imported_image_xobject_png(doc, page_id, *id, name).ok().flatten().map(|png| (png, "png"))
        });
        let Some((bytes, ext)) = image_bytes else {
            continue;
        };
        *image_seq += 1;
        let file_name = format!("page-{page_number}-img-{image_seq}.{ext}");
        let ocr_text = try_ocr_image_bytes(&bytes, ext)?;
        fs::write(sink.assets_dir.join(&file_name), &bytes).map_err(|e| e.to_string())?;
        block.push_str(&format!("![Page {page_number} embedded image]({}/{file_name})\n\n", sink.rel_prefix));
        stats.embedded_image_blocks += 1;
        if let Some(text) = ocr_text {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                block.push_str("#### OCR (embedded image)\n\n");
                block.push_str(&plain_text_to_markdown(trimmed));
                stats.text_blocks += 1;
            }
        }
    }
    Ok(block)
}
