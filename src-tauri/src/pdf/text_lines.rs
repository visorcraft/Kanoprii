use lopdf::{content::Operation, Document, Object, ObjectId};

/// A decoded text line from a page content stream.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLine {
    pub text: String,
    /// Text matrix [a, b, c, d, e, f] at line start (PDF user space).
    pub transform: [f64; 6],
    /// Resource font name (e.g. "F1").
    pub font_name: String,
    pub font_size: f64,
    /// Approximate bounding box in PDF user space [left, bottom, right, top].
    pub bbox: [f64; 4],
}

/// Decode the top-level content stream of a page into text lines.
///
/// Falls back to an empty vec for:
/// - pages that use Form XObjects for text
/// - Type3 fonts
/// - streams we cannot decompress
///
/// This is intentionally conservative; the caller should fall back to v1
/// white-out editing when this returns an empty or short result.
pub fn decode_page_text_lines(doc: &Document, page_id: ObjectId) -> Result<Vec<TextLine>, String> {
    let bytes = super::page_text::read_page_content(doc, page_id)?;
    let ops = parse_content_ops(&bytes)?;
    let lines = ops_to_lines(doc, page_id, &ops)?;
    Ok(lines)
}

fn parse_content_ops(bytes: &[u8]) -> Result<Vec<Operation>, String> {
    // lopdf 0.41 parser takes a raw byte slice.
    let content = lopdf::content::Content::decode(bytes).map_err(|_| "Failed to parse content stream".to_string())?;
    Ok(content.operations)
}

