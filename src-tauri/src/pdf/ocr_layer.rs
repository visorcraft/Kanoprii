use crate::pdf::coords::page_media_box;
use crate::pdf::export::{EXPORT_RENDER_H, EXPORT_RENDER_W};
use crate::pdf::io::mutate_pdf;
use crate::pdf::ocr::{ocr_available, ocr_missing_hint, ocr_png_to_tsv};
use crate::pdf::page_text::{ensure_helvetica_font, escape_pdf_literal_string};
use crate::pdf::pdfium_bind::get_pdfium;
use crate::pdf::render;
use crate::pdf::text_layer::get_page_text_layout;
use lopdf::Document;
use pdfium_render::prelude::Pdfium;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct TsvWord {
    pub text: String,
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

pub fn parse_tsv_words(tsv: &str) -> Vec<TsvWord> {
    let mut out = Vec::new();
    for line in tsv.lines().skip(1) {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 12 {
            continue;
        }
        let level: i32 = cols[0].parse().unwrap_or(-1);
        if level != 5 {
            continue;
        }
        let conf: i32 = cols[10].parse().unwrap_or(-1);
        if conf < 0 {
            continue;
        }
        let text = cols[11].trim();
        if text.is_empty() {
            continue;
        }
        let left = cols[6].parse().unwrap_or(0.0);
        let top = cols[7].parse().unwrap_or(0.0);
        let width = cols[8].parse().unwrap_or(0.0);
        let height = cols[9].parse().unwrap_or(0.0);
        if width <= 0.0 || height <= 0.0 {
            continue;
        }
        out.push(TsvWord { text: text.to_string(), left, top, width, height });
    }
    out
}

pub fn invisible_text_ops(
    words: &[TsvWord],
    img_w: f64,
    img_h: f64,
    page_w: f64,
    page_h: f64,
    font_name: &str,
) -> String {
    if words.is_empty() || img_w <= 0.0 || img_h <= 0.0 {
        return String::new();
    }
    let sx = page_w / img_w;
    let sy = page_h / img_h;
    let mut ops = String::from("\nBT\n3 Tr\n");
    for word in words {
        let font_size = (word.height * sy).max(1.0);
        let est_width = (word.text.chars().count() as f64 * font_size * 0.5).max(font_size);
        let scale_x = (word.width * sx) / est_width;
        let tx = word.left * sx;
        let ty = page_h - (word.top + word.height) * sy;
        let escaped = escape_pdf_literal_string(&word.text);
        ops.push_str(&format!(
            "/{font_name} {font_size} Tf {scale_x} 0 0 1 {tx} {ty} Tm ({escaped}) Tj\n",
            font_name = font_name,
            font_size = font_size,
            scale_x = scale_x,
            tx = tx,
            ty = ty,
            escaped = escaped,
        ));
    }
    ops.push_str("ET\n");
    ops
}

pub fn add_invisible_text_layer(
    doc: &mut Document,
    page_index: u32,
    words: &[TsvWord],
    img_w: f64,
    img_h: f64,
) -> Result<(), String> {
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    let media = page_media_box(doc, page_id)?;
    let page_w = media[2] - media[0];
    let page_h = media[3] - media[1];
    let font_name = ensure_helvetica_font(doc, page_id)?;
    let ops = invisible_text_ops(words, img_w, img_h, page_w, page_h, &font_name);
    if ops.is_empty() {
        return Ok(());
    }
    crate::pdf::content::append_page_content(doc, page_id, ops.as_bytes())
}

fn page_visible_char_count(pdfium: &Pdfium, path: &Path, page_index: u32) -> Result<usize, String> {
    Ok(get_page_text_layout(pdfium, path, page_index)?
        .iter()
        .map(|run| run.text.chars().filter(|c| !c.is_control()).count())
        .sum())
}

pub fn make_pdf_searchable(path: &Path, start_page: u32, end_page: u32) -> Result<u32, String> {
    let total = Document::load(path).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }

    let mut ocr_count = 0u32;
    for page_index in start_page..=end_page {
        let char_count = {
            let pdfium = get_pdfium()?;
            page_visible_char_count(&pdfium, path, page_index)?
        };
        if char_count > 16 {
            continue;
        }
        if !ocr_available() {
            return Err(ocr_missing_hint("Make Searchable").trim().to_string());
        }

        let png = {
            let pdfium = get_pdfium()?;
            render::render_page_bytes(
                &pdfium,
                path,
                page_index,
                EXPORT_RENDER_W,
                EXPORT_RENDER_H,
                image::ImageFormat::Png,
            )?
        };

        let tsv = ocr_png_to_tsv(&png)?;
        let words = parse_tsv_words(&tsv);
        if words.is_empty() {
            continue;
        }

        mutate_pdf(path, |doc| {
            add_invisible_text_layer(doc, page_index, &words, f64::from(EXPORT_RENDER_W), f64::from(EXPORT_RENDER_H))
        })?;
        ocr_count += 1;
    }
    Ok(ocr_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::io::mutate_pdf;
    use lopdf::{Dictionary, Object, Stream};

    #[test]
    fn parse_tsv_words_extracts_word_boxes() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
4\t1\t1\t1\t1\t0\t0\t0\t100\t20\t90\tnoise\n\
5\t1\t1\t1\t1\t1\t100\t50\t80\t20\t85\tHello\n\
5\t1\t1\t1\t2\t1\t200\t50\t60\t20\t-1\tSkip\n";
        let words = parse_tsv_words(tsv);
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "Hello");
        assert!((words[0].left - 100.0).abs() < f64::EPSILON);
        assert!((words[0].top - 50.0).abs() < f64::EPSILON);
        assert!((words[0].width - 80.0).abs() < f64::EPSILON);
        assert!((words[0].height - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn invisible_text_ops_emit_render_mode_3() {
        let words = vec![TsvWord { text: "Hi".into(), left: 100.0, top: 50.0, width: 80.0, height: 20.0 }];
        let ops = invisible_text_ops(&words, 1600.0, 2264.0, 612.0, 792.0, "Helv");
        assert!(ops.contains("BT"));
        assert!(ops.contains("3 Tr"));
        assert!(ops.contains("Tm"));
        assert!(ops.contains("(Hi)"));
        assert!(ops.contains("ET"));
    }

    fn blank_page_pdf(path: &std::path::Path) {
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();
        doc.objects.insert(content_id, Object::Stream(Stream::new(Dictionary::new(), Vec::new())));
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set("Resources", Object::Dictionary(Dictionary::new()));
        page.set(
            "MediaBox",
            Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(612), Object::Integer(792)]),
        );
        page.set("Contents", Object::Reference(content_id));
        doc.objects.insert(page_id, Object::Dictionary(page));
        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name(b"Pages".to_vec()));
        pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let catalog_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.save(path).unwrap();
    }

    #[test]
    fn add_invisible_text_layer_appends_content() {
        let path = std::env::temp_dir().join(format!("ocr_layer_test_{}.pdf", std::process::id()));
        blank_page_pdf(&path);
        let words = vec![TsvWord { text: "SECRET".into(), left: 10.0, top: 10.0, width: 80.0, height: 20.0 }];
        mutate_pdf(&path, |doc| add_invisible_text_layer(doc, 0, &words, 1600.0, 2264.0)).unwrap();
        let doc = Document::load(&path).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let content = crate::pdf::page_text::read_page_content(&doc, page_id).unwrap();
        let text = String::from_utf8_lossy(&content);
        assert!(text.contains("3 Tr"));
        assert!(text.contains("SECRET"));
        assert_eq!(doc.get_pages().len(), 1);
        let _ = std::fs::remove_file(path);
    }
}
