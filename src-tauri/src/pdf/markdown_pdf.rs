//! Minimal Markdown → PDF converter.
//!
//! Lays out markdown source as real vector text (Standard-14 Helvetica /
//! Helvetica-Bold / Courier — no font embedding) using hardcoded Adobe metrics
//! for line wrapping. Avoids the browser rasterization path (html2canvas)
//! entirely, which is prohibitively slow under WebKitGTK for long documents.

use crate::pdf::history::temp_hist_path;
use crate::pdf::io::save_atomic;
use lopdf::{Dictionary, Document, Object, Stream};
use std::path::Path;

const PAGE_W: f64 = 612.0;
const PAGE_H: f64 = 792.0;
const MARGIN: f64 = 54.0;
const CONTENT_W: f64 = PAGE_W - 2.0 * MARGIN;
const BODY_SIZE: f64 = 10.5;
const CODE_SIZE: f64 = 9.0;
const SMALL_SIZE: f64 = 8.5;
const LINE_LEADING: f64 = 1.45;
const PARA_GAP: f64 = 6.0;
const HEADING_SIZES: [f64; 6] = [20.0, 16.0, 13.5, 12.0, 11.0, 10.5];

#[derive(Debug, Clone)]
enum Block {
    Heading { level: usize, text: String },
    Paragraph(String),
    Code(String),
    ListItem { marker: String, text: String },
    Table(Vec<Vec<String>>),
    Rule,
}

/// Convert markdown source text to a PDF file.
pub fn markdown_to_pdf(text: &str, output: &Path) -> Result<(), String> {
    let blocks = parse_markdown(text);
    let pages = layout_pages(&blocks);
    write_pdf(&pages, output)
}

fn parse_markdown(text: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_end();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        if trimmed.starts_with("```") {
            i += 1;
            let mut buf = String::new();
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                buf.push_str(lines[i]);
                buf.push('\n');
                i += 1;
            }
            i += 1;
            blocks.push(Block::Code(buf));
            continue;
        }

        if rule_re(trimmed).is_some() {
            blocks.push(Block::Rule);
            i += 1;
            continue;
        }

        if let Some((level, text)) = heading(trimmed) {
            blocks.push(Block::Heading { level, text: strip_inline(&text) });
            i += 1;
            continue;
        }

        if trimmed.starts_with('|') {
            let mut rows: Vec<Vec<String>> = Vec::new();
            while i < lines.len() && lines[i].trim_start().starts_with('|') {
                let row = split_table_row(lines[i].trim());
                if !row.iter().all(|c| is_separator(c)) {
                    rows.push(row.into_iter().map(|c| strip_inline(&c)).collect());
                }
                i += 1;
            }
            if !rows.is_empty() {
                blocks.push(Block::Table(rows));
            }
            continue;
        }

        if let Some((marker, body)) = list_item(trimmed) {
            blocks.push(Block::ListItem { marker, text: strip_inline(&body) });
            i += 1;
            continue;
        }

        if trimmed.starts_with('>') {
            let mut buf = String::new();
            while i < lines.len() {
                let l = lines[i].trim_end();
                if let Some(rest) = l.strip_prefix('>') {
                    let rest = rest.strip_prefix(' ').unwrap_or(rest);
                    if !buf.is_empty() {
                        buf.push(' ');
                    }
                    buf.push_str(rest);
                    i += 1;
                } else {
                    break;
                }
            }
            blocks.push(Block::Paragraph(format!("“{}”", strip_inline(&buf))));
            continue;
        }

        let mut buf = String::new();
        while i < lines.len() {
            let l = lines[i].trim_end();
            if l.is_empty()
                || l.starts_with('#')
                || l.starts_with("```")
                || l.starts_with('|')
                || rule_re(l).is_some()
                || list_item(l).is_some()
                || l.starts_with('>')
            {
                break;
            }
            if !buf.is_empty() {
                buf.push(' ');
            }
            buf.push_str(l);
            i += 1;
        }
        blocks.push(Block::Paragraph(strip_inline(&buf)));
    }
    blocks
}

