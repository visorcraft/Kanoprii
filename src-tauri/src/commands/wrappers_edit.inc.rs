use crate::pdf::edit_types::{PdfRect, TextStyle};

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
    let _ = (path, page_index, text, style, box_rect);
    Ok(())
}

#[tauri::command]
fn list_page_images(path: String, page_index: u32) -> Result<Vec<()>, String> {
    let _ = (path, page_index);
    Ok(vec![])
}

#[tauri::command]
fn transform_page_image(
    path: String,
    page_index: u32,
    image_index: usize,
    new_rect: PdfRect,
) -> Result<(), String> {
    let _ = (path, page_index, image_index, new_rect);
    Ok(())
}

#[tauri::command]
fn remove_page_image(path: String, page_index: u32, image_index: usize) -> Result<(), String> {
    let _ = (path, page_index, image_index);
    Ok(())
}
