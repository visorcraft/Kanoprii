use crate::pdf::page_images::list_page_images;
use image::{DynamicImage, GenericImageView, GrayImage, RgbImage};
use lopdf::{Document, Object, ObjectId, Stream};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

const MIN_QUALITY: u8 = 10;
const MAX_QUALITY: u8 = 90;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeEstimate {
    pub original_bytes: u64,
    pub estimated_bytes: u64,
    pub images_recompressed: u32,
}

fn clamp_quality(quality: u8) -> u8 {
    quality.clamp(MIN_QUALITY, MAX_QUALITY)
}

fn filter_has(filter: &Object, target: &[u8]) -> bool {
    match filter {
        Object::Name(name) => name.as_slice() == target,
        Object::Array(items) => {
            items.iter().any(|item| matches!(item, Object::Name(name) if name.as_slice() == target))
        }
        _ => false,
    }
}

fn filter_names(stream: &Stream) -> Vec<Vec<u8>> {
    match stream.dict.get(b"Filter") {
        Ok(Object::Name(name)) => vec![name.clone()],
        Ok(Object::Array(items)) => items.iter().filter_map(|item| item.as_name().ok().map(|n| n.to_vec())).collect(),
        _ => Vec::new(),
    }
}

fn should_skip(stream: &Stream) -> bool {
    let bits = stream.dict.get(b"BitsPerComponent").ok().and_then(|obj| obj.as_i64().ok()).unwrap_or(8);
    if bits <= 1 {
        return true;
    }
    if stream.dict.get(b"ImageMask").ok().and_then(|obj| obj.as_bool().ok()) == Some(true) {
        return true;
    }
    if let Ok(filter) = stream.dict.get(b"Filter") {
        if filter_has(filter, b"CCITTFaxDecode") || filter_has(filter, b"JBIG2Decode") {
            return true;
        }
    }
    false
}

fn stream_image_bytes(stream: &Stream) -> Option<Vec<u8>> {
    let filters = filter_names(stream);
    if filters.is_empty() {
        return Some(stream.content.clone());
    }
    if filters.iter().any(|name| name.as_slice() == b"DCTDecode") {
        if filters.len() == 1 {
            return Some(stream.content.clone());
        }
        let mut wrapper = stream.clone();
        let peeled: Vec<Object> = filters
            .iter()
            .filter(|name| name.as_slice() != b"DCTDecode")
            .map(|name| Object::Name(name.clone()))
            .collect();
        wrapper.dict.set(b"Filter", Object::Array(peeled));
        return wrapper.decompressed_content().ok().or_else(|| Some(stream.content.clone()));
    }
    stream.decompressed_content().ok().or_else(|| Some(stream.content.clone()))
}

fn decode_stream(stream: &Stream) -> Option<DynamicImage> {
    if should_skip(stream) {
        return None;
    }
    let bytes = stream_image_bytes(stream)?;
    if stream.dict.get(b"Filter").ok().is_some_and(|filter| filter_has(filter, b"DCTDecode"))
        || bytes.starts_with(&[0xFF, 0xD8])
    {
        return image::load_from_memory(&bytes).ok();
    }
    let width = stream.dict.get(b"Width").ok().and_then(|obj| obj.as_i64().ok())? as u32;
    let height = stream.dict.get(b"Height").ok().and_then(|obj| obj.as_i64().ok())? as u32;
    if width == 0 || height == 0 {
        return None;
    }
    let gray = match stream.dict.get(b"ColorSpace") {
        Ok(Object::Name(name)) => name.as_slice() == b"DeviceGray",
        _ => false,
    };
    if gray {
        let expected = (width as usize).checked_mul(height as usize)?;
        if bytes.len() < expected {
            return None;
        }
        let img = GrayImage::from_raw(width, height, bytes[..expected].to_vec())?;
        Some(DynamicImage::ImageLuma8(img))
    } else {
        let expected = (width as usize).checked_mul(height as usize)?.checked_mul(3)?;
        if bytes.len() < expected {
            return None;
        }
        let img = RgbImage::from_raw(width, height, bytes[..expected].to_vec())?;
        Some(DynamicImage::ImageRgb8(img))
    }
}

fn encode_jpeg(img: &DynamicImage, quality: u8) -> Option<Vec<u8>> {
    use image::codecs::jpeg::JpegEncoder;
    let mut buf = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut buf, clamp_quality(quality));
    img.write_with_encoder(encoder).ok()?;
    Some(buf)
}