fn rule_re(line: &str) -> Option<()> {
    let t = line.trim();
    if t.len() >= 3 && t.chars().all(|c| c == '-' || c == '_' || c == '*') && t.chars().any(|c| c != ' ') {
        Some(())
    } else {
        None
    }
}

fn heading(line: &str) -> Option<(usize, String)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &line[hashes..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let text = rest.trim_start().trim_end_matches('#').trim().to_string();
    Some((hashes, text))
}

fn list_item(line: &str) -> Option<(String, String)> {
    let t = line.trim_start();
    if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("+ ")) {
        Some(("•".to_string(), rest.to_string()))
    } else if let Some(rest) = t.strip_prefix("[] ").or_else(|| t.strip_prefix("[ ] ")) {
        Some(("o".to_string(), rest.to_string()))
    } else if let Some(rest) =
        t.strip_prefix("[x] ").or_else(|| t.strip_prefix("[X] ")).or_else(|| t.strip_prefix("[v] "))
    {
        Some(("x".to_string(), rest.to_string()))
    } else {
        let digits_end = t.find('.').filter(|&pos| pos > 0 && t[..pos].chars().all(|c| c.is_ascii_digit()))?;
        let rest = t.get(digits_end + 1..)?.strip_prefix(' ')?;
        Some((format!("{}.", &t[..digits_end]), rest.to_string()))
    }
}

fn split_table_row(line: &str) -> Vec<String> {
    let inner = line.trim_matches('|');
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

fn is_separator(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c == '-' || c == ':' || c == ' ')
}

/// Strip inline markdown formatting (**bold**, *italic*, `code`, [text](url),
/// ~~strike~~) down to plain text. Collapses runs of whitespace.
fn strip_inline(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '`' {
            i += 1;
            while i < chars.len() && chars[i] != '`' {
                out.push(chars[i]);
                i += 1;
            }
            i += 1;
        } else if chars[i] == '[' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != ']' {
                out.push(chars[j]);
                j += 1;
            }
            if j < chars.len() {
                i = j + 1;
                if i < chars.len() && chars[i] == '(' {
                    while i < chars.len() && chars[i] != ')' {
                        i += 1;
                    }
                    i += 1;
                }
            } else {
                i = j;
            }
        } else if i + 1 < chars.len()
            && (chars[i] == '*' || chars[i] == '_' || chars[i] == '~')
            && chars[i] == chars[i + 1]
        {
            i += 2;
        } else if chars[i] == '*' || chars[i] == '_' {
            i += 1;
        } else if chars[i] == '\\' && i + 1 < chars.len() {
            out.push(chars[i + 1]);
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    let collapsed: String = out.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
}

// --- Adobe Standard-14 metrics (Helvetica / Helvetica-Bold / Courier) ---

