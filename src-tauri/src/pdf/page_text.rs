use super::coords::{page_media_box, viewer_rect_to_pdf, VIEWER_PAGE_H, VIEWER_PAGE_W};
use crate::pdf::content::append_page_content;
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageTextEdit {
    pub index: u32,
    pub x: f64,
    pub y: f64,
    pub font_size: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageVectorEdit {
    pub index: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub kind: String,
}

pub fn stream_plain_content(doc: &Document, id: ObjectId) -> Result<Vec<u8>, String> {
    let stream =
        doc.get_object(id).map_err(|e| e.to_string())?.as_stream().map_err(|_| "Bad content stream".to_string())?;
    match stream.get_plain_content() {
        Ok(bytes) => Ok(bytes),
        Err(_) => Ok(stream.content.clone()),
    }
}

pub fn read_page_content(doc: &Document, page_id: ObjectId) -> Result<Vec<u8>, String> {
    let contents = doc.get_dictionary(page_id).map_err(|e| e.to_string())?.get(b"Contents").ok().cloned();
    match contents {
        Some(Object::Reference(id)) => stream_plain_content(doc, id),
        Some(Object::Array(items)) => {
            let mut merged = Vec::new();
            for item in items {
                match item {
                    Object::Reference(id) => {
                        merged.extend_from_slice(&stream_plain_content(doc, id)?);
                        merged.push(b'\n');
                    }
                    Object::Stream(stream) => {
                        let bytes = stream.get_plain_content().unwrap_or_else(|_| stream.content.clone());
                        merged.extend_from_slice(&bytes);
                        merged.push(b'\n');
                    }
                    _ => {}
                }
            }
            Ok(merged)
        }
        Some(Object::Stream(stream)) => {
            let bytes = stream.get_plain_content().unwrap_or_else(|_| stream.content.clone());
            Ok(bytes)
        }
        _ => Ok(Vec::new()),
    }
}

pub fn write_page_content(doc: &mut Document, page_id: ObjectId, body: Vec<u8>) -> Result<(), String> {
    let mut stream = Stream::new(Dictionary::new(), body.clone());
    stream.set_plain_content(body);
    let stream_id = doc.add_object(Object::Stream(stream));
    doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.set(b"Contents", Object::Reference(stream_id));
    Ok(())
}

pub fn viewer_point_to_pdf(doc: &Document, page_id: ObjectId, x: f64, y: f64) -> Result<(f64, f64), String> {
    let media = page_media_box(doc, page_id)?;
    let mw = media[2] - media[0];
    let mh = media[3] - media[1];
    if mw <= 0.0 || mh <= 0.0 {
        return Err("Invalid page size".to_string());
    }
    let px = x * mw / VIEWER_PAGE_W;
    let py = mh - y * mh / VIEWER_PAGE_H;
    Ok((px, py))
}

pub fn escape_pdf_literal_string(text: &str) -> String {
    // ponytail: content-stream literals go via raw `format!` (NOT lopdf
    // objects, which are escaped on save). Per PDF 32000-1 §7.3.4.2, a literal
    // string must escape backslashes, parens, AND any byte in \x00..=\x1f via
    // backslash sequences (\n \r \t \b \f) or octal (\nnn). Without this,
    // a newline inside `(...)` desyncs the content-stream parser and injects
    // subsequent tokens as operators.
    //
    // Iterate by `char` so non-ASCII (UTF-8 multi-byte, e.g. 'é', '🚀', '日')
    // passes through verbatim — they are valid in PDF literals and get
    // WinAnsi-interpreted (or font-fallback) at render time. Earlier byte-
    // iteration form panicked on any byte >= 0x80.
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        let code = ch as u32;
        match ch {
            '\\' => out.push_str("\\\\"),
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            _ if code < 0x20 => out.push_str(&format!("\\{:03o}", code as u8)),
            _ => out.push(ch),
        }
    }
    out
}

