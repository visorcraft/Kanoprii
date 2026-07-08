use lopdf::{Document, Object};
use std::path::Path;

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

pub fn optimize_pdf_file(path: &Path) -> Result<String, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;

    if let Ok(catalog) = doc.catalog_mut() {
        catalog.set(b"Metadata", Object::Null);
    }

    if let Ok(trailer) = doc.trailer.get_mut(b"Info") {
        *trailer = Object::Null;
    }

    let images_recompressed = recompress_images(&mut doc)?;

    doc.prune_objects();
    doc.compress();

    let output_path = path.with_file_name(format!(
        "{}_optimized.pdf",
        path.file_stem().unwrap_or_else(|| std::ffi::OsStr::new("document")).to_string_lossy()
    ));
    crate::pdf::io::save_atomic(&mut doc, &output_path)?;

    Ok(format!(
        "Saved to {}. Metadata stripped, objects pruned & streams compressed. {} image(s) recompressed.",
        output_path.file_name().unwrap().to_string_lossy(),
        images_recompressed
    ))
}
