use crate::pdf::content::append_page_content;
use crate::pdf::coords::viewer_rect_to_pdf;
use crate::pdf::fonts::{ensure_full_font, font_has_glyphs_for};
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
}