fn marker_label(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn ensure_helvetica_font(doc: &mut Document, page_id: ObjectId) -> Result<String, String> {
    let page_dict = doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?;
    if !matches!(page_dict.get(b"Resources"), Ok(Object::Dictionary(_))) {
        page_dict.set(b"Resources", Object::Dictionary(Dictionary::new()));
    }
    let resources = page_dict
        .get_mut(b"Resources")
        .map_err(|e| e.to_string())?
        .as_dict_mut()
        .map_err(|_| "Bad Resources".to_string())?;
    let font_name = match resources.get_mut(b"Font") {
        Ok(Object::Dictionary(fonts)) => {
            if fonts.get(b"Helv").is_ok() {
                "Helv".to_string()
            } else {
                fonts.set(
                    b"Helv",
                    Object::Dictionary(Dictionary::from_iter(vec![
                        (b"Type".to_vec(), Object::Name(b"Font".to_vec())),
                        (b"Subtype".to_vec(), Object::Name(b"Type1".to_vec())),
                        (b"BaseFont".to_vec(), Object::Name(b"Helvetica".to_vec())),
                    ])),
                );
                "Helv".to_string()
            }
        }
        _ => {
            let mut fonts = Dictionary::new();
            fonts.set(
                b"Helv",
                Object::Dictionary(Dictionary::from_iter(vec![
                    (b"Type".to_vec(), Object::Name(b"Font".to_vec())),
                    (b"Subtype".to_vec(), Object::Name(b"Type1".to_vec())),
                    (b"BaseFont".to_vec(), Object::Name(b"Helvetica".to_vec())),
                ])),
            );
            resources.set(b"Font", Object::Dictionary(fonts));
            "Helv".to_string()
        }
    };
    Ok(font_name)
}

pub fn next_panda_text_index(content: &str) -> u32 {
    content
        .lines()
        .filter_map(|line| line.strip_prefix("%PP-TXT "))
        .filter_map(|rest| rest.split_whitespace().next()?.parse::<u32>().ok())
        .max()
        .map(|max| max + 1)
        .unwrap_or(0)
}

pub fn next_panda_vector_index(content: &str) -> u32 {
    content
        .lines()
        .filter_map(|line| line.strip_prefix("%PP-VEC "))
        .filter_map(|rest| rest.split_whitespace().next()?.parse::<u32>().ok())
        .max()
        .map(|max| max + 1)
        .unwrap_or(0)
}

pub fn parse_page_text_edits(content: &str) -> Vec<PageTextEdit> {
    let mut edits = Vec::new();
    for line in content.lines() {
        let Some(rest) = line.strip_prefix("%PP-TXT ") else { continue };
        let mut parts = rest.split_whitespace();
        let Some(index) = parts.next().and_then(|v| v.parse::<u32>().ok()) else { continue };
        let Some(x) = parts.next().and_then(|v| v.parse::<f64>().ok()) else { continue };
        let Some(y) = parts.next().and_then(|v| v.parse::<f64>().ok()) else { continue };
        let Some(font_size) = parts.next().and_then(|v| v.parse::<f64>().ok()) else { continue };
        let text = parts.collect::<Vec<_>>().join(" ");
        edits.push(PageTextEdit { index, x, y, font_size, text });
    }
    edits.sort_by_key(|edit| edit.index);
    edits
}

pub fn parse_page_vectors(content: &str) -> Vec<PageVectorEdit> {
    let mut vectors = Vec::new();
    for line in content.lines() {
        let Some(rest) = line.strip_prefix("%PP-VEC ") else { continue };
        let mut parts = rest.split_whitespace();
        let Some(index) = parts.next().and_then(|v| v.parse::<u32>().ok()) else { continue };
        let Some(x) = parts.next().and_then(|v| v.parse::<f64>().ok()) else { continue };
        let Some(y) = parts.next().and_then(|v| v.parse::<f64>().ok()) else { continue };
        let Some(width) = parts.next().and_then(|v| v.parse::<f64>().ok()) else { continue };
        let Some(height) = parts.next().and_then(|v| v.parse::<f64>().ok()) else { continue };
        let kind = parts.next().unwrap_or("stroke").to_string();
        vectors.push(PageVectorEdit { index, x, y, width, height, kind });
    }
    vectors.sort_by_key(|vector| vector.index);
    vectors
}

pub fn remove_panda_block(content: &str, marker_prefix: &str, index: u32) -> Result<String, String> {
    let needle = format!("{marker_prefix} {index} ");
    let mut lines = content.lines().collect::<Vec<_>>();
    let start = lines
        .iter()
        .position(|line| line.starts_with(&needle))
        .ok_or_else(|| format!("{marker_prefix} block not found"))?;
    let mut end = start + 1;
    while end < lines.len() && !lines[end].starts_with("%PP-") {
        end += 1;
    }
    lines.drain(start..end);
    let mut output = lines.join("\n");
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    Ok(output)
}

pub struct PageTextOpsArgs<'a> {
    pub index: u32,
    pub x: f64,
    pub y: f64,
    pub font_size: f64,
    pub text: &'a str,
    pub px: f64,
    pub py: f64,
    pub font_name: &'a str,
}

