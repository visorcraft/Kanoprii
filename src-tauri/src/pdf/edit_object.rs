use crate::pdf::content::append_page_content;
use crate::pdf::coords::{finite_f64, viewer_rect_to_pdf};
use crate::pdf::edit_types::{PdfRect, TextStyle};
use crate::pdf::fonts::{ensure_font_family, measure_text_width, style_supports_text, uses_synthetic_font_style};
use crate::pdf::page_images::page_resources;
use crate::pdf::page_text::escape_pdf_literal_string;
use crate::pdf::text_lines::decode_page_text_lines;
use crate::pdf::text_replace::replace_text_line_styled;
use lopdf::{Document, Object};
use std::path::Path;

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
    let synthetic_style = uses_synthetic_font_style(style);
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
        let text_matrix = if synthetic_style && style.italic {
            format!("1 0 0.25 1 {tx} {baseline}")
        } else {
            format!("1 0 0 1 {tx} {baseline}")
        };
        ops.push_str(&format!(
            "BT /{font_name} {font_size} Tf {text_matrix} Tm ({escaped}) Tj ET\n",
            font_name = font_name,
            font_size = style.font_size,
        ));

        if synthetic_style && style.bold {
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

/// Remove a single existing text line by whiteing-out its bbox.
pub fn delete_text_line(doc: &mut Document, page_index: u32, line_index: usize) -> Result<(), String> {
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    let lines = decode_page_text_lines(doc, page_id)?;
    if line_index >= lines.len() {
        return Err("Line index out of range".to_string());
    }
    let [left, bottom, right, top] = lines[line_index].bbox;
    let w = (right - left).max(1.0);
    let h = (top - bottom).max(1.0);
    let whiteout = format!("q 1 1 1 rg {left} {bottom} {w} {h} re f Q\n");
    append_page_content(doc, page_id, whiteout.as_bytes())?;
    Ok(())
}

fn whiteout_viewer_rect(doc: &mut Document, page_id: lopdf::ObjectId, rect: &PdfRect) -> Result<(), String> {
    validate_rect_finite(rect, "source rect")?;
    let (x, y, width, height) = viewer_rect_to_pdf(doc, page_id, rect.x, rect.y, rect.width, rect.height)?;
    append_page_content(doc, page_id, format!("q 1 1 1 rg {x} {y} {width} {height} re f Q\n").as_bytes())
}

/// Replace text located by PDFium when the content-stream decoder cannot address a line.
pub fn edit_text_region(
    doc: &mut Document,
    page_index: u32,
    source_rect: &PdfRect,
    new_text: &str,
    style: &TextStyle,
    box_rect: &PdfRect,
) -> Result<(), String> {
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    whiteout_viewer_rect(doc, page_id, source_rect)?;
    add_text_box(doc, page_index, new_text, style, box_rect)
}

pub fn delete_text_region(doc: &mut Document, page_index: u32, source_rect: &PdfRect) -> Result<(), String> {
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    whiteout_viewer_rect(doc, page_id, source_rect)
}

/// Move, resize, or rotate one page-level image occurrence.
pub fn transform_page_image(
    doc: &mut Document,
    page_index: u32,
    image_index: usize,
    new_rect: &PdfRect,
    rotation_degrees: f64,
) -> Result<(), String> {
    validate_rect_finite(new_rect, "new rect")?;
    finite_f64(rotation_degrees, "rotation")?;
    let page_id = doc.page_iter().nth(page_index as usize).ok_or_else(|| "page index out of range".to_string())?;
    let images = crate::pdf::page_images::list_page_images(doc, page_index)?;
    let target = images.get(image_index).ok_or_else(|| "image index out of range".to_string())?;

    // Build the `cm` matrix that maps the unit image square onto a rectangle
    // of size (w, h), rotated by theta about its center (cx, cy). The first
    // two columns (a, b) and (c, d) are the transformed axes; (e, f) translate
    // the result so the rectangle stays centered at (cx, cy).
    let theta = rotation_degrees.to_radians();
    let (sin, cos) = theta.sin_cos();
    let w = new_rect.width;
    let h = new_rect.height;
    let cx = new_rect.x + w / 2.0;
    let cy = new_rect.y + h / 2.0;
    let a = w * cos;
    let b = w * sin;
    let c = -h * sin;
    let d = h * cos;
    let e = cx - (a + c) / 2.0;
    let f = cy - (b + d) / 2.0;

    // Avoid negative zero in encoded content streams.
    let sanitize = |v: f64| if v.abs() < 1e-12 { 0.0 } else { v };
    let a = sanitize(a);
    let b = sanitize(b);
    let c = sanitize(c);
    let d = sanitize(d);
    let e = sanitize(e);
    let f = sanitize(f);

    rewrite_image_do(doc, page_id, &target.resource_name, target.occurrence, None)?;
    let ops = format!("q {a} {b} {c} {d} {e} {f} cm /{} Do Q\n", target.resource_name);
    append_page_content(doc, page_id, ops.as_bytes())?;
    Ok(())
}

/// Remove one page-level image occurrence.
pub fn remove_page_image(doc: &mut Document, page_index: u32, image_index: usize) -> Result<(), String> {
    let page_id = doc.page_iter().nth(page_index as usize).ok_or_else(|| "page index out of range".to_string())?;
    let images = crate::pdf::page_images::list_page_images(doc, page_index)?;
    let target = images.get(image_index).ok_or_else(|| "image index out of range".to_string())?;
    rewrite_image_do(doc, page_id, &target.resource_name, target.occurrence, None)
}

/// Replace one image occurrence while leaving other uses of the same XObject unchanged.
pub fn replace_page_image(
    doc: &mut Document,
    page_index: u32,
    image_index: usize,
    image_path: &Path,
) -> Result<(), String> {
    if !image_path.is_file() {
        return Err("Image file not found".to_string());
    }
    let image = image::open(image_path).map_err(|e| e.to_string())?.to_rgb8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Err("Image has no pixels".to_string());
    }
    let mut jpeg = Vec::new();
    image::DynamicImage::ImageRgb8(image)
        .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
        .map_err(|e| e.to_string())?;

    let page_id = doc.page_iter().nth(page_index as usize).ok_or_else(|| "page index out of range".to_string())?;
    let images = crate::pdf::page_images::list_page_images(doc, page_index)?;
    let target = images.get(image_index).ok_or_else(|| "image index out of range".to_string())?;
    let target_name = target.resource_name.clone();
    let target_occurrence = target.occurrence;

    let image_id = crate::pdf::content::embed_jpeg_xobject(doc, jpeg, width, height);
    let mut resources = page_resources(doc, page_id)?;
    let mut xobjects = resources
        .get(b"XObject")
        .ok()
        .and_then(|object| match object {
            Object::Dictionary(dict) => Some(dict.clone()),
            Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
            _ => None,
        })
        .unwrap_or_default();
    let replacement_name = crate::pdf::content::next_image_xobject_name(&xobjects);
    xobjects.set(replacement_name.as_bytes(), Object::Reference(image_id));
    resources.set(b"XObject", Object::Dictionary(xobjects));
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(b"Resources", Object::Dictionary(resources));
    rewrite_image_do(doc, page_id, &target_name, target_occurrence, Some(&replacement_name))
}