fn helv_width(c: char) -> f64 {
    match c {
        ' ' => 278.0,
        '!' => 278.0,
        '"' => 355.0,
        '#' => 556.0,
        '$' => 556.0,
        '%' => 889.0,
        '&' => 667.0,
        '\'' => 191.0,
        '(' | ')' => 333.0,
        '*' => 389.0,
        '+' => 584.0,
        ',' => 278.0,
        '-' => 333.0,
        '.' => 278.0,
        '/' => 278.0,
        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => 556.0,
        ':' | ';' => 278.0,
        '<' | '=' | '>' => 584.0,
        '?' => 556.0,
        '@' => 1015.0,
        'A' => 667.0,
        'B' => 667.0,
        'C' => 722.0,
        'D' => 722.0,
        'E' => 667.0,
        'F' => 611.0,
        'G' => 778.0,
        'H' => 722.0,
        'I' => 278.0,
        'J' => 500.0,
        'K' => 667.0,
        'L' => 556.0,
        'M' => 833.0,
        'N' => 722.0,
        'O' => 778.0,
        'P' => 667.0,
        'Q' => 778.0,
        'R' => 722.0,
        'S' => 667.0,
        'T' => 611.0,
        'U' => 722.0,
        'V' => 667.0,
        'W' => 944.0,
        'X' => 667.0,
        'Y' => 667.0,
        'Z' => 611.0,
        '[' => 278.0,
        '\\' => 278.0,
        ']' => 278.0,
        '^' => 469.0,
        '_' => 556.0,
        '`' => 222.0,
        'a' => 556.0,
        'b' => 556.0,
        'c' => 500.0,
        'd' => 556.0,
        'e' => 556.0,
        'f' => 278.0,
        'g' => 556.0,
        'h' => 556.0,
        'i' => 222.0,
        'j' => 222.0,
        'k' => 500.0,
        'l' => 222.0,
        'm' => 833.0,
        'n' => 556.0,
        'o' => 556.0,
        'p' => 556.0,
        'q' => 556.0,
        'r' => 333.0,
        's' => 500.0,
        't' => 278.0,
        'u' => 556.0,
        'v' => 500.0,
        'w' => 722.0,
        'x' => 500.0,
        'y' => 500.0,
        'z' => 500.0,
        '{' => 334.0,
        '|' => 260.0,
        '}' => 334.0,
        '~' => 584.0,
        _ => 556.0,
    }
}

/// Map a char to a WinAnsi byte, or None if not representable.
fn winansi_byte(c: char) -> Option<u8> {
    if (' '..='~').contains(&c) {
        return Some(c as u8);
    }
    let b = match c {
        '©' => 169,
        '®' => 174,
        '™' => 153,
        '•' => 149,
        '—' => 151,
        '–' => 150,
        '…' => 133,
        '\u{2018}' => 145,
        '\u{2019}' => 146,
        '\u{201C}' => 147,
        '\u{201D}' => 148,
        '„' => 132,
        '\u{2020}' => 134,
        '\u{2021}' => 135,
        '\u{20AC}' => 128,
        '°' => 176,
        '±' => 177,
        'µ' => 181,
        '·' => 183,
        '¼' => 188,
        '½' => 189,
        '¾' => 190,
        '×' => 215,
        '÷' => 247,
        _ => return None,
    };
    Some(b as u8)
}

/// Build a PDF text-showing string body (escaped, WinAnsi-encoded).
fn pdf_text(input: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for c in input.chars() {
        let b = winansi_byte(c).unwrap_or(b'?');
        match b {
            b'(' | b')' | b'\\' => {
                out.push(b'\\');
                out.push(b);
            }
            _ => out.push(b),
        }
    }
    out
}