pub fn build_page_text_ops(args: PageTextOpsArgs<'_>) -> String {
    let escaped = escape_pdf_literal_string(args.text);
    format!(
        "%PP-TXT {index} {x} {y} {font_size} {label}\nBT /{font_name} {font_size} Tf 1 0 0 1 {px} {py} Tm ({escaped}) Tj ET\n",
        index = args.index,
        x = args.x,
        y = args.y,
        font_size = args.font_size,
        font_name = args.font_name,
        px = args.px,
        py = args.py,
        label = marker_label(args.text)
    )
}

pub fn add_page_text(
    path: &Path,
    page_index: u32,
    x: f64,
    y: f64,
    font_size: f64,
    text: String,
) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Text cannot be empty".to_string());
    }
    if !(8.0..=72.0).contains(&font_size) {
        return Err("Font size must be between 8 and 72".to_string());
    }
    let path = path.to_path_buf();
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    let font_name = ensure_helvetica_font(&mut doc, page_id)?;
    let (px, py) = viewer_point_to_pdf(&doc, page_id, x, y)?;
    let content = String::from_utf8_lossy(&read_page_content(&doc, page_id)?).into_owned();
    let index = next_panda_text_index(&content);
    let mut ops =
        build_page_text_ops(PageTextOpsArgs { index, x, y, font_size, text: trimmed, px, py, font_name: &font_name });
    if !ops.starts_with('\n') {
        ops.insert(0, '\n');
    }
    append_page_content(&mut doc, page_id, ops.as_bytes())?;
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(index)
}

pub fn list_page_text_edits(path: &Path, page_index: u32) -> Result<Vec<PageTextEdit>, String> {
    let path = path.to_path_buf();
    let doc = Document::load(&path).map_err(|e| e.to_string())?;
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    let bytes = read_page_content(&doc, page_id)?;
    let content = String::from_utf8_lossy(&bytes).into_owned();
    Ok(parse_page_text_edits(&content))
}

pub fn update_page_text(
    path: &Path,
    page_index: u32,
    index: u32,
    text: String,
    x: Option<f64>,
    y: Option<f64>,
    font_size: Option<f64>,
) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Text cannot be empty".to_string());
    }
    let path = path.to_path_buf();
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    let font_name = ensure_helvetica_font(&mut doc, page_id)?;
    let content = String::from_utf8_lossy(&read_page_content(&doc, page_id)?).into_owned();
    let existing = parse_page_text_edits(&content)
        .into_iter()
        .find(|edit| edit.index == index)
        .ok_or_else(|| "Text block not found".to_string())?;
    let next_x = x.unwrap_or(existing.x);
    let next_y = y.unwrap_or(existing.y);
    let next_font_size = font_size.unwrap_or(existing.font_size);
    if !(8.0..=72.0).contains(&next_font_size) {
        return Err("Font size must be between 8 and 72".to_string());
    }
    let mut content = remove_panda_block(&content, "%PP-TXT", index)?;
    let (px, py) = viewer_point_to_pdf(&doc, page_id, next_x, next_y)?;
    content.push_str(&build_page_text_ops(PageTextOpsArgs {
        index,
        x: next_x,
        y: next_y,
        font_size: next_font_size,
        text: trimmed,
        px,
        py,
        font_name: &font_name,
    }));
    write_page_content(&mut doc, page_id, content.into_bytes())?;
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(())
}

