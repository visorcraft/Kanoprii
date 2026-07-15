use crate::pdf::content::append_page_content;
use crate::pdf::coords::{finite_f64, viewer_rect_to_pdf};
use crate::pdf::edit_types::{PdfRect, TextStyle};
use crate::pdf::fonts::{ensure_font_family, measure_text_width, style_supports_text};
use crate::pdf::page_images::page_resources;
use crate::pdf::page_text::escape_pdf_literal_string;
use crate::pdf::text_lines::decode_page_text_lines;
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
    validate_rect_finite(box_rect, "box rect")?;
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

pub fn validate_rect_finite(rect: &PdfRect, name: &str) -> Result<(), String> {
    finite_f64(rect.x, &format!("{name} x"))?;
    finite_f64(rect.y, &format!("{name} y"))?;
    finite_f64(rect.width, &format!("{name} width"))?;
    finite_f64(rect.height, &format!("{name} height"))?;
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return Err(format!("{name} must have positive width and height"));
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
    let ops = render_wrapped_text_box(trimmed, style, &font_name, px, py, pw, ph)?;
    append_page_content(doc, page_id, ops.as_bytes())?;
    Ok(())
}

/// Render wrapped text inside a PDF rectangle and return the content-stream
/// operator string. Caller must append the returned ops to the page.
pub fn render_wrapped_text_box(
    text: &str,
    style: &TextStyle,
    font_name: &str,
    px: f64,
    py: f64,
    pw: f64,
    ph: f64,
) -> Result<String, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Text cannot be empty".to_string());
    }

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
    Ok(ops)
}

/// Edit a paragraph of existing text by whiteing-out the original lines and
/// rendering wrapped replacement text inside `box_rect`.
pub fn edit_paragraph(
    doc: &mut Document,
    page_index: u32,
    line_indices: &[usize],
    new_text: &str,
    style: &TextStyle,
    box_rect: &PdfRect,
) -> Result<(), String> {
    let trimmed = new_text.trim();
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

    let lines = decode_page_text_lines(doc, page_id)?;
    let mut sorted_indices: Vec<usize> = line_indices.to_vec();
    sorted_indices.sort();
    for &idx in &sorted_indices {
        if idx >= lines.len() {
            return Err("Line index out of range".to_string());
        }
    }

    whiteout_paragraph_lines(doc, page_id, &lines, &sorted_indices)?;

    let (px, py, pw, ph) = viewer_rect_to_pdf(doc, page_id, box_rect.x, box_rect.y, box_rect.width, box_rect.height)?;
    let ops = render_wrapped_text_box(trimmed, style, &font_name, px, py, pw, ph)?;
    append_page_content(doc, page_id, ops.as_bytes())?;
    Ok(())
}

fn whiteout_paragraph_lines(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    lines: &[crate::pdf::text_lines::TextLine],
    sorted_indices: &[usize],
) -> Result<(), String> {
    let mut left = f64::MAX;
    let mut bottom = f64::MAX;
    let mut right = f64::MIN;
    let mut top = f64::MIN;
    for &idx in sorted_indices {
        let [l, b, r, t] = lines[idx].bbox;
        left = left.min(l);
        bottom = bottom.min(b);
        right = right.max(r);
        top = top.max(t);
    }
    let w = (right - left).max(1.0);
    let h = (top - bottom).max(1.0);
    let whiteout = format!("q 1 1 1 rg {left} {bottom} {w} {h} re f Q\n");
    append_page_content(doc, page_id, whiteout.as_bytes())?;
    Ok(())
}

