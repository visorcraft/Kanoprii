use crate::pdf::content::append_page_content;
use crate::pdf::coords::viewer_rect_to_pdf;
use crate::pdf::edit_object::validate_style_inputs;
use crate::pdf::edit_types::{PdfRect, TextStyle};
use crate::pdf::fonts::{
    ensure_font_family, ensure_full_font, font_has_glyphs_for, measure_text_width, style_supports_text,
    uses_synthetic_font_style,
};
use crate::pdf::io::mutate_pdf;
use crate::pdf::page_text::ensure_helvetica_font;
use crate::pdf::page_text::escape_pdf_literal_string;
#[cfg(test)]
use crate::pdf::page_text::read_page_content;
use crate::pdf::text_lines::decode_page_text_lines;
use std::path::Path;

/// White-out a viewer-pixel region and draw replacement text on top (append-only).
#[allow(clippy::too_many_arguments)]
pub fn replace_text_region(
    path: &Path,
    page_index: u32,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    new_text: &str,
    font_size: f64,
) -> Result<(), String> {
    let trimmed = new_text.trim();
    if trimmed.is_empty() {
        return Err("Text cannot be empty".to_string());
    }
    if !(6.0..=72.0).contains(&font_size) {
        return Err("Font size must be between 6 and 72".to_string());
    }
    mutate_pdf(path, |doc| {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
        let (px, py, pw, ph) = viewer_rect_to_pdf(doc, page_id, x, y, w, h)?;
        let whiteout = format!("q 1 1 1 rg {px} {py} {pw} {ph} re f Q\n");
        append_page_content(doc, page_id, whiteout.as_bytes())?;

        let font_name = ensure_helvetica_font(doc, page_id)?;
        let escaped = escape_pdf_literal_string(trimmed);
        let descent = font_size * 0.2;
        let text_ops = format!(
            "BT /{font_name} {font_size} Tf 1 0 0 1 {tx} {ty} Tm ({escaped}) Tj ET\n",
            font_name = font_name,
            font_size = font_size,
            tx = px,
            ty = py + descent,
            escaped = escaped,
        );
        append_page_content(doc, page_id, text_ops.as_bytes())?;
        Ok(())
    })
}

/// Replace a decoded text line in-place (v2 editing).
///
/// 1. White-out the line's approximate bounding box.
/// 2. Emit new text at the original transform using the embedded full font.
/// 3. Apply horizontal scaling so the replacement fits the original line width.
pub fn replace_text_line(path: &Path, page_index: u32, line_index: usize, new_text: &str) -> Result<(), String> {
    let trimmed = new_text.trim();
    if trimmed.is_empty() {
        return Err("Text cannot be empty".to_string());
    }
    mutate_pdf(path, |doc| {
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
        let lines = decode_page_text_lines(doc, page_id)?;
        let line = lines.get(line_index).ok_or_else(|| "Line not found".to_string())?;

        // Glyph coverage check → caller should fall back to v1 if this fails.
        if !font_has_glyphs_for(trimmed) {
            return Err("Replacement text contains characters not supported by the embedded font".to_string());
        }

        // White-out the line box.
        let [x1, y1, x2, y2] = line.bbox;
        let w = (x2 - x1).max(1.0);
        let h = (y2 - y1).max(1.0);
        let whiteout = format!("q 1 1 1 rg {x1} {y1} {w} {h} re f Q\n");
        append_page_content(doc, page_id, whiteout.as_bytes())?;

        // Compute horizontal scaling so replacement fits original width.
        let original_width = w;
        let est_new_width = (trimmed.chars().count() as f64 * line.font_size * 0.5).max(1.0);
        let scale = if est_new_width > original_width { original_width / est_new_width } else { 1.0 };

        // Build new text matrix with scaling applied to the horizontal axis.
        let [a, b, c, d, e, f] = line.transform;
        let new_a = a * scale;
        let new_b = b * scale;

        let font_name = ensure_full_font(doc, page_id)?;
        let escaped = escape_pdf_literal_string(trimmed);
        let text_ops = format!(
            "BT /{font_name} {font_size} Tf {new_a} {new_b} {c} {d} {e} {f} Tm ({escaped}) Tj ET\n",
            font_name = font_name,
            font_size = line.font_size,
        );
        append_page_content(doc, page_id, text_ops.as_bytes())?;
        Ok(())
    })
}

