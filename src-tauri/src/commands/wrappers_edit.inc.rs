use crate::pdf::edit_object::validate_rect_finite;
use crate::pdf::edit_types::{PageImageInfo, PdfRect, TextStyle};

#[tauri::command]
fn edit_text_line(
    path: String,
    page_index: u32,
    line_index: usize,
    new_text: String,
    style: TextStyle,
    box_rect: PdfRect,
) -> Result<(), String> {
    crate::pdf::io::mutate_pdf(&PathBuf::from(path), |doc| {
        crate::pdf::edit_object::edit_text_line(doc, page_index, line_index, &new_text, &style, &box_rect)
    })
}

#[tauri::command]
fn add_text_box(
    path: String,
    page_index: u32,
    text: String,
    style: TextStyle,
    box_rect: PdfRect,
) -> Result<(), String> {
    crate::pdf::io::mutate_pdf(&PathBuf::from(path), |doc| {
        crate::pdf::edit_object::add_text_box(doc, page_index, &text, &style, &box_rect)
    })
}

#[tauri::command]
fn list_page_images(path: String, page_index: u32) -> Result<Vec<PageImageInfo>, String> {
    crate::pdf::io::with_pdf(&PathBuf::from(path), |doc| {
        crate::pdf::page_images::list_page_images(doc, page_index)
    })
}

#[tauri::command]
fn transform_page_image(
    path: String,
    page_index: u32,
    image_index: usize,
    new_rect: PdfRect,
) -> Result<(), String> {
    crate::pdf::io::mutate_pdf(&PathBuf::from(path), |doc| {
        crate::pdf::edit_object::transform_page_image(doc, page_index, image_index, &new_rect)
    })
}

#[tauri::command]
fn remove_page_image(path: String, page_index: u32, image_index: usize) -> Result<(), String> {
    crate::pdf::io::mutate_pdf(&PathBuf::from(path), |doc| {
        crate::pdf::edit_object::remove_page_image(doc, page_index, image_index)
    })
}

#[tauri::command]
fn edit_paragraph(
    path: String,
    page_index: u32,
    line_indices: Vec<usize>,
    new_text: String,
    style: TextStyle,
    box_rect: PdfRect,
) -> Result<(), String> {
    crate::pdf::io::mutate_pdf(&PathBuf::from(path), |doc| {
        crate::pdf::edit_object::edit_paragraph(doc, page_index, &line_indices, &new_text, &style, &box_rect)
    })
}

#[tauri::command]
fn delete_paragraph(path: String, page_index: u32, line_indices: Vec<usize>) -> Result<(), String> {
    crate::pdf::io::mutate_pdf(&PathBuf::from(path), |doc| {
        crate::pdf::edit_object::delete_paragraph(doc, page_index, &line_indices)
    })
}

#[tauri::command]
fn find_paragraph(path: String, page_index: u32, line_index: usize) -> Result<Option<ParagraphInfo>, String> {
    crate::pdf::io::with_pdf(&PathBuf::from(path), |doc| {
        let page_id = doc
            .page_iter()
            .nth(page_index as usize)
            .ok_or_else(|| "page index out of range".to_string())?;
        let lines = crate::pdf::text_lines::decode_page_text_lines(doc, page_id)?;
        let paragraph = crate::pdf::paragraphs::find_paragraph_for_line(&lines, line_index);
        match paragraph {
            Some(p) if p.line_indices.len() > 1 => {
                let media = crate::pdf::coords::page_media_box(doc, page_id)?;
                let page_w = (media[2] - media[0]).max(1.0) as f32;
                let page_h = (media[3] - media[1]).max(1.0) as f32;
                let [left, bottom, right, top] = p.bbox;
                let viewer = crate::pdf::coords::pdf_rect_to_viewer_px(left, bottom, right, top, page_w, page_h);
                Ok(Some(ParagraphInfo {
                    line_indices: p.line_indices,
                    x: viewer[0],
                    y: viewer[1],
                    w: (viewer[2] - viewer[0]).max(1.0),
                    h: (viewer[3] - viewer[1]).max(1.0),
                }))
            }
            _ => Ok(None),
        }
    })
}

#[tauri::command]
fn viewer_rect_to_pdf(path: String, page_index: u32, rect: PdfRect) -> Result<PdfRect, String> {
    validate_rect_finite(&rect, "rect")?;
    crate::pdf::io::with_pdf(&PathBuf::from(path), |doc| {
        let page_id = doc
            .page_iter()
            .nth(page_index as usize)
            .ok_or_else(|| "page index out of range".to_string())?;
        let (x, y, w, h) =
            crate::pdf::coords::viewer_rect_to_pdf(doc, page_id, rect.x, rect.y, rect.width, rect.height)?;
        Ok(PdfRect {
            x,
            y,
            width: w,
            height: h,
        })
    })
}

#[tauri::command]
fn pdf_rect_to_viewer_px(path: String, page_index: u32, rect: PdfRect) -> Result<PdfRect, String> {
    validate_rect_finite(&rect, "rect")?;
    crate::pdf::io::with_pdf(&PathBuf::from(path), |doc| {
        let page_id = doc
            .page_iter()
            .nth(page_index as usize)
            .ok_or_else(|| "page index out of range".to_string())?;
        let media = crate::pdf::coords::page_media_box(doc, page_id)?;
        let page_w = (media[2] - media[0]) as f32;
        let page_h = (media[3] - media[1]) as f32;
        let viewer = crate::pdf::coords::pdf_rect_to_viewer_px(
            rect.x,
            rect.y,
            rect.x + rect.width,
            rect.y + rect.height,
            page_w,
            page_h,
        );
        Ok(PdfRect {
            x: viewer[0],
            y: viewer[1],
            width: viewer[2] - viewer[0],
            height: viewer[3] - viewer[1],
        })
    })
}