fn maybe_downsample(img: DynamicImage, drawn: Option<(f64, f64)>, max_dpi: u32) -> DynamicImage {
    if max_dpi == 0 {
        return img;
    }
    let Some((drawn_w, drawn_h)) = drawn else {
        return img;
    };
    if drawn_w <= 0.0 || drawn_h <= 0.0 {
        return img;
    }
    let (px_w, px_h) = img.dimensions();
    let target_w = ((max_dpi as f64) * drawn_w / 72.0).round().max(1.0);
    let target_h = ((max_dpi as f64) * drawn_h / 72.0).round().max(1.0);
    let scale = (target_w / px_w as f64).min(target_h / px_h as f64).min(1.0);
    if scale >= 1.0 {
        return img;
    }
    let new_w = (px_w as f64 * scale).round().max(1.0) as u32;
    let new_h = (px_h as f64 * scale).round().max(1.0) as u32;
    img.resize_exact(new_w, new_h, image::imageops::FilterType::Triangle)
}

fn replace_jpeg_stream(stream: &mut Stream, jpeg: Vec<u8>, width: u32, height: u32, gray: bool) {
    stream.set_plain_content(jpeg);
    stream.dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
    stream.dict.set("Width", Object::Integer(width as i64));
    stream.dict.set("Height", Object::Integer(height as i64));
    stream.dict.set("ColorSpace", Object::Name(if gray { b"DeviceGray".to_vec() } else { b"DeviceRGB".to_vec() }));
    stream.dict.set("BitsPerComponent", Object::Integer(8));
}

fn image_drawn_sizes(doc: &Document) -> HashMap<ObjectId, (f64, f64)> {
    let mut map = HashMap::new();
    let page_count = doc.get_pages().len();
    for index in 0..page_count {
        let Ok(images) = list_page_images(doc, index as u32) else {
            continue;
        };
        for img in images {
            let id = (img.object_id.0, img.object_id.1);
            let width = img.rect.width.abs().max(img.bbox.width.abs());
            let height = img.rect.height.abs().max(img.bbox.height.abs());
            let entry = map.entry(id).or_insert((0.0, 0.0));
            entry.0 = f64::max(entry.0, width);
            entry.1 = f64::max(entry.1, height);
        }
    }
    map
}

fn resize_smask(doc: &mut Document, id: ObjectId, width: u32, height: u32) -> Result<(), String> {
    let decoded = match decode_image_at(doc, id) {
        Some(img) => img,
        None => return Err("Could not decode image mask".to_string()),
    };
    let gray = DynamicImage::ImageLuma8(decoded.to_luma8());
    let resized = gray.resize_exact(width, height, image::imageops::FilterType::Triangle);
    let pixels = resized.to_luma8().into_raw();
    let obj = doc.get_object_mut(id).map_err(|e| e.to_string())?;
    let Object::Stream(stream) = obj else {
        return Err("Image mask is not a stream".to_string());
    };
    stream.set_plain_content(pixels);
    stream.dict.set("Width", Object::Integer(width as i64));
    stream.dict.set("Height", Object::Integer(height as i64));
    stream.dict.set("ColorSpace", Object::Name(b"DeviceGray".to_vec()));
    stream.dict.set("BitsPerComponent", Object::Integer(8));
    Ok(())
}

fn decode_image_at(doc: &Document, id: ObjectId) -> Option<DynamicImage> {
    let Object::Stream(stream) = doc.get_object(id).ok()? else {
        return None;
    };
    decode_stream(stream)
}

fn recompress_one(
    doc: &mut Document,
    id: ObjectId,
    drawn: Option<(f64, f64)>,
    quality: u8,
    max_dpi: u32,
) -> Result<bool, String> {
    let (orig_len, smask_id) = {
        let obj = doc.get_object(id).map_err(|e| e.to_string())?;
        let Object::Stream(stream) = obj else {
            return Ok(false);
        };
        if should_skip(stream) {
            return Ok(false);
        }
        let smask = stream.dict.get(b"SMask").ok().and_then(|obj| obj.as_reference().ok());
        (stream.content.len(), smask)
    };
    let Some(decoded) = decode_image_at(doc, id) else {
        return Ok(false);
    };
    let (px_w, px_h) = decoded.dimensions();
    let resized = maybe_downsample(decoded, drawn, max_dpi);
    let (new_w, new_h) = resized.dimensions();
    let gray = !resized.color().has_color();
    let Some(jpeg) = encode_jpeg(&resized, quality) else {
        return Ok(false);
    };
    if jpeg.len() >= orig_len {
        return Ok(false);
    }
    if let Some(smask) = smask_id {
        if (new_w != px_w || new_h != px_h) && resize_smask(doc, smask, new_w, new_h).is_err() {
            return Ok(false);
        }
    }
    let obj = doc.get_object_mut(id).map_err(|e| e.to_string())?;
    let Object::Stream(stream) = obj else {
        return Ok(false);
    };
    replace_jpeg_stream(stream, jpeg, new_w, new_h, gray);
    Ok(true)
}