/// Replace a decoded text line with styled text (Phase 1 full editing).
///
/// 1. Validate glyph coverage for the requested style.
/// 2. Ensure the page has the requested font family.
/// 3. White-out the original line bbox.
/// 4. Draw the replacement text inside `box_rect` with the style's font,
///    size, color, and optional synthetic bold/italic/underline.
pub fn replace_text_line_styled(
    doc: &mut lopdf::Document,
    page_index: u32,
    line_index: usize,
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
    let line = lines.get(line_index).ok_or_else(|| "Line not found".to_string())?;

    // White-out the original line box.
    let [x1, y1, x2, y2] = line.bbox;
    let w = (x2 - x1).max(1.0);
    let h = (y2 - y1).max(1.0);
    let whiteout = format!("q 1 1 1 rg {x1} {y1} {w} {h} re f Q\n");
    append_page_content(doc, page_id, whiteout.as_bytes())?;

    // Convert the viewer-pixel box to PDF user space.
    let (px, py, pw, ph) = viewer_rect_to_pdf(doc, page_id, box_rect.x, box_rect.y, box_rect.width, box_rect.height)?;

    // Measure rendered width and compute horizontal alignment offset.
    let est_width = measure_text_width(trimmed, &style.font_family, style.font_size);
    let align = style.align.to_lowercase();
    let tx = match align.as_str() {
        "center" => (px + (pw - est_width) / 2.0).max(px),
        "right" => (px + pw - est_width).max(px),
        _ => px,
    };
    let baseline = py + style.font_size * 0.2;
    if baseline > py + ph {
        return Err("Box rect is too short for the requested font size".to_string());
    }

    let escaped = escape_pdf_literal_string(trimmed);
    let mut ops =
        format!("q {r} {g} {b} rg {r} {g} {b} RG\n", r = style.color.r, g = style.color.g, b = style.color.b,);

    let synthetic_style = uses_synthetic_font_style(style);
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
        let line_width = style.font_size * 0.05;
        let x_end = tx + est_width;
        ops.push_str(&format!("{tx} {uy} m {x_end} {uy} l {line_width} w S\n"));
    }

    ops.push_str("Q\n");
    append_page_content(doc, page_id, ops.as_bytes())?;
    Ok(())
}