/// Remove a paragraph's text by whiteing-out the union bbox of its lines.
pub fn delete_paragraph(doc: &mut Document, page_index: u32, line_indices: &[usize]) -> Result<(), String> {
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    let lines = decode_page_text_lines(doc, page_id)?;
    let mut sorted_indices: Vec<usize> = line_indices.to_vec();
    sorted_indices.sort();
    for &idx in &sorted_indices {
        if idx >= lines.len() {
            return Err("Line index out of range".to_string());
        }
    }
    whiteout_paragraph_lines(doc, page_id, &lines, &sorted_indices)?;
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
    validate_rect_finite(new_rect, "new rect")?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::edit_types::RgbColor;

    #[test]
    fn validate_rect_finite_accepts_positive_box() {
        validate_rect_finite(&PdfRect { x: 0.0, y: 0.0, width: 100.0, height: 50.0 }, "rect").unwrap();
    }

    #[test]
    fn validate_rect_finite_rejects_non_positive_dimensions() {
        let err = validate_rect_finite(&PdfRect { x: 0.0, y: 0.0, width: 0.0, height: 50.0 }, "rect").unwrap_err();
        assert!(err.contains("positive width"));
    }

    #[test]
    fn validate_rect_finite_rejects_nan() {
        let err =
            validate_rect_finite(&PdfRect { x: f64::NAN, y: 0.0, width: 100.0, height: 50.0 }, "rect").unwrap_err();
        assert!(err.contains("must be a finite number"));
    }

    #[test]
    fn validate_rect_finite_rejects_infinity() {
        let err = validate_rect_finite(&PdfRect { x: 0.0, y: f64::INFINITY, width: 100.0, height: 50.0 }, "rect")
            .unwrap_err();
        assert!(err.contains("must be a finite number"));
    }

    fn build_doc_with_text(content_ops: &str) -> (lopdf::Document, lopdf::ObjectId) {
        let mut doc = lopdf::Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();

        let content_bytes = content_ops.as_bytes().to_vec();
        doc.set_object(content_id, lopdf::Object::Stream(lopdf::Stream::new(lopdf::Dictionary::new(), content_bytes)));

        doc.set_object(
            page_id,
            lopdf::Object::Dictionary(lopdf::Dictionary::from_iter(vec![
                (b"Type".to_vec(), lopdf::Object::Name(b"Page".to_vec())),
                (b"Parent".to_vec(), lopdf::Object::Reference(pages_id)),
                (
                    b"MediaBox".to_vec(),
                    lopdf::Object::Array(vec![
                        lopdf::Object::Integer(0),
                        lopdf::Object::Integer(0),
                        lopdf::Object::Integer(612),
                        lopdf::Object::Integer(792),
                    ]),
                ),
                (b"Contents".to_vec(), lopdf::Object::Reference(content_id)),
                (
                    b"Resources".to_vec(),
                    lopdf::Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                        b"Font".to_vec(),
                        lopdf::Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                            b"F1".to_vec(),
                            lopdf::Object::Dictionary(lopdf::Dictionary::from_iter(vec![
                                (b"Type".to_vec(), lopdf::Object::Name(b"Font".to_vec())),
                                (b"Subtype".to_vec(), lopdf::Object::Name(b"Type1".to_vec())),
                                (b"BaseFont".to_vec(), lopdf::Object::Name(b"Helvetica".to_vec())),
                            ])),
                        )])),
                    )])),
                ),
            ])),
        );

        doc.set_object(
            pages_id,
            lopdf::Object::Dictionary(lopdf::Dictionary::from_iter(vec![
                (b"Type".to_vec(), lopdf::Object::Name(b"Pages".to_vec())),
                (b"Kids".to_vec(), lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)])),
                (b"Count".to_vec(), lopdf::Object::Integer(1)),
            ])),
        );

        let catalog_id = doc.new_object_id();
        doc.set_object(
            catalog_id,
            lopdf::Object::Dictionary(lopdf::Dictionary::from_iter(vec![
                (b"Type".to_vec(), lopdf::Object::Name(b"Catalog".to_vec())),
                (b"Pages".to_vec(), lopdf::Object::Reference(pages_id)),
            ])),
        );
        doc.trailer.set(b"Root", lopdf::Object::Reference(catalog_id));
        (doc, page_id)
    }

    fn full_page_box() -> PdfRect {
        PdfRect { x: 0.0, y: 0.0, width: 800.0, height: 1132.0 }
    }

    fn style_with_align(align: &str) -> TextStyle {
        TextStyle {
            font_family: "Helvetica".to_string(),
            font_size: 12.0,
            bold: false,
            italic: false,
            underline: false,
            color: RgbColor { r: 0.1, g: 0.2, b: 0.3 },
            align: align.to_string(),
        }
    }

    #[test]
    fn edit_paragraph_replaces_two_lines() {
        let ops =
            "BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello world) Tj ET\nBT /F1 12 Tf 1 0 0 1 100 686 Tm (Second line) Tj ET";
        let (mut doc, page_id) = build_doc_with_text(ops);
        let style = style_with_align("left");
        edit_paragraph(&mut doc, 0, &[0, 1], "Replaced paragraph", &style, &full_page_box()).unwrap();
        let content =
            String::from_utf8_lossy(&crate::pdf::page_text::read_page_content(&doc, page_id).unwrap()).into_owned();
        assert!(content.contains("Replaced"));
        assert!(content.contains("q 1 1 1 rg"));
    }

    #[test]
    fn edit_paragraph_empty_text_fails() {
        let (mut doc, _) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET");
        let result = edit_paragraph(&mut doc, 0, &[0], "   ", &style_with_align("left"), &full_page_box());
        assert!(result.is_err());
    }

    #[test]
    fn edit_paragraph_out_of_range_line_fails() {
        let (mut doc, _) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET");
        let result = edit_paragraph(&mut doc, 0, &[0, 5], "Text", &style_with_align("left"), &full_page_box());
        assert!(result.is_err());
    }
}
