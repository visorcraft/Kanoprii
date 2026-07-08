use crate::pdf::annotations::redaction_rects_on_page;
use crate::pdf::content::embed_jpeg_xobject;
use crate::pdf::coords::{page_media_box, pdf_rect_to_render_px};
use crate::pdf::export::{EXPORT_RENDER_H, EXPORT_RENDER_W};
use crate::pdf::io::mutate_pdf;
use crate::pdf::ocr_layer::make_pdf_searchable;
use crate::pdf::pdfium_bind::get_pdfium;
use crate::pdf::render;
use image::{Rgb, RgbImage};
use lopdf::{Dictionary, Document, Object, Stream};
use std::path::Path;

pub type RedactionPageRects = Vec<(u32, Vec<[f64; 4]>)>;

pub fn pages_with_redactions(path: &Path) -> Result<RedactionPageRects, String> {
    let doc = Document::load(path).map_err(|e| e.to_string())?;
    Ok(pages_with_redactions_in_doc(&doc))
}

pub(crate) fn pages_with_redactions_in_doc(doc: &Document) -> RedactionPageRects {
    let pages = doc.get_pages();
    let mut page_nums: Vec<u32> = pages.keys().copied().collect();
    page_nums.sort_unstable();
    let mut out = Vec::new();
    for page_num in page_nums {
        let page_index = page_num.saturating_sub(1);
        let page_id = *pages.get(&page_num).unwrap_or_else(|| unreachable!());
        let rects = redaction_rects_on_page(doc, page_id);
        if !rects.is_empty() {
            out.push((page_index, rects));
        }
    }
    out
}

pub fn has_redaction_boxes(path: &Path) -> Result<bool, String> {
    Ok(!pages_with_redactions(path)?.is_empty())
}

pub fn render_page_redacted(path: &Path, page_index: u32, rects_pdf: &[[f64; 4]]) -> Result<RgbImage, String> {
    let doc = Document::load(path).map_err(|e| e.to_string())?;
    render_page_redacted_with_doc(&doc, path, page_index, rects_pdf)
}

pub(crate) fn render_page_redacted_with_doc(
    doc: &Document,
    render_path: &Path,
    page_index: u32,
    rects_pdf: &[[f64; 4]],
) -> Result<RgbImage, String> {
    let png = {
        let pdfium = get_pdfium()?;
        render::render_page_bytes(
            &pdfium,
            render_path,
            page_index,
            EXPORT_RENDER_W,
            EXPORT_RENDER_H,
            image::ImageFormat::Png,
        )?
    };
    let mut img = image::load_from_memory(&png).map_err(|e| e.to_string())?.to_rgb8();

    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    let media = page_media_box(doc, page_id)?;
    let page_w = (media[2] - media[0]) as f32;
    let page_h = (media[3] - media[1]) as f32;
    let render_w = f64::from(EXPORT_RENDER_W);
    let render_h = f64::from(EXPORT_RENDER_H);

    for rect in rects_pdf {
        let (x, y, w, h) = pdf_rect_to_render_px(*rect, page_w, page_h, render_w, render_h);
        paint_black_rect(&mut img, x, y, w, h);
    }
    Ok(img)
}

fn paint_black_rect(img: &mut RgbImage, x: i32, y: i32, w: i32, h: i32) {
    let (iw, ih) = img.dimensions();
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(iw as i32);
    let y1 = (y + h).min(ih as i32);
    for py in y0..y1 {
        for px in x0..x1 {
            img.put_pixel(px as u32, py as u32, Rgb([0, 0, 0]));
        }
    }
}

pub fn replace_page_with_render(path: &Path, page_index: u32, img: &RgbImage) -> Result<(), String> {
    let mut jpeg = Vec::new();
    image::DynamicImage::ImageRgb8(img.clone())
        .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
        .map_err(|e| e.to_string())?;

    mutate_pdf(path, |doc| replace_page_with_render_in_doc(doc, page_index, img, jpeg.clone()))
}