/// Read page content as UTF-8 lossy string (test helper).
#[cfg(test)]
pub fn page_content_string(path: &Path, page_index: u32) -> Result<String, String> {
    let doc = lopdf::Document::load(path).map_err(|e| e.to_string())?;
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    Ok(String::from_utf8_lossy(&read_page_content(&doc, page_id)?).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::edit_types::RgbColor;
    use lopdf::{Dictionary, Object, Stream};
    use std::path::PathBuf;

    fn build_pdf_with_text(content_ops: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let mut doc = lopdf::Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();

        let content_bytes = content_ops.as_bytes().to_vec();
        doc.set_object(content_id, Object::Stream(Stream::new(Dictionary::new(), content_bytes)));

        doc.set_object(
            page_id,
            Object::Dictionary(Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"Page".to_vec())),
                (b"Parent".to_vec(), Object::Reference(pages_id)),
                (
                    b"MediaBox".to_vec(),
                    Object::Array(vec![
                        Object::Integer(0),
                        Object::Integer(0),
                        Object::Integer(612),
                        Object::Integer(792),
                    ]),
                ),
                (b"Contents".to_vec(), Object::Reference(content_id)),
                (
                    b"Resources".to_vec(),
                    Object::Dictionary(Dictionary::from_iter(vec![(
                        b"Font".to_vec(),
                        Object::Dictionary(Dictionary::from_iter(vec![(
                            b"F1".to_vec(),
                            Object::Dictionary(Dictionary::from_iter(vec![
                                (b"Type".to_vec(), Object::Name(b"Font".to_vec())),
                                (b"Subtype".to_vec(), Object::Name(b"Type1".to_vec())),
                                (b"BaseFont".to_vec(), Object::Name(b"Helvetica".to_vec())),
                            ])),
                        )])),
                    )])),
                ),
            ])),
        );

        doc.set_object(
            pages_id,
            Object::Dictionary(Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"Pages".to_vec())),
                (b"Kids".to_vec(), Object::Array(vec![Object::Reference(page_id)])),
                (b"Count".to_vec(), Object::Integer(1)),
            ])),
        );

        let catalog_id = doc.new_object_id();
        doc.set_object(
            catalog_id,
            Object::Dictionary(Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"Catalog".to_vec())),
                (b"Pages".to_vec(), Object::Reference(pages_id)),
            ])),
        );
        doc.trailer.set(b"Root", Object::Reference(catalog_id));

        let path = std::env::temp_dir().join(format!(
            "kanoprii_test_{}_{}.pdf",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        doc.save(&path).unwrap();
        path
    }

    fn build_doc_with_text(content_ops: &str) -> (lopdf::Document, lopdf::ObjectId) {
        let mut doc = lopdf::Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();

        let content_bytes = content_ops.as_bytes().to_vec();
        doc.set_object(content_id, Object::Stream(Stream::new(Dictionary::new(), content_bytes)));

        doc.set_object(
            page_id,
            Object::Dictionary(Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"Page".to_vec())),
                (b"Parent".to_vec(), Object::Reference(pages_id)),
                (
                    b"MediaBox".to_vec(),
                    Object::Array(vec![
                        Object::Integer(0),
                        Object::Integer(0),
                        Object::Integer(612),
                        Object::Integer(792),
                    ]),
                ),
                (b"Contents".to_vec(), Object::Reference(content_id)),
                (
                    b"Resources".to_vec(),
                    Object::Dictionary(Dictionary::from_iter(vec![(
                        b"Font".to_vec(),
                        Object::Dictionary(Dictionary::from_iter(vec![(
                            b"F1".to_vec(),
                            Object::Dictionary(Dictionary::from_iter(vec![
                                (b"Type".to_vec(), Object::Name(b"Font".to_vec())),
                                (b"Subtype".to_vec(), Object::Name(b"Type1".to_vec())),
                                (b"BaseFont".to_vec(), Object::Name(b"Helvetica".to_vec())),
                            ])),
                        )])),
                    )])),
                ),
            ])),
        );

        doc.set_object(
            pages_id,
            Object::Dictionary(Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"Pages".to_vec())),
                (b"Kids".to_vec(), Object::Array(vec![Object::Reference(page_id)])),
                (b"Count".to_vec(), Object::Integer(1)),
            ])),
        );

        let catalog_id = doc.new_object_id();
        doc.set_object(
            catalog_id,
            Object::Dictionary(Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"Catalog".to_vec())),
                (b"Pages".to_vec(), Object::Reference(pages_id)),
            ])),
        );
        doc.trailer.set(b"Root", Object::Reference(catalog_id));
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

    fn last_tm_x(content: &str) -> Option<f64> {
        let before = content.rsplit("Tm").nth(1)?;
        let tokens: Vec<&str> = before.split_whitespace().collect();
        if tokens.len() < 6 {
            return None;
        }
        tokens[tokens.len() - 2].parse().ok()
    }

    #[test]
    fn replace_text_line_replaces_and_preserves_transform() {
        let ops = "BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET";
        let path = build_pdf_with_text(ops);
        replace_text_line(&path, 0, 0, "World").unwrap();
        let content = page_content_string(&path, 0).unwrap();
        // White-out + replacement BT...ET should be appended.
        assert!(content.contains("q 1 1 1 rg"));
        assert!(content.contains("World"));
        assert!(content.contains("PPFullFont"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn replace_text_line_missing_glyph_fails() {
        let ops = "BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET";
        let path = build_pdf_with_text(ops);
        // Japanese characters are not in Liberation Sans.
        let result = replace_text_line(&path, 0, 0, "こんにちは");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not supported"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn replace_text_line_empty_text_fails() {
        let ops = "BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET";
        let path = build_pdf_with_text(ops);
        let result = replace_text_line(&path, 0, 0, "   ");
        assert!(result.is_err());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn replace_text_line_styled_alignment_offsets_differ() {
        let ops = "BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET";

        let left = {
            let (mut doc, page_id) = build_doc_with_text(ops);
            replace_text_line_styled(&mut doc, 0, 0, "World", &style_with_align("left"), &full_page_box()).unwrap();
            last_tm_x(&String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap())).unwrap()
        };
        let center = {
            let (mut doc, page_id) = build_doc_with_text(ops);
            replace_text_line_styled(&mut doc, 0, 0, "World", &style_with_align("center"), &full_page_box()).unwrap();
            last_tm_x(&String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap())).unwrap()
        };
        let right = {
            let (mut doc, page_id) = build_doc_with_text(ops);
            replace_text_line_styled(&mut doc, 0, 0, "World", &style_with_align("right"), &full_page_box()).unwrap();
            last_tm_x(&String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap())).unwrap()
        };

        assert!(left < center, "center should shift text right: left={left} center={center}");
        assert!(center < right, "right should shift text further right: center={center} right={right}");
    }

    #[test]
    fn replace_text_line_styled_emits_color_operators() {
        let (mut doc, page_id) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET");
        replace_text_line_styled(&mut doc, 0, 0, "World", &style_with_align("left"), &full_page_box()).unwrap();
        let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
        assert!(content.contains("rg"), "fill color operator missing");
        assert!(content.contains("RG"), "stroke color operator missing");
    }

    #[test]
    fn replace_text_line_styled_bold_uses_standard_bold_face() {
        let (mut doc, page_id) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET");
        let mut style = style_with_align("left");
        style.bold = true;
        replace_text_line_styled(&mut doc, 0, 0, "World", &style, &full_page_box()).unwrap();
        let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
        assert_eq!(content.matches("Tj").count(), 2);
        assert!(content.contains("/HelvB 12 Tf"));
    }

    #[test]
    fn replace_text_line_styled_italic_uses_standard_italic_face() {
        let (mut doc, page_id) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET");
        let mut style = style_with_align("left");
        style.italic = true;
        replace_text_line_styled(&mut doc, 0, 0, "World", &style, &full_page_box()).unwrap();
        let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
        assert!(content.contains("/HelvI 12 Tf"));
        assert!(!content.contains("1 0 0.25 1"));
    }

    #[test]
    fn replace_text_line_styled_underline_emits_stroke() {
        let (mut doc, page_id) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET");
        let mut style = style_with_align("left");
        style.underline = true;
        replace_text_line_styled(&mut doc, 0, 0, "World", &style, &full_page_box()).unwrap();
        let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
        assert!(content.contains(" m "), "underline should contain a moveto");
        assert!(content.contains(" l "), "underline should contain a lineto");
        assert!(content.contains(" S"), "underline should contain a stroke operator");
    }

    #[test]
    fn replace_text_line_styled_empty_text_fails() {
        let (mut doc, _) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET");
        let result = replace_text_line_styled(&mut doc, 0, 0, "   ", &style_with_align("left"), &full_page_box());
        assert!(result.is_err());
    }

    #[test]
    fn replace_text_line_styled_missing_glyph_fails_for_liberation_sans() {
        let (mut doc, _) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET");
        let mut style = style_with_align("left");
        style.font_family = "LiberationSans".to_string();
        let result = replace_text_line_styled(&mut doc, 0, 0, "こんにちは", &style, &full_page_box());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not supported"));
    }

    #[test]
    fn replace_text_line_styled_rejects_invalid_color() {
        let (mut doc, _) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET");
        let mut style = style_with_align("left");
        style.color = RgbColor { r: 1.5, g: 0.0, b: 0.0 };
        let result = replace_text_line_styled(&mut doc, 0, 0, "World", &style, &full_page_box());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Color components"));
    }

    #[test]
    fn replace_text_line_styled_rejects_non_positive_box() {
        let (mut doc, _) = build_doc_with_text("BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET");
        let mut bad_box = full_page_box();
        bad_box.width = 0.0;
        let result = replace_text_line_styled(&mut doc, 0, 0, "World", &style_with_align("left"), &bad_box);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("positive width"));
    }
}
