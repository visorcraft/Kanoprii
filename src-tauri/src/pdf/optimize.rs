use lopdf::{Document, Object};
use std::path::{Path, PathBuf};

fn recompress_images(doc: &mut Document) -> Result<u32, String> {
    let pages = doc.get_pages();
    let mut all_images: Vec<(lopdf::ObjectId, Vec<u8>, u32, u32)> = Vec::new();

    for page_id in pages.values() {
        let images = doc.get_page_images(*page_id).map_err(|e| e.to_string())?;
        for img in &images {
            all_images.push((img.id, img.content.to_vec(), img.width as u32, img.height as u32));
        }
    }

    let mut count = 0u32;
    for (obj_id, content, width, height) in &all_images {
        let reencoded = reencode_image(content, *width, *height);
        if let Some(data) = reencoded {
            let obj = doc.get_object_mut(*obj_id).map_err(|e| e.to_string())?;
            if let Object::Stream(ref mut s) = obj {
                s.set_plain_content(data);
                s.dict.set(b"Filter", Object::Name(b"DCTDecode".to_vec()));
                count += 1;
            }
        }
    }

    Ok(count)
}

fn reencode_image(raw: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    use image::{DynamicImage, GrayImage, RgbImage};
    let expected_len = (width * height * 3) as usize;

    let img: DynamicImage = if raw.len() >= expected_len && expected_len > 0 {
        let rgb = RgbImage::from_raw(width, height, raw[..expected_len].to_vec())?;
        DynamicImage::ImageRgb8(rgb)
    } else if raw.len() >= (width * height) as usize {
        let gray = GrayImage::from_raw(width, height, raw[..(width * height) as usize].to_vec())?;
        DynamicImage::ImageLuma8(gray)
    } else {
        return None;
    };

    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    img.write_to(&mut cursor, image::ImageFormat::Jpeg).ok()?;
    Some(buf)
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

fn optimized_sibling_path(original: &Path) -> PathBuf {
    original.with_file_name(format!(
        "{}_optimized.pdf",
        original.file_stem().unwrap_or_else(|| std::ffi::OsStr::new("document")).to_string_lossy()
    ))
}

fn strip_and_compress(doc: &mut Document) -> Result<u32, String> {
    if let Ok(catalog) = doc.catalog_mut() {
        catalog.set(b"Metadata", Object::Null);
    }
    if let Ok(trailer) = doc.trailer.get_mut(b"Info") {
        *trailer = Object::Null;
    }
    let images_recompressed = recompress_images(doc)?;
    doc.prune_objects();
    doc.compress();
    Ok(images_recompressed)
}

/// Optimize `source` (the open working copy).
///
/// - `replace == false`: write `<original-stem>_optimized.pdf` next to `original`.
///   Leaves `source` and `original` unchanged.
/// - `replace == true`: overwrite `source`, then write the same bytes to `original`
///   so the open document and the file on disk stay in sync. No sibling is created.
pub fn optimize_pdf_file(source: &Path, original: &Path, replace: bool) -> Result<String, String> {
    let original = if original.as_os_str().is_empty() { source } else { original };
    let mut doc = Document::load(source).map_err(|e| e.to_string())?;
    let images_recompressed = strip_and_compress(&mut doc)?;
    let summary = format!(
        "Metadata stripped, objects pruned & streams compressed. {} image(s) recompressed.",
        images_recompressed
    );

    if replace {
        if crate::pdf::security::is_encrypted(original)? {
            return Err("Cannot replace a password-protected original. Save as a new file instead.".to_string());
        }
        // Write the working copy first. If the subsequent original write fails, Save
        // still persists the optimized bytes instead of clobbering the original with
        // the pre-optimize working copy.
        crate::pdf::io::save_atomic(&mut doc, source)?;
        crate::pdf::render::invalidate_document_cache(source);
        if !same_path(source, original) {
            crate::pdf::io::save_atomic(&mut doc, original)?;
            crate::pdf::render::invalidate_document_cache(original);
        }
        Ok(format!("Replaced {}. {summary}", original.display()))
    } else {
        let dest = optimized_sibling_path(original);
        crate::pdf::io::save_atomic(&mut doc, &dest)?;
        Ok(format!("Saved to {}. {summary}", dest.display()))
    }
}