pub(crate) fn replace_page_with_render_in_doc(
    doc: &mut Document,
    page_index: u32,
    img: &RgbImage,
    jpeg: Vec<u8>,
) -> Result<(), String> {
    let (img_w, img_h) = img.dimensions();
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    let media = page_media_box(doc, page_id)?;
    let mw = media[2] - media[0];
    let mh = media[3] - media[1];

    let image_id = embed_jpeg_xobject(doc, jpeg, img_w, img_h);
    let mut xobjects = Dictionary::new();
    xobjects.set(b"Im1", Object::Reference(image_id));
    let mut resources = Dictionary::new();
    resources.set(b"XObject", Object::Dictionary(xobjects));

    let ops = format!("q {mw} 0 0 {mh} 0 0 cm /Im1 Do Q\n", mw = mw, mh = mh);
    let content_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), ops.into_bytes())));

    let page_dict = doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?;
    page_dict.set(b"Contents", Object::Reference(content_id));
    page_dict.set(b"Resources", Object::Dictionary(resources));
    page_dict.remove(b"Annots");
    doc.prune_objects();
    Ok(())
}

pub fn apply_redactions(path: &Path, ocr_after: bool) -> Result<u32, String> {
    let inventory = pages_with_redactions(path)?;
    if inventory.is_empty() {
        return Err("No redaction boxes found in this document".to_string());
    }

    let mut affected = 0u32;
    for (page_index, rects) in &inventory {
        let img = render_page_redacted(path, *page_index, rects)?;
        replace_page_with_render(path, *page_index, &img)?;
        affected += 1;
    }

    if ocr_after && affected > 0 {
        let start = inventory.first().map(|(p, _)| *p).unwrap_or(0);
        let end = inventory.last().map(|(p, _)| *p).unwrap_or(start);
        let _ = make_pdf_searchable(path, start, end)?;
    }

    Ok(affected)
}

/// Apply redactions to an in-memory document. A temporary file is used for
/// rendering because the PDFium binding loads documents from disk.
pub(crate) fn apply_redactions_to_doc(doc: &mut Document) -> Result<u32, String> {
    let inventory = pages_with_redactions_in_doc(doc);
    if inventory.is_empty() {
        return Ok(0);
    }

    let nanos =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let counter = REDACT_TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temp_name = format!("kanoprii_redact_print_{}_{}_{}.pdf", std::process::id(), nanos, counter);
    let temp_path = std::env::temp_dir().join(&temp_name);
    crate::pdf::io::save_atomic(doc, &temp_path)?;

    // ponytail: RAII guard so the *un-redacted* temp source PDF is removed
    // even if rendering panics or returns Err mid-loop. A leaked temp file
    // with a predictable name is a data-leak hazard.
    struct TempGuard(std::path::PathBuf);
    impl Drop for TempGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _guard = TempGuard(temp_path.clone());

    let mut affected = 0u32;
    for (page_index, rects) in &inventory {
        let img = render_page_redacted_with_doc(doc, &temp_path, *page_index, rects)?;
        let mut jpeg = Vec::new();
        image::DynamicImage::ImageRgb8(img.clone())
            .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
            .map_err(|e| e.to_string())?;
        replace_page_with_render_in_doc(doc, *page_index, &img, jpeg)?;
        affected += 1;
    }
    Ok(affected)
}

static REDACT_TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::coords::{VIEWER_PAGE_H, VIEWER_PAGE_W};

    #[test]
    fn redaction_pixel_rect_maps_and_flips() {
        let (x, y, w, h) = pdf_rect_to_render_px([72.0, 700.0, 200.0, 720.0], 612.0, 792.0, 1600.0, 2264.0);
        let viewer = crate::pdf::coords::pdf_rect_to_viewer_px(72.0, 700.0, 200.0, 720.0, 612.0, 792.0);
        let sx = 1600.0 / VIEWER_PAGE_W;
        let sy = 2264.0 / VIEWER_PAGE_H;
        assert_eq!(x, (viewer[0] * sx).round() as i32);
        assert_eq!(y, (viewer[1] * sy).round() as i32);
        assert!(w > 0);
        assert!(h > 0);
    }
}
