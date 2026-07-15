use crate::pdf::content::append_page_content;
use crate::pdf::coords::{finite_f64, viewer_rect_to_pdf};
use crate::pdf::edit_types::{PdfRect, TextStyle};
use crate::pdf::fonts::{ensure_font_family, measure_text_width, style_supports_text};
use crate::pdf::page_text::escape_pdf_literal_string;
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

pub fn validate_style_inputs(style: &TextStyle, box_rect: &PdfRect) -> Result<(), String> {
    finite_f64(style.font_size, "font size")?;
    finite_f64(box_rect.x, "box x")?;
    finite_f64(box_rect.y, "box y")?;
    finite_f64(box_rect.width, "box width")?;
    finite_f64(box_rect.height, "box height")?;
    if box_rect.width <= 0.0 || box_rect.height <= 0.0 {
        return Err("Box rect must have positive width and height".into());
    }
    finite_f64(style.color.r, "color r")?;
    finite_f64(style.color.g, "color g")?;
    finite_f64(style.color.b, "color b")?;
    if !(0.0..=1.0).contains(&style.color.r)
        || !(0.0..=1.0).contains(&style.color.g)
        || !(0.0..=1.0).contains(&style.color.b)
    {
        return Err("Color components must be in the range [0, 1]".into());
    }
    Ok(())
}

/// Wrap `text` into lines that fit inside `max_width` using a simple greedy
/// word-wrap algorithm. A single word that is wider than the box is placed on
/// its own line rather than discarded.
fn wrap_text_to_width(text: &str, font_family: &str, font_size: f64, max_width: f64) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw_line in text.lines() {
        let words: Vec<&str> = raw_line.split_whitespace().collect();
        if words.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in words {
            let candidate = if current.is_empty() { word.to_string() } else { format!("{} {}", current, word) };
            if measure_text_width(&candidate, font_family, font_size) <= max_width || current.is_empty() {
                current = candidate;
            } else {
                lines.push(current);
                current = word.to_string();
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    debug_assert!(!lines.is_empty(), "wrap_text_to_width should never return empty for non-empty text");
    lines
}

/// Add a new text box to a PDF page.
///
/// The caller is responsible for loading/saving the document; this helper is
/// intended to be invoked inside `mutate_pdf` from the Tauri command wrapper.
pub fn add_text_box(
    doc: &mut Document,
    page_index: u32,
    text: &str,
    style: &TextStyle,
    box_rect: &PdfRect,
) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Text cannot be empty".to_string());
    }
    if !(6.0..=72.0).contains(&style.font_size) {
        return Err("Font size must be between 6 and 72".to_string());
    }

    validate_style_inputs(style, box_rect)?;
    style_supports_text(style, trimmed)?;

    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    let font_name = ensure_font_family(doc, style, page_id)?;

    let (px, py, pw, ph) = viewer_rect_to_pdf(doc, page_id, box_rect.x, box_rect.y, box_rect.width, box_rect.height)?;

    let lines = wrap_text_to_width(trimmed, &style.font_family, style.font_size, pw);
    let line_height = style.font_size * 1.2;
    let top_margin = style.font_size;
    let required_height = top_margin + (lines.len().saturating_sub(1) as f64) * line_height;
    if required_height > ph {
        return Err("Box rect is too short for the wrapped text".to_string());
    }

    let align = style.align.to_lowercase();
    let mut ops = format!("q {r} {g} {b} rg {r} {g} {b} RG\n", r = style.color.r, g = style.color.g, b = style.color.b);

    for (i, line) in lines.iter().enumerate() {
        let line_width = measure_text_width(line, &style.font_family, style.font_size);
        let tx = match align.as_str() {
            "center" => (px + (pw - line_width) / 2.0).max(px),
            "right" => (px + pw - line_width).max(px),
            _ => px,
        };
        let baseline = py + ph - top_margin - (i as f64 * line_height);
        if baseline < py {
            return Err("Box rect is too short for the wrapped text".to_string());
        }

        let escaped = escape_pdf_literal_string(line);
        let text_matrix =
            if style.italic { format!("1 0 0.25 1 {tx} {baseline}") } else { format!("1 0 0 1 {tx} {baseline}") };
        ops.push_str(&format!(
            "BT /{font_name} {font_size} Tf {text_matrix} Tm ({escaped}) Tj ET\n",
            font_name = font_name,
            font_size = style.font_size,
        ));

        if style.bold {
            let bold_tx = tx + 0.5;
            let bold_matrix = if style.italic {
                format!("1 0 0.25 1 {bold_tx} {baseline}")
            } else {
                format!("1 0 0 1 {bold_tx} {baseline}")
            };
            ops.push_str(&format!(
                "BT /{font_name} {font_size} Tf {bold_matrix} Tm ({escaped}) Tj ET\n",
                font_name = font_name,
                font_size = style.font_size,
            ));
        }

        if style.underline {
            let uy = baseline - style.font_size * 0.15;
            let stroke_width = style.font_size * 0.05;
            let x_end = tx + line_width;
            ops.push_str(&format!("{tx} {uy} m {x_end} {uy} l {stroke_width} w S\n"));
        }
    }

    ops.push_str("Q\n");
    append_page_content(doc, page_id, ops.as_bytes())?;
    Ok(())
}