pub fn remove_page_text(path: &Path, page_index: u32, index: u32) -> Result<(), String> {
    let path = path.to_path_buf();
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    let content = String::from_utf8_lossy(&read_page_content(&doc, page_id)?).into_owned();
    let content = remove_panda_block(&content, "%PP-TXT", index)?;
    write_page_content(&mut doc, page_id, content.into_bytes())?;
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(())
}

pub fn add_page_vector_rect(
    path: &Path,
    page_index: u32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<u32, String> {
    if width < 2.0 || height < 2.0 {
        return Err("Vector shape is too small".to_string());
    }
    let path = path.to_path_buf();
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    let (px, py, pw, ph) = viewer_rect_to_pdf(&doc, page_id, x, y, width, height)?;
    let content = String::from_utf8_lossy(&read_page_content(&doc, page_id)?).into_owned();
    let index = next_panda_vector_index(&content);
    let ops = format!("\n%PP-VEC {index} {x} {y} {width} {height} stroke\nq 1 w {px} {py} {pw} {ph} re S Q\n");
    append_page_content(&mut doc, page_id, ops.as_bytes())?;
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(index)
}

pub fn list_page_vectors(path: &Path, page_index: u32) -> Result<Vec<PageVectorEdit>, String> {
    let path = path.to_path_buf();
    let doc = Document::load(&path).map_err(|e| e.to_string())?;
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    let bytes = read_page_content(&doc, page_id)?;
    let content = String::from_utf8_lossy(&bytes).into_owned();
    Ok(parse_page_vectors(&content))
}

pub fn remove_page_vector(path: &Path, page_index: u32, index: u32) -> Result<(), String> {
    let path = path.to_path_buf();
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
    let content = String::from_utf8_lossy(&read_page_content(&doc, page_id)?).into_owned();
    let content = remove_panda_block(&content, "%PP-VEC", index)?;
    write_page_content(&mut doc, page_id, content.into_bytes())?;
    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(())
}

#[cfg(test)]
mod escape_tests {
    use super::escape_pdf_literal_string;

    #[test]
    fn escapes_backslash_and_parens() {
        assert_eq!(escape_pdf_literal_string("a(b)c\\d"), "a\\(b\\)c\\\\d");
    }

    #[test]
    fn escapes_control_chars_per_pdf_spec() {
        // Raw newline would break content-stream syntax; must become \n.
        assert_eq!(escape_pdf_literal_string("a\nb"), "a\\nb");
        assert_eq!(escape_pdf_literal_string("x\ry"), "x\\ry");
        assert_eq!(escape_pdf_literal_string("x\ty"), "x\\ty");
        assert_eq!(escape_pdf_literal_string("a\0b"), "a\\000b");
        // Other \x01..\x1f go via octal.
        assert_eq!(escape_pdf_literal_string("a\x01b"), "a\\001b");
        assert_eq!(escape_pdf_literal_string("a\x1fb"), "a\\037b");
    }

    #[test]
    fn operator_injection_is_neutralised() {
        // An attacker-controlled string with newline + PDF operators must NOT
        // be emitted verbatim into the content stream.
        let s = "abc\nTj Td 0 0 (xyz";
        let esc = escape_pdf_literal_string(s);
        assert!(esc.contains("\\n"), "newline must be escaped: got {esc}");
        assert!(!esc.contains('\n'), "no raw LF in escaped form: got {esc}");
    }

    #[test]
    fn passes_through_non_ascii_unicode() {
        // Non-ASCII must not panic. 'é' (U+00E9) is one UTF-8 byte-pair; the
        // prior byte-iteration form would panic on byte 0xE9 with `as char`.
        let esc = escape_pdf_literal_string("café");
        assert_eq!(esc, "café");
        // Emoji (4-byte UTF-8) and CJK also pass through.
        assert_eq!(escape_pdf_literal_string("🚀"), "🚀");
        assert_eq!(escape_pdf_literal_string("日本語"), "日本語");
    }
}