/// Word-wrap `text` into lines that fit within `max_width` points at `size`
/// using proportional Helvetica metrics.
fn wrap_proportional(text: &str, max_width: f64, size: f64) -> Vec<String> {
    let unit = size / 1000.0;
    let mut lines: Vec<String> = Vec::new();
    for raw_paragraph in text.split('\n') {
        let words: Vec<&str> = raw_paragraph.split_whitespace().collect();
        if words.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        let mut current_w = 0.0;
        for word in words {
            let word_w: f64 = word.chars().map(|c| helv_width(c) * unit).sum::<f64>() + unit * helv_width(' ');
            if !current.is_empty() && current_w + word_w > max_width {
                lines.push(current.trim_end().to_string());
                current.clear();
                current_w = 0.0;
            }
            current.push_str(word);
            current.push(' ');
            current_w += word_w;
        }
        if !current.is_empty() {
            lines.push(current.trim_end().to_string());
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Wrap for monospace Courier (uniform 600-unit width).
fn wrap_monospace(text: &str, max_width: f64, size: f64) -> Vec<String> {
    let char_w = size * 0.6;
    let max_chars = (max_width / char_w).floor().max(1.0) as usize;
    let mut lines: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        if raw.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut start = 0;
        let chars: Vec<char> = raw.chars().collect();
        while start < chars.len() {
            let end = (start + max_chars).min(chars.len());
            lines.push(chars[start..end].iter().collect());
            start = end;
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

struct PageLines {
    pages: Vec<Vec<u8>>,
    current: Vec<u8>,
    y: f64,
}

impl PageLines {
    fn new() -> Self {
        PageLines { pages: Vec::new(), current: Vec::new(), y: PAGE_H - MARGIN }
    }

    fn ensure(&mut self, needed: f64) {
        if self.y - needed < MARGIN {
            self.flush();
        }
    }

    fn line(&mut self, font: &str, size: f64, x: f64, text: &str, leading: f64) {
        self.ensure(leading);
        let y = self.y - size;
        self.current.extend_from_slice(b"BT /");
        self.current.extend_from_slice(font.as_bytes());
        self.current.extend_from_slice(format!(" {size:.2} Tf 1 0 0 1 {x:.2} {y:.2} Tm (").as_bytes());
        self.current.extend_from_slice(&pdf_text(text));
        self.current.extend_from_slice(b") Tj ET\n");
        self.y -= leading;
    }

    fn gap(&mut self, g: f64) {
        self.y -= g;
    }

    fn flush(&mut self) {
        let content = std::mem::take(&mut self.current);
        if !content.is_empty() {
            self.pages.push(content);
        } else {
            self.pages.push(b"BT /F1 1 Tf 1 0 0 1 1 1 Tm ( ) Tj ET\n".to_vec());
        }
        self.y = PAGE_H - MARGIN;
    }
}

fn layout_pages(blocks: &[Block]) -> Vec<Vec<u8>> {
    let mut pg = PageLines::new();
    for block in blocks {
        match block {
            Block::Heading { level, text } => {
                let lvl = (*level - 1).min(HEADING_SIZES.len() - 1);
                let size = HEADING_SIZES[lvl];
                pg.gap(PARA_GAP);
                let leading = size * LINE_LEADING;
                for ln in wrap_proportional(text, CONTENT_W, size) {
                    pg.line("F2", size, MARGIN, &ln, leading);
                }
                pg.gap(PARA_GAP * 0.5);
            }
            Block::Paragraph(text) => {
                pg.gap(PARA_GAP * 0.5);
                let leading = BODY_SIZE * LINE_LEADING;
                for ln in wrap_proportional(text, CONTENT_W, BODY_SIZE) {
                    pg.line("F1", BODY_SIZE, MARGIN, &ln, leading);
                }
                pg.gap(PARA_GAP * 0.5);
            }
            Block::Code(text) => {
                pg.gap(PARA_GAP * 0.5);
                let leading = CODE_SIZE * 1.3;
                let indent = MARGIN + 12.0;
                for ln in wrap_monospace(text, CONTENT_W - 24.0, CODE_SIZE) {
                    pg.line("F3", CODE_SIZE, indent, &ln, leading);
                }
                pg.gap(PARA_GAP * 0.5);
            }
            Block::ListItem { marker, text, .. } => {
                let leading = BODY_SIZE * LINE_LEADING;
                let label_w = (marker.len() as f64 + 1.0) * BODY_SIZE * 0.5;
                let body_w = CONTENT_W - label_w - 8.0;
                let wrapped = wrap_proportional(text, body_w, BODY_SIZE);
                for (idx, ln) in wrapped.iter().enumerate() {
                    let prefix = if idx == 0 { format!("{}  ", marker) } else { String::new() };
                    let x = if idx == 0 { MARGIN } else { MARGIN + label_w + 8.0 };
                    let display = if idx == 0 { format!("{}{}", prefix, ln) } else { ln.clone() };
                    pg.line("F1", BODY_SIZE, x, &display, leading);
                }
                pg.gap(2.0);
            }
            Block::Table(rows) => {
                pg.gap(PARA_GAP * 0.5);
                let cols = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
                let col_w = CONTENT_W / cols as f64;
                let leading = SMALL_SIZE * LINE_LEADING;
                for row in rows {
                    let mut max_lines = 1usize;
                    let mut cell_wrapped: Vec<Vec<String>> = Vec::with_capacity(cols);
                    for cell in row.iter() {
                        let wrapped = wrap_proportional(cell, col_w - 8.0, SMALL_SIZE);
                        max_lines = max_lines.max(wrapped.len());
                        cell_wrapped.push(wrapped);
                    }
                    for li in 0..max_lines {
                        for ci in 0..cols {
                            let txt = cell_wrapped.get(ci).and_then(|w| w.get(li)).map(|s| s.as_str()).unwrap_or("");
                            let font = if li == 0 { "F2" } else { "F1" };
                            let x = MARGIN + ci as f64 * col_w + 3.0;
                            pg.line(font, SMALL_SIZE, x, txt, leading);
                        }
                    }
                    pg.gap(2.0);
                }
                pg.gap(PARA_GAP * 0.5);
            }
            Block::Rule => {
                pg.gap(PARA_GAP * 0.5);
                let y = self_y(&pg);
                pg.current.extend_from_slice(
                    format!("0.7 0.7 0.7 RG {MARGIN:.2} {y:.2} m {:.2} {y:.2} l S\n", MARGIN + CONTENT_W).as_bytes(),
                );
                pg.gap(PARA_GAP);
            }
        }
    }
    pg.flush();
    pg.pages
}

fn self_y(pg: &PageLines) -> f64 {
    pg.y - 2.0
}

fn write_pdf(pages: &[Vec<u8>], output: &Path) -> Result<(), String> {
    let mut doc = Document::with_version("1.4");
    let pages_id = doc.new_object_id();
    let font_ids = add_fonts(&mut doc);
    let mut kids = Vec::with_capacity(pages.len());
    for content in pages {
        let content_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), content.clone())));
        let mut resources = Dictionary::new();
        let mut font_dict = Dictionary::new();
        font_dict.set(b"F1", Object::Reference(font_ids[0]));
        font_dict.set(b"F2", Object::Reference(font_ids[1]));
        font_dict.set(b"F3", Object::Reference(font_ids[2]));
        resources.set(b"Font", Object::Dictionary(font_dict));
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set("Resources", Object::Dictionary(resources));
        page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), PAGE_W.into(), PAGE_H.into()]));
        page.set("Contents", Object::Reference(content_id));
        kids.push(Object::Reference(doc.add_object(Object::Dictionary(page))));
    }
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Count", Object::Integer(kids.len() as i64));
    pages_dict.set("Kids", Object::Array(kids));
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));
    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer.set("Root", Object::Reference(catalog_id));
    save_atomic(&mut doc, output)
}

fn add_fonts(doc: &mut Document) -> [lopdf::ObjectId; 3] {
    let f1 = doc.add_object(font_dict("Helvetica"));
    let f2 = doc.add_object(font_dict("Helvetica-Bold"));
    let f3 = doc.add_object(font_dict("Courier"));
    [f1, f2, f3]
}

fn font_dict(name: &str) -> Object {
    let mut d = Dictionary::new();
    d.set("Type", Object::Name(b"Font".to_vec()));
    d.set("Subtype", Object::Name(b"Type1".to_vec()));
    d.set("BaseFont", Object::Name(name.as_bytes().to_vec()));
    d.set("Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
    Object::Dictionary(d)
}

/// Render markdown text to a temp working PDF and return its path.
pub fn write_markdown_pdf(text: &str) -> Result<String, String> {
    let path = temp_hist_path("document", "pdf");
    markdown_to_pdf(text, &path)?;
    Ok(path.to_string_lossy().into_owned())
}