fn page_content_ids(doc: &Document, page_id: lopdf::ObjectId) -> Result<Vec<lopdf::ObjectId>, String> {
    let dict = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;
    let obj = dict.get(b"Contents").map_err(|_| "page has no content stream".to_string())?;
    match obj {
        Object::Reference(id) => Ok(vec![*id]),
        Object::Array(items) => items
            .iter()
            .map(|item| item.as_reference().map_err(|_| "page content array contains a non-reference".to_string()))
            .collect(),
        _ => Err("page has no content stream".to_string()),
    }
}

fn rewrite_image_do(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    resource_name: &str,
    target_occurrence: usize,
    replacement_name: Option<&str>,
) -> Result<(), String> {
    let mut occurrence = 0usize;
    for content_id in page_content_ids(doc, page_id)? {
        let mut content = doc
            .get_object(content_id)
            .map_err(|e| e.to_string())?
            .as_stream()
            .map_err(|_| "content not stream".to_string())?
            .decode_content()
            .map_err(|e| e.to_string())?;
        let mut found = None;
        for (index, op) in content.operations.iter().enumerate() {
            if op.operator != "Do"
                || op.operands.first().and_then(|object| object.as_name().ok()) != Some(resource_name.as_bytes())
            {
                continue;
            }
            if occurrence == target_occurrence {
                found = Some(index);
                break;
            }
            occurrence += 1;
        }
        if let Some(index) = found {
            if let Some(name) = replacement_name {
                content.operations[index].operands[0] = Object::Name(name.as_bytes().to_vec());
            } else {
                content.operations.remove(index);
            }
            let encoded = content.encode().map_err(|e| e.to_string())?;
            let stream = doc
                .get_object_mut(content_id)
                .map_err(|e| e.to_string())?
                .as_stream_mut()
                .map_err(|_| "content not stream".to_string())?;
            stream.set_plain_content(encoded);
            return Ok(());
        }
    }
    Err("image Do operator not found".to_string())
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

    fn build_doc_with_image() -> (lopdf::Document, lopdf::ObjectId, lopdf::ObjectId) {
        let mut doc = lopdf::Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();
        let image_id = doc.new_object_id();

        let content_bytes = b"q 100 0 0 100 50 50 cm /Im1 Do Q\n".to_vec();
        doc.set_object(content_id, lopdf::Object::Stream(lopdf::Stream::new(lopdf::Dictionary::new(), content_bytes)));

        doc.set_object(
            image_id,
            lopdf::Object::Stream(lopdf::Stream::new(
                lopdf::Dictionary::from_iter(vec![
                    (b"Type".to_vec(), lopdf::Object::Name(b"XObject".to_vec())),
                    (b"Subtype".to_vec(), lopdf::Object::Name(b"Image".to_vec())),
                    (b"Width".to_vec(), lopdf::Object::Integer(100)),
                    (b"Height".to_vec(), lopdf::Object::Integer(100)),
                ]),
                vec![],
            )),
        );

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
                        b"XObject".to_vec(),
                        lopdf::Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                            b"Im1".to_vec(),
                            lopdf::Object::Reference(image_id),
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
        (doc, page_id, content_id)
    }

    fn assert_real_eq(actual: f64, expected: f64, epsilon: f64) {
        assert!((actual - expected).abs() < epsilon, "expected {expected} +/- {epsilon}, got {actual}");
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

    #[test]
    fn delete_text_line_whiteouts_existing_line() {
        let ops = "BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello world) Tj ET";
        let (mut doc, page_id) = build_doc_with_text(ops);
        delete_text_line(&mut doc, 0, 0).unwrap();
        let content =
            String::from_utf8_lossy(&crate::pdf::page_text::read_page_content(&doc, page_id).unwrap()).into_owned();
        assert!(content.contains("q 1 1 1 rg"));
        assert!(content.contains("re f Q"));
    }

    #[test]
    fn delete_text_line_out_of_range_fails() {
        let (mut doc, _) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET");
        let result = delete_text_line(&mut doc, 0, 5);
        assert!(result.is_err());
    }

    #[test]
    fn pdfium_text_region_can_be_replaced_or_deleted() {
        let source = PdfRect { x: 100.0, y: 100.0, width: 120.0, height: 30.0 };
        let style = style_with_align("left");
        let (mut edited, edited_page) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hidden) Tj ET");
        edit_text_region(&mut edited, 0, &source, "Replacement", &style, &source).unwrap();
        let content = String::from_utf8_lossy(&crate::pdf::page_text::read_page_content(&edited, edited_page).unwrap())
            .into_owned();
        assert!(content.contains("(Replacement)"));
        assert!(content.contains("re f Q"));

        let (mut deleted, deleted_page) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hidden) Tj ET");
        delete_text_region(&mut deleted, 0, &source).unwrap();
        let content =
            String::from_utf8_lossy(&crate::pdf::page_text::read_page_content(&deleted, deleted_page).unwrap())
                .into_owned();
        assert!(content.contains("re f Q"));
    }

    #[test]
    fn transform_page_image_rotates_90_degrees() {
        let (mut doc, _page_id, content_id) = build_doc_with_image();
        let image = crate::pdf::page_images::list_page_images(&doc, 0).unwrap().remove(0);
        assert_eq!(image.bbox.x, 50.0);
        assert_eq!(image.bbox.y, 50.0);
        assert_eq!(image.bbox.width, 100.0);
        assert_eq!(image.bbox.height, 100.0);
        let new_rect = PdfRect { x: 50.0, y: 50.0, width: 100.0, height: 100.0 };
        transform_page_image(&mut doc, 0, 0, &new_rect, 90.0).unwrap();

        let content_obj = doc.get_object(content_id).unwrap();
        let stream = content_obj.as_stream().unwrap();
        let decoded = stream.decode_content().unwrap();
        let cm_op = decoded
            .operations
            .iter()
            .enumerate()
            .find(|(i, op)| {
                op.operator == "cm" && i + 1 < decoded.operations.len() && decoded.operations[i + 1].operator == "Do"
            })
            .map(|(_, op)| op)
            .expect("cm Do pair not found");

        let vals: Vec<f64> = cm_op
            .operands
            .iter()
            .map(|o| match o {
                lopdf::Object::Real(v) => *v as f64,
                lopdf::Object::Integer(v) => *v as f64,
                _ => f64::NAN,
            })
            .collect();
        assert_eq!(vals.len(), 6);
        // For a 100x100 box centered at (100,100) rotated 90 degrees:
        // a=0, b=100, c=-100, d=0, e=150, f=50.
        assert_real_eq(vals[0], 0.0, 1e-4);
        assert_real_eq(vals[1], 100.0, 1e-4);
        assert_real_eq(vals[2], -100.0, 1e-4);
        assert_real_eq(vals[3], 0.0, 1e-4);
        assert_real_eq(vals[4], 150.0, 1e-4);
        assert_real_eq(vals[5], 50.0, 1e-4);

        let moved = crate::pdf::page_images::list_page_images(&doc, 0).unwrap();
        assert_real_eq(moved[0].rect.x, 50.0, 1e-4);
        assert_real_eq(moved[0].rect.y, 50.0, 1e-4);
        assert_real_eq(moved[0].rect.width, 100.0, 1e-4);
        assert_real_eq(moved[0].rect.height, 100.0, 1e-4);
        assert_real_eq(moved[0].rotation, 90.0, 1e-4);
    }

    #[test]
    fn image_instances_work_across_content_arrays_and_inherited_resources() {
        let (mut doc, page_id, first_content_id) = build_doc_with_image();
        doc.get_object_mut(first_content_id)
            .unwrap()
            .as_stream_mut()
            .unwrap()
            .set_plain_content(b"q 40 0 0 40 10 10 cm /Im1 Do Q\n".to_vec());
        let second_content_id = doc.add_object(Object::Stream(lopdf::Stream::new(
            lopdf::Dictionary::new(),
            b"q 60 0 0 60 100 100 cm /Im1 Do Q\n".to_vec(),
        )));
        doc.get_dictionary_mut(page_id).unwrap().set(
            b"Contents",
            Object::Array(vec![Object::Reference(first_content_id), Object::Reference(second_content_id)]),
        );

        let resources = doc.get_dictionary(page_id).unwrap().get(b"Resources").unwrap().clone();
        let pages_id = doc.get_dictionary(page_id).unwrap().get(b"Parent").unwrap().as_reference().unwrap();
        doc.get_dictionary_mut(page_id).unwrap().remove(b"Resources");
        doc.get_dictionary_mut(pages_id).unwrap().set(b"Resources", resources);

        let before = crate::pdf::page_images::list_page_images(&doc, 0).unwrap();
        assert_eq!(before.len(), 2);
        assert_eq!(before[0].occurrence, 0);
        assert_eq!(before[1].occurrence, 1);

        transform_page_image(&mut doc, 0, 1, &PdfRect { x: 200.0, y: 210.0, width: 80.0, height: 90.0 }, 0.0).unwrap();
        let moved = crate::pdf::page_images::list_page_images(&doc, 0).unwrap();
        assert_eq!(moved.len(), 2);
        assert_real_eq(moved[0].bbox.x, 10.0, 1e-4);
        assert_real_eq(moved[1].bbox.x, 200.0, 1e-4);
        assert_real_eq(moved[1].bbox.y, 210.0, 1e-4);

        remove_page_image(&mut doc, 0, 0).unwrap();
        let remaining = crate::pdf::page_images::list_page_images(&doc, 0).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_real_eq(remaining[0].bbox.x, 200.0, 1e-4);
    }

    #[test]
    fn replace_page_image_changes_only_target_resource() {
        let (mut doc, _, _) = build_doc_with_image();
        let path = std::env::temp_dir().join(format!(
            "kanoprii-replace-image-{}-{}.png",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        image::DynamicImage::new_rgb8(2, 3).save(&path).unwrap();

        replace_page_image(&mut doc, 0, 0, &path).unwrap();
        let images = crate::pdf::page_images::list_page_images(&doc, 0).unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(images.len(), 1);
        assert_ne!(images[0].resource_name, "Im1");
        assert_eq!(images[0].width, 2);
        assert_eq!(images[0].height, 3);
        assert_real_eq(images[0].bbox.x, 50.0, 1e-4);
        assert_real_eq(images[0].bbox.y, 50.0, 1e-4);
    }
}