fn recompress_images(doc: &mut Document, quality: u8, max_dpi: u32) -> Result<u32, String> {
    let drawn = image_drawn_sizes(doc);
    let mut smask_ids = HashSet::new();
    let mut image_ids = Vec::new();
    for (id, obj) in &doc.objects {
        let Object::Stream(stream) = obj else {
            continue;
        };
        if stream.dict.get(b"Subtype").ok().and_then(|obj| obj.as_name().ok()) != Some(b"Image") {
            continue;
        }
        image_ids.push(*id);
        if let Ok(Object::Reference(smask)) = stream.dict.get(b"SMask") {
            smask_ids.insert(*smask);
        }
    }
    let mut count = 0u32;
    for id in image_ids {
        if smask_ids.contains(&id) {
            continue;
        }
        if recompress_one(doc, id, drawn.get(&id).copied(), quality, max_dpi)? {
            count += 1;
        }
    }
    Ok(count)
}

fn strip_and_compress(doc: &mut Document, quality: u8, max_dpi: u32) -> Result<u32, String> {
    if let Ok(catalog) = doc.catalog_mut() {
        catalog.set(b"Metadata", Object::Null);
    }
    if let Ok(trailer) = doc.trailer.get_mut(b"Info") {
        *trailer = Object::Null;
    }
    let images_recompressed = recompress_images(doc, quality, max_dpi)?;
    doc.prune_objects();
    doc.compress();
    Ok(images_recompressed)
}

fn same_path(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn format_bytes(n: u64) -> String {
    if n >= 1024 * 1024 {
        format!("{:.2} MB", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

fn size_clause(original: u64, next: u64) -> String {
    if original > 0 && next < original {
        let percent = ((original - next) as f64 / original as f64 * 100.0).round() as u32;
        format!("{} → {} ({}% smaller)", format_bytes(original), format_bytes(next), percent)
    } else {
        format!("{} → {}", format_bytes(original), format_bytes(next))
    }
}

fn transform_doc(source: &Path, quality: u8, max_dpi: u32) -> Result<(Document, u32, u64), String> {
    let original_bytes = std::fs::metadata(source).map_err(|e| e.to_string())?.len();
    let mut doc = Document::load(source).map_err(|e| e.to_string())?;
    let images_recompressed = strip_and_compress(&mut doc, clamp_quality(quality), max_dpi)?;
    Ok((doc, images_recompressed, original_bytes))
}

fn serialize_doc(doc: &mut Document) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    doc.save_to(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

pub fn source_size(path: &Path) -> Result<u64, String> {
    std::fs::metadata(path).map(|meta| meta.len()).map_err(|e| e.to_string())
}

pub fn estimate_optimize_pdf(source: &Path, quality: u8, max_dpi: u32) -> Result<OptimizeEstimate, String> {
    let (mut doc, images_recompressed, original_bytes) = transform_doc(source, quality, max_dpi)?;
    let buf = serialize_doc(&mut doc)?;
    Ok(OptimizeEstimate { original_bytes, estimated_bytes: buf.len() as u64, images_recompressed })
}

/// Optimize `source` (the open working copy).
///
/// - `replace == false`: write `<original-stem>_optimized.pdf` next to `original`.
///   Leaves `source` and `original` unchanged.
/// - `replace == true`: overwrite `source`, then write the same bytes to `original`
///   so the open document and the file on disk stay in sync. No sibling is created.
/// - `max_dpi == 0`: do not downsample (Original).
pub fn optimize_pdf_file(
    source: &Path,
    original: &Path,
    replace: bool,
    quality: u8,
    max_dpi: u32,
) -> Result<String, String> {
    let original = if original.as_os_str().is_empty() { source } else { original };
    let (mut doc, images_recompressed, original_bytes) = transform_doc(source, quality, max_dpi)?;
    let dest = if replace {
        if crate::pdf::security::is_encrypted(original)? {
            return Err("Cannot replace a password-protected original. Save as a new file instead.".to_string());
        }
        crate::pdf::io::save_atomic(&mut doc, source)?;
        crate::pdf::render::invalidate_document_cache(source);
        if !same_path(source, original) {
            crate::pdf::io::save_atomic(&mut doc, original)?;
            crate::pdf::render::invalidate_document_cache(original);
        }
        source.to_path_buf()
    } else {
        let dest = crate::pdf::io::unique_sibling_pdf(original, "_optimized");
        crate::pdf::io::save_atomic(&mut doc, &dest)?;
        dest
    };
    let new_len = std::fs::metadata(&dest).map(|m| m.len()).map_err(|e| e.to_string())?;
    let summary = format!(
        "Metadata stripped, objects pruned & streams compressed. {} image(s) recompressed. {}.",
        images_recompressed,
        size_clause(original_bytes, new_len)
    );
    if replace {
        Ok(format!("Replaced {}. {summary}", original.display()))
    } else {
        Ok(format!("Saved to {}. {summary}", dest.display()))
    }
}
