use crate::pdf::edit_types::{PdfRect, TextStyle};
use crate::pdf::text_replace::replace_text_line_styled;
use lopdf::Document;

/// Edit an existing text line in a PDF document.
///
/// The caller is responsible for loading/saving the document; this helper is
/// intended to be invoked inside `mutate_pdf` from the Tauri command wrapper.
pub fn edit_text_line(
    doc: &mut Document,
    page_index: u32,
    line_index: usize,
    new_text: &str,
    style: &TextStyle,
    box_rect: &PdfRect,
) -> Result<(), String> {
    replace_text_line_styled(doc, page_index, line_index, new_text, style, box_rect)
}
