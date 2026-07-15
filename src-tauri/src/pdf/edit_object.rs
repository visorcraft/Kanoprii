use crate::pdf::content::append_page_content;
use crate::pdf::coords::{finite_f64, viewer_rect_to_pdf};
use crate::pdf::edit_types::{PdfRect, TextStyle};
use crate::pdf::fonts::{ensure_font_family, measure_text_width, style_supports_text};
use crate::pdf::page_images::page_resources;
use crate::pdf::page_text::escape_pdf_literal_string;
use crate::pdf::text_replace::replace_text_line_styled;
use lopdf::{Document, Object};

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

/// Transform the page-space rectangle of an image drawn with a simple
/// `q ... cm Do Q` pattern.
///
/// Phase 1 limitation: only images whose preceding operator is `cm` can be
/// transformed. More complex nested or scaled patterns return an error.
pub fn transform_page_image(
    doc: &mut Document,
    page_index: u32,
    image_index: usize,
    new_rect: &PdfRect,
) -> Result<(), String> {
    let page_id = doc.page_iter().nth(page_index as usize).ok_or_else(|| "page index out of range".to_string())?;
    let contents = page_contents_id(doc, page_id)?;
    let content_obj = doc.get_object(contents).map_err(|e| e.to_string())?;
    let mut content = content_obj
        .as_stream()
        .map_err(|_| "content not stream".to_string())?
        .decode_content()
        .map_err(|e| e.to_string())?;

    let images = crate::pdf::page_images::list_page_images(doc, page_index)?;
    let target = images.get(image_index).ok_or_else(|| "image index out of range".to_string())?;
    let target_name = find_xobject_name(doc, page_index, target.object_id)?;

    let mut found = false;
    for (i, op) in content.operations.iter().enumerate() {
        if op.operator == "Do"
            && op.operands.first().and_then(|o| o.as_name().ok()) == Some(target_name.as_bytes())
            && i > 0
            && content.operations[i - 1].operator == "cm"
        {
            content.operations[i - 1].operands = vec![
                Object::Real(new_rect.width as f32),
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(new_rect.height as f32),
                Object::Real(new_rect.x as f32),
                Object::Real(new_rect.y as f32),
            ];
            found = true;
            break;
        }
    }

    if !found {
        return Err("image transform not supported for this content pattern".to_string());
    }

    let encoded = content.encode().map_err(|e| e.to_string())?;
    doc.set_object(contents, Object::Stream(lopdf::Stream::new(lopdf::Dictionary::new(), encoded)));
    Ok(())
}

/// Remove an image drawn with a simple `q ... cm Do Q` pattern.
///
/// Phase 1: removes the `cm Do` pair and, when present, the surrounding
/// `q ... Q` wrapper. Complex nested patterns return an error.
pub fn remove_page_image(doc: &mut Document, page_index: u32, image_index: usize) -> Result<(), String> {
    let page_id = doc.page_iter().nth(page_index as usize).ok_or_else(|| "page index out of range".to_string())?;
    let contents = page_contents_id(doc, page_id)?;
    let content_obj = doc.get_object(contents).map_err(|e| e.to_string())?;
    let mut content = content_obj
        .as_stream()
        .map_err(|_| "content not stream".to_string())?
        .decode_content()
        .map_err(|e| e.to_string())?;

    let images = crate::pdf::page_images::list_page_images(doc, page_index)?;
    let target = images.get(image_index).ok_or_else(|| "image index out of range".to_string())?;
    let target_name = find_xobject_name(doc, page_index, target.object_id)?;

    let do_idx = content
        .operations
        .iter()
        .position(|op| {
            op.operator == "Do" && op.operands.first().and_then(|o| o.as_name().ok()) == Some(target_name.as_bytes())
        })
        .ok_or_else(|| "image Do operator not found".to_string())?;

    // Determine the range to remove. Phase 1 supports q <cm> Do Q.
    let mut start = do_idx;
    let mut end = do_idx;
    if do_idx > 0 && content.operations[do_idx - 1].operator == "cm" {
        start = do_idx - 1;
        if do_idx > 1 && content.operations[do_idx - 2].operator == "q" {
            start = do_idx - 2;
        }
        if do_idx + 1 < content.operations.len() && content.operations[do_idx + 1].operator == "Q" {
            end = do_idx + 1;
        }
    }

    content.operations.drain(start..=end);

    let encoded = content.encode().map_err(|e| e.to_string())?;
    doc.set_object(contents, Object::Stream(lopdf::Stream::new(lopdf::Dictionary::new(), encoded)));
    Ok(())
}

fn page_contents_id(doc: &Document, page_id: lopdf::ObjectId) -> Result<lopdf::ObjectId, String> {
    let dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;
    let obj = dict.get(b"Contents").map_err(|_| "page has no content stream".to_string())?;
    match obj {
        Object::Reference(id) => Ok(*id),
        Object::Array(_) => Err("page content array not supported for image editing".to_string()),
        _ => Err("page has no content stream".to_string()),
    }
}

fn find_xobject_name(doc: &Document, page_index: u32, object_id: (u32, u16)) -> Result<String, String> {
    let page_id = doc.page_iter().nth(page_index as usize).ok_or_else(|| "page index out of range".to_string())?;
    let resources = page_resources(doc, page_id)?;
    let xobjects = resources.get(b"XObject").map_err(|_| "missing XObject resources".to_string())?;
    let xobjects = xobjects.as_dict().map_err(|_| "XObject not dict".to_string())?;
    for (name, obj) in xobjects.iter() {
        if obj.as_reference().map(|id| id == object_id).unwrap_or(false) {
            return String::from_utf8(name.clone()).map_err(|_| "invalid xobject name".to_string());
        }
    }
    Err("image xobject name not found".to_string())
}
