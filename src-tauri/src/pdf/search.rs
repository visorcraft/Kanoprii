use crate::pdf::coords::pdf_rect_to_viewer_px;
use pdfium_render::prelude::*;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct PdfTextSearchMatch {
    pub page_index: u32,
    pub match_index: u32,
    pub rect: [f64; 4],
}

pub fn pdf_rect_to_search_pixels(rect: PdfRect, page_w: f32, page_h: f32) -> [f64; 4] {
    pdf_rect_to_viewer_px(
        f64::from(rect.left().value),
        f64::from(rect.bottom().value),
        f64::from(rect.right().value),
        f64::from(rect.top().value),
        page_w,
        page_h,
    )
}

pub fn union_search_bounds(segments: &PdfPageTextSegments<'_>) -> Result<PdfRect, String> {
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    for idx in segments.as_range() {
        let seg = segments.get(idx).map_err(|e| e.to_string())?;
        let b = seg.bounds();
        let l = b.left().value;
        let r = b.right().value;
        let bo = b.bottom().value;
        let t = b.top().value;
        bounds = Some(match bounds {
            None => (l, bo, r, t),
            Some((ul, ubo, ur, ut)) => (ul.min(l), ubo.min(bo), ur.max(r), ut.max(t)),
        });
    }
    let (l, bo, r, t) = bounds.ok_or_else(|| "Empty search result".to_string())?;
    Ok(PdfRect::new(PdfPoints::new(bo), PdfPoints::new(l), PdfPoints::new(t), PdfPoints::new(r)))
}

pub fn search_pdf_text(
    pdfium: &Pdfium,
    path: &Path,
    query: &str,
    match_case: bool,
    match_whole_word: bool,
) -> Result<Vec<PdfTextSearchMatch>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Search query is empty".to_string());
    }
    if !path.is_file() {
        return Err("File not found".to_string());
    }

    let document = pdfium.load_pdf_from_file(path, None).map_err(|e| e.to_string())?;
    let options = PdfSearchOptions::new().match_case(match_case).match_whole_word(match_whole_word);
    let mut results = Vec::new();
    let mut match_index = 0u32;

    for (page_index, page) in document.pages().iter().enumerate() {
        let text = page.text().map_err(|e| e.to_string())?;
        let search = text.search(query, &options).map_err(|e| e.to_string())?;
        let page_w = page.width().value;
        let page_h = page.height().value;

        for segments in search.iter(PdfSearchDirection::SearchForward) {
            let bounds = union_search_bounds(&segments)?;
            results.push(PdfTextSearchMatch {
                page_index: page_index as u32,
                match_index,
                rect: pdf_rect_to_search_pixels(bounds, page_w, page_h),
            });
            match_index += 1;
        }
    }

    Ok(results)
}