/// Convert parsed operations into line records.
fn ops_to_lines(doc: &Document, page_id: ObjectId, ops: &[Operation]) -> Result<Vec<TextLine>, String> {
    let mut lines: Vec<TextLine> = Vec::new();

    // Graphics-state stack for q/Q.
    let mut ctm = identity_matrix();
    let mut gs_stack: Vec<[f64; 6]> = Vec::new();

    // Text-state variables valid inside a BT … ET pair.
    let mut in_text = false;
    let mut text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut text_line_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut font_name = String::new();
    let mut font_size = 0.0;
    let mut h_scale = 1.0;
    let mut char_spacing = 0.0;
    let mut word_spacing = 0.0;
    let mut leading = 0.0;

    // Line accumulator.
    let mut current_text = String::new();
    let mut line_start_tm = [0.0f64; 6];
    let mut line_min_y = f64::MAX;
    let mut line_max_y = f64::MIN;
    let mut line_start_x = 0.0f64;
    let mut line_end_x = 0.0f64;
    let mut line_font_name = String::new();
    let mut line_font_size = 0.0f64;
    let mut has_line = false;

    let media = super::coords::page_media_box(doc, page_id)?;
    let page_h = (media[3] - media[1]).max(1.0);

    // Helper to flush the current line.
    let flush_line = |text: &mut String,
                      start_tm: &mut [f64; 6],
                      min_y: &mut f64,
                      max_y: &mut f64,
                      start_x: &mut f64,
                      end_x: &mut f64,
                      lfn: &mut String,
                      lfs: &mut f64,
                      has: &mut bool,
                      lines: &mut Vec<TextLine>| {
        if !*has || text.trim().is_empty() {
            text.clear();
            *has = false;
            return;
        }
        let left = *start_x;
        let right = (*end_x).max(left + 1.0);
        let baseline = (*start_tm)[5];
        let bottom = (*min_y).min(baseline - *lfs * 0.2);
        let top = (*max_y).max(baseline + *lfs * 0.8);
        lines.push(TextLine {
            text: std::mem::take(text),
            transform: *start_tm,
            font_name: std::mem::take(lfn),
            font_size: *lfs,
            bbox: [left, bottom, right, top],
        });
        *has = false;
    };

    for op in ops {
        match op.operator.as_str() {
            "q" => {
                gs_stack.push(ctm);
            }
            "Q" => {
                ctm = gs_stack.pop().unwrap_or_else(identity_matrix);
            }
            "cm" => {
                if let Some(vals) = op_to_six_f64(&op.operands) {
                    ctm = multiply_matrices(ctm, vals);
                }
            }
            "BT" => {
                in_text = true;
                text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                text_line_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                // Flush any dangling line (shouldn't happen, but be safe).
                flush_line(
                    &mut current_text,
                    &mut line_start_tm,
                    &mut line_min_y,
                    &mut line_max_y,
                    &mut line_start_x,
                    &mut line_end_x,
                    &mut line_font_name,
                    &mut line_font_size,
                    &mut has_line,
                    &mut lines,
                );
            }
            "ET" => {
                in_text = false;
                flush_line(
                    &mut current_text,
                    &mut line_start_tm,
                    &mut line_min_y,
                    &mut line_max_y,
                    &mut line_start_x,
                    &mut line_end_x,
                    &mut line_font_name,
                    &mut line_font_size,
                    &mut has_line,
                    &mut lines,
                );
            }
            "Tm" if in_text => {
                if let Some(vals) = op_to_six_f64(&op.operands) {
                    let effective_vals = multiply_matrices(ctm, vals);
                    // Tm sets both Tm and Tlm.
                    // If we are already accumulating a line and the baseline
                    // changes significantly, treat this as a new line.
                    let new_y = effective_vals[5];
                    if has_line {
                        let old_y = line_start_tm[5];
                        let threshold = line_font_size * 0.5;
                        if (new_y - old_y).abs() > threshold {
                            flush_line(
                                &mut current_text,
                                &mut line_start_tm,
                                &mut line_min_y,
                                &mut line_max_y,
                                &mut line_start_x,
                                &mut line_end_x,
                                &mut line_font_name,
                                &mut line_font_size,
                                &mut has_line,
                                &mut lines,
                            );
                        }
                    }
                    text_matrix = vals;
                    text_line_matrix = vals;
                    if !has_line {
                        line_start_tm = effective_vals;
                        line_start_x = effective_vals[4];
                        line_end_x = effective_vals[4];
                        line_min_y = effective_vals[5];
                        line_max_y = effective_vals[5];
                        line_font_name = font_name.clone();
                        line_font_size = font_size;
                        has_line = true;
                    }
                }
            }
            "Tf" if in_text => {
                if op.operands.len() >= 2 {
                    if let (Some(name), Some(size)) =
                        (operand_to_name(doc, &op.operands[0]), operand_to_f64(&op.operands[1]))
                    {
                        // If the font is Type3, bail out conservatively.
                        if is_type3_font(doc, page_id, &name) {
                            return Ok(Vec::new());
                        }
                        font_name = name;
                        font_size = size;
                        // If font changes mid-line, flush and start new line.
                        if has_line && (font_name != line_font_name || (font_size - line_font_size).abs() > 0.1) {
                            flush_line(
                                &mut current_text,
                                &mut line_start_tm,
                                &mut line_min_y,
                                &mut line_max_y,
                                &mut line_start_x,
                                &mut line_end_x,
                                &mut line_font_name,
                                &mut line_font_size,
                                &mut has_line,
                                &mut lines,
                            );
                        }
                    }
                }
            }
            "Td" | "TD" if in_text => {
                if let (Some(dx), Some(dy)) =
                    (op.operands.first().and_then(operand_to_f64), op.operands.get(1).and_then(operand_to_f64))
                {
                    // TD also sets leading = -dy for TD.
                    if op.operator == "TD" {
                        leading = -dy;
                    }
                    // Td translates the text line matrix.
                    text_line_matrix[4] += dx;
                    text_line_matrix[5] += dy;
                    text_matrix = text_line_matrix;
                    // Always start a new line for Td/TD.
                    flush_line(
                        &mut current_text,
                        &mut line_start_tm,
                        &mut line_min_y,
                        &mut line_max_y,
                        &mut line_start_x,
                        &mut line_end_x,
                        &mut line_font_name,
                        &mut line_font_size,
                        &mut has_line,
                        &mut lines,
                    );
                    let effective_tm = multiply_matrices(ctm, text_matrix);
                    line_start_tm = effective_tm;
                    line_start_x = effective_tm[4];
                    line_end_x = effective_tm[4];
                    line_min_y = effective_tm[5];
                    line_max_y = effective_tm[5];
                    line_font_name = font_name.clone();
                    line_font_size = font_size;
                    has_line = true;
                }
            }
            "T*" if in_text => {
                // Move to start of next line using leading.
                text_line_matrix[5] -= leading;
                text_matrix = text_line_matrix;
                flush_line(
                    &mut current_text,
                    &mut line_start_tm,
                    &mut line_min_y,
                    &mut line_max_y,
                    &mut line_start_x,
                    &mut line_end_x,
                    &mut line_font_name,
                    &mut line_font_size,
                    &mut has_line,
                    &mut lines,
                );
                let effective_tm = multiply_matrices(ctm, text_matrix);
                line_start_tm = effective_tm;
                line_start_x = effective_tm[4];
                line_end_x = effective_tm[4];
                line_min_y = effective_tm[5];
                line_max_y = effective_tm[5];
                line_font_name = font_name.clone();
                line_font_size = font_size;
                has_line = true;
            }
            "Tz" if in_text => {
                if let Some(v) = op.operands.first().and_then(operand_to_f64) {
                    h_scale = v / 100.0;
                }
            }
            "Tc" if in_text => {
                if let Some(v) = op.operands.first().and_then(operand_to_f64) {
                    char_spacing = v;
                }
            }
            "Tw" if in_text => {
                if let Some(v) = op.operands.first().and_then(operand_to_f64) {
                    word_spacing = v;
                }
            }
            "Tj" if in_text => {
                if let Some(s) = operand_to_string(&op.operands[0]) {
                    // If no line is active, start one at current text matrix.
                    if !has_line {
                        let effective_tm = multiply_matrices(ctm, text_matrix);
                        line_start_tm = effective_tm;
                        line_start_x = effective_tm[4];
                        line_end_x = effective_tm[4];
                        line_min_y = effective_tm[5];
                        line_max_y = effective_tm[5];
                        line_font_name = font_name.clone();
                        line_font_size = font_size;
                        has_line = true;
                    }
                    let est_width = estimate_text_width(&s, font_size, h_scale, char_spacing, word_spacing);
                    let baseline_tm = multiply_matrices(ctm, text_matrix);
                    let end = transform_point(ctm, text_matrix[4] + est_width, text_matrix[5]);
                    line_start_x = line_start_x.min(baseline_tm[4]).min(end.0);
                    line_end_x = line_end_x.max(baseline_tm[4]).max(end.0);
                    let baseline = baseline_tm[5];
                    line_min_y = line_min_y.min(baseline - font_size * 0.2);
                    line_max_y = line_max_y.max(baseline + font_size * 0.8);
                    current_text.push_str(&s);
                    // Advance text matrix (simplified).
                    text_matrix[4] += est_width;
                }
            }
            "TJ" if in_text => {
                if let Some(arr) = op.operands.first().and_then(|o| o.as_array().ok()) {
                    // If no line is active, start one at current text matrix.
                    if !has_line {
                        let effective_tm = multiply_matrices(ctm, text_matrix);
                        line_start_tm = effective_tm;
                        line_start_x = effective_tm[4];
                        line_end_x = effective_tm[4];
                        line_min_y = effective_tm[5];
                        line_max_y = effective_tm[5];
                        line_font_name = font_name.clone();
                        line_font_size = font_size;
                        has_line = true;
                    }
                    for item in arr {
                        if let Some(s) = operand_to_string(item) {
                            let est_width = estimate_text_width(&s, font_size, h_scale, char_spacing, word_spacing);
                            let baseline_tm = multiply_matrices(ctm, text_matrix);
                            let end = transform_point(ctm, text_matrix[4] + est_width, text_matrix[5]);
                            line_start_x = line_start_x.min(baseline_tm[4]).min(end.0);
                            line_end_x = line_end_x.max(baseline_tm[4]).max(end.0);
                            let baseline = baseline_tm[5];
                            line_min_y = line_min_y.min(baseline - font_size * 0.2);
                            line_max_y = line_max_y.max(baseline + font_size * 0.8);
                            current_text.push_str(&s);
                            text_matrix[4] += est_width;
                        } else if let Some(adj) = operand_to_f64(item) {
                            // Negative adjustment means move backward.
                            let adj_pts = -adj / 1000.0 * font_size * h_scale;
                            text_matrix[4] += adj_pts;
                        }
                    }
                }
            }
            // Skip Form XObjects (we don't recurse into them).
            "Do" => {
                // If a Form XObject is drawn, we conservatively bail out
                // because text inside it is invisible to this walk.
                if let Some(name_obj) = op.operands.first() {
                    if let Ok(name) = name_obj.as_name() {
                        let name_str = String::from_utf8_lossy(name);
                        if is_form_xobject(doc, page_id, &name_str) {
                            // Bail: return empty so caller falls back to v1.
                            return Ok(Vec::new());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Flush final line.
    flush_line(
        &mut current_text,
        &mut line_start_tm,
        &mut line_min_y,
        &mut line_max_y,
        &mut line_start_x,
        &mut line_end_x,
        &mut line_font_name,
        &mut line_font_size,
        &mut has_line,
        &mut lines,
    );

    // Post-process: filter out lines that are effectively empty
    // and clamp bbox to page.
    lines.retain(|l| !l.text.trim().is_empty());
    for line in &mut lines {
        line.bbox[0] = line.bbox[0].max(0.0);
        line.bbox[1] = line.bbox[1].max(0.0);
        line.bbox[2] = line.bbox[2].min(media[2]);
        line.bbox[3] = line.bbox[3].min(page_h);
    }

    Ok(lines)
}

fn op_to_six_f64(ops: &[Object]) -> Option<[f64; 6]> {
    if ops.len() < 6 {
        return None;
    }
    let mut out = [0.0; 6];
    for (i, o) in ops.iter().take(6).enumerate() {
        out[i] = operand_to_f64(o)?;
    }
    Some(out)
}

fn identity_matrix() -> [f64; 6] {
    [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
}

fn multiply_matrices(left: [f64; 6], right: [f64; 6]) -> [f64; 6] {
    [
        left[0] * right[0] + left[2] * right[1],
        left[1] * right[0] + left[3] * right[1],
        left[0] * right[2] + left[2] * right[3],
        left[1] * right[2] + left[3] * right[3],
        left[0] * right[4] + left[2] * right[5] + left[4],
        left[1] * right[4] + left[3] * right[5] + left[5],
    ]
}

fn transform_point(matrix: [f64; 6], x: f64, y: f64) -> (f64, f64) {
    (matrix[0] * x + matrix[2] * y + matrix[4], matrix[1] * x + matrix[3] * y + matrix[5])
}

fn operand_to_f64(o: &Object) -> Option<f64> {
    match o {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(f64::from(*r)),
        _ => None,
    }
}

fn operand_to_name(doc: &Document, o: &Object) -> Option<String> {
    match o {
        Object::Name(n) => Some(String::from_utf8_lossy(n).into_owned()),
        Object::Reference(id) => {
            doc.get_object(*id).ok().and_then(|obj| obj.as_name().ok()).map(|n| String::from_utf8_lossy(n).into_owned())
        }
        _ => None,
    }
}

fn operand_to_string(o: &Object) -> Option<String> {
    match o {
        Object::String(bytes, format) => {
            if format == &lopdf::StringFormat::Hexadecimal {
                Some(decode_hex_string(bytes))
            } else {
                Some(decode_literal_string(bytes))
            }
        }
        _ => None,
    }
}

/// Decode a parenthesized PDF literal string (PDFDocEncoding / WinAnsi fallback).
fn decode_literal_string(bytes: &[u8]) -> String {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' && i + 1 < bytes.len() {
            i += 1;
            match bytes[i] {
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'b' => out.push(0x08),
                b'f' => out.push(0x0C),
                b'(' => out.push(b'('),
                b')' => out.push(b')'),
                b'\\' => out.push(b'\\'),
                d @ b'0'..=b'7' => {
                    // Octal escape: up to 3 digits.
                    let mut oct = (d - b'0') as u32;
                    i += 1;
                    if i < bytes.len() && bytes[i].is_ascii_digit() {
                        oct = oct * 8 + (bytes[i] - b'0') as u32;
                        i += 1;
                        if i < bytes.len() && bytes[i].is_ascii_digit() {
                            oct = oct * 8 + (bytes[i] - b'0') as u32;
                            i += 1;
                        }
                    }
                    out.push((oct & 0xFF) as u8);
                    continue;
                }
                c => out.push(c),
            }
            i += 1;
        } else {
            out.push(b);
            i += 1;
        }
    }
    // Map bytes to chars using latin-1 / PDFDocEncoding for common range.
    out.iter().map(|&b| b as char).collect::<String>()
}

/// Decode a hex PDF string.
fn decode_hex_string(bytes: &[u8]) -> String {
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        let hi = bytes[i];
        let lo = bytes[i + 1];
        if let (Some(h), Some(l)) = (hex_val(hi), hex_val(lo)) {
            out.push((h << 4) | l);
        }
        i += 2;
    }
    // If odd number of digits, pad with trailing zero per spec.
    if bytes.len() % 2 == 1 {
        if let Some(h) = hex_val(bytes[bytes.len() - 1]) {
            out.push(h << 4);
        }
    }
    out.iter().map(|&b| b as char).collect::<String>()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' | b'A'..=b'F' => Some(b.to_ascii_lowercase() - b'a' + 10),
        _ => None,
    }
}

fn estimate_text_width(text: &str, font_size: f64, h_scale: f64, char_spacing: f64, _word_spacing: f64) -> f64 {
    let char_count = text.chars().count() as f64;
    let base = char_count * font_size * 0.5 * h_scale;
    let spacing = char_count * char_spacing * h_scale;
    base + spacing
}

/// Check whether the named font on this page is a Type3 font.
fn is_type3_font(doc: &Document, page_id: ObjectId, name: &str) -> bool {
    let Some(resources) = super::markdown_images::resolve_page_resources(doc, page_id) else { return false };
    let Ok(fonts) = resources.get(b"Font").and_then(|o| o.as_dict()) else { return false };
    let Ok(obj) = fonts.get(name.as_bytes()) else { return false };
    let id = match obj {
        Object::Reference(id) => *id,
        Object::Dictionary(d) => {
            return d.get(b"Subtype").and_then(|o| o.as_name()).map(|n| n == b"Type3").unwrap_or(false);
        }
        _ => return false,
    };
    let Ok(dict) = doc.get_dictionary(id) else { return false };
    dict.get(b"Subtype").and_then(|o| o.as_name()).map(|n| n == b"Type3").unwrap_or(false)
}

/// Check whether a named XObject on this page is a Form XObject.
fn is_form_xobject(doc: &Document, page_id: ObjectId, name: &str) -> bool {
    let Some(resources) = super::markdown_images::resolve_page_resources(doc, page_id) else { return false };
    let Ok(xobjects) = resources.get(b"XObject").and_then(|o| o.as_dict()) else { return false };
    let Ok(obj) = xobjects.get(name.as_bytes()) else { return false };
    let id = match obj {
        Object::Reference(id) => *id,
        _ => return false,
    };
    let Ok(dict) = doc.get_dictionary(id) else { return false };
    dict.get(b"Subtype").and_then(|o| o.as_name()).map(|n| n == b"Form").unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Object, Stream};

    fn build_doc_with_text(content_ops: &str) -> (Document, ObjectId) {
        let mut doc = Document::with_version("1.4");
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

    #[test]
    fn single_line_tj() {
        let ops = "BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello World) Tj ET";
        let (doc, page_id) = build_doc_with_text(ops);
        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Hello World");
        assert_eq!(lines[0].font_name, "F1");
        assert_eq!(lines[0].font_size, 12.0);
        assert!((lines[0].transform[4] - 100.0).abs() < 0.01);
        assert!((lines[0].transform[5] - 700.0).abs() < 0.01);
    }

    #[test]
    fn two_lines_td() {
        let ops = "BT /F1 12 Tf 1 0 0 1 100 700 Tm (First line) Tj 0 -14 Td (Second line) Tj ET";
        let (doc, page_id) = build_doc_with_text(ops);
        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "First line");
        assert_eq!(lines[1].text, "Second line");
        assert!((lines[1].transform[5] - 686.0).abs() < 0.01); // 700 - 14
    }

    #[test]
    fn tj_array() {
        let ops = "BT /F1 10 Tf 1 0 0 1 50 600 Tm [(He)10(ll) -5(o)] TJ ET";
        let (doc, page_id) = build_doc_with_text(ops);
        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Hello");
    }

    #[test]
    fn q_q_nesting() {
        let ops = "q BT /F1 12 Tf 1 0 0 1 100 700 Tm (Nested) Tj ET Q";
        let (doc, page_id) = build_doc_with_text(ops);
        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Nested");
    }

    #[test]
    fn cm_translation_applies_to_line_transform() {
        let ops = "q 1 0 0 1 50 25 cm BT /F1 12 Tf 1 0 0 1 100 700 Tm (Moved) Tj ET Q";
        let (doc, page_id) = build_doc_with_text(ops);
        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Moved");
        assert!((lines[0].transform[4] - 150.0).abs() < 0.01);
        assert!((lines[0].transform[5] - 725.0).abs() < 0.01);
    }

    #[test]
    fn type3_font_bails() {
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();

        let content_bytes = b"BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello) Tj ET".to_vec();
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
                                (b"Subtype".to_vec(), Object::Name(b"Type3".to_vec())),
                                (b"BaseFont".to_vec(), Object::Name(b"MyType3".to_vec())),
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

        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn direct_stream_contents() {
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();

        let content_bytes = b"BT /F1 12 Tf 1 0 0 1 100 700 Tm (Direct) Tj ET".to_vec();
        let content_stream = Object::Stream(Stream::new(Dictionary::new(), content_bytes));

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
                (b"Contents".to_vec(), content_stream),
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

        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Direct");
    }

    #[test]
    #[ignore = "lopdf content parser may not support hex strings in content streams"]
    fn hex_string_tj() {
        let ops = "BT /F1 12 Tf 1 0 0 1 100 700 Tm <48656c6c6f> Tj ET";
        let (doc, page_id) = build_doc_with_text(ops);
        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Hello");
    }

    #[test]
    fn escaped_parentheses_tj() {
        let ops = r"BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello \(world\)) Tj ET";
        let (doc, page_id) = build_doc_with_text(ops);
        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Hello (world)");
    }

    #[test]
    fn font_change_mid_line_flushes() {
        let ops = "BT /F1 12 Tf 1 0 0 1 100 700 Tm (Hello ) Tj /F2 14 Tf (World) Tj ET";
        let (doc, page_id) = build_doc_with_text(ops);
        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "Hello ");
        assert_eq!(lines[1].text, "World");
    }

    #[test]
    fn tstar_operator() {
        let ops = "BT /F1 12 Tf 1 0 0 1 100 700 Tm (First) Tj 14 TL T* (Second) Tj ET";
        let (doc, page_id) = build_doc_with_text(ops);
        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "First");
        assert_eq!(lines[1].text, "Second");
    }

    #[test]
    fn td_operator_sets_leading() {
        let ops = "BT /F1 12 Tf 1 0 0 1 100 700 Tm (First) Tj 0 -14 TD (Second) Tj ET";
        let (doc, page_id) = build_doc_with_text(ops);
        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].transform[5], 686.0); // 700 - 14
    }

    #[test]
    fn multiple_bt_et_blocks() {
        let ops = "BT /F1 12 Tf 1 0 0 1 100 700 Tm (Block1) Tj ET BT /F1 12 Tf 1 0 0 1 100 600 Tm (Block2) Tj ET";
        let (doc, page_id) = build_doc_with_text(ops);
        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "Block1");
        assert_eq!(lines[1].text, "Block2");
    }

    #[test]
    fn form_xobject_bails() {
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();
        let form_id = doc.new_object_id();

        // Form XObject
        doc.set_object(
            form_id,
            Object::Dictionary(Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
                (b"Subtype".to_vec(), Object::Name(b"Form".to_vec())),
                (
                    b"BBox".to_vec(),
                    Object::Array(vec![
                        Object::Integer(0),
                        Object::Integer(0),
                        Object::Integer(100),
                        Object::Integer(100),
                    ]),
                ),
            ])),
        );

        let content_bytes = b"q /Form1 Do Q".to_vec();
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
                        b"XObject".to_vec(),
                        Object::Dictionary(Dictionary::from_iter(vec![(
                            b"Form1".to_vec(),
                            Object::Reference(form_id),
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

        let lines = decode_page_text_lines(&doc, page_id).unwrap();
        assert!(lines.is_empty());
    }
}
