use crate::pdf::markdown_heuristic::{
    autolink_inline_text, collect_page_links, format_markdown_lines, lines_from_pdfium_text, polish_heuristic_lines,
    MarkdownPageLink, MarkdownTextLine,
};
use crate::pdf::markdown_images::{
    append_page_embedded_images, append_page_ocr_supplement, append_scanned_page_markdown, page_needs_ocr_supplement,
    MarkdownImageSink,
};
use crate::pdf::markdown_tagged::{plain_text_to_markdown, tagged_markdown_by_page, tagged_page_has_content};
use crate::pdf::ocr::{ocr_language, resolve_tesseract, OcrExportStats};
use crate::pdf::pdfium_bind::get_pdfium;
use lopdf::Document;
use std::collections::BTreeMap;
use std::path::Path;

/// Per-page text plan from PDFium. Built while the PDFium mutex is held; OCR and
/// image rendering run afterward so nested `render_page_png` calls cannot deadlock.
pub enum PdfPageMarkdownPlan {
    Tagged(String),
    Scanned,
    Plain { text: String, needs_ocr_supplement: bool },
    Heuristic { lines: Vec<MarkdownTextLine>, links: Vec<MarkdownPageLink>, needs_ocr_supplement: bool },
}

fn extract_pdf_page_markdown_plans(
    path: &Path,
    tagged_pages: &Option<BTreeMap<u32, String>>,
) -> Result<Vec<PdfPageMarkdownPlan>, String> {
    // Use PDFium's text layer: it decodes font encodings (including CID/Type0
    // fonts) that a raw content-stream walk cannot, so real-world PDFs actually
    // produce text instead of empty pages.
    let pdfium = get_pdfium()?;
    let document = pdfium.load_pdf_from_file(path, None).map_err(|e| e.to_string())?;
    let mut plans = Vec::with_capacity(document.pages().len() as usize);
    for (index, page) in document.pages().iter().enumerate() {
        if let Some(page_md) = tagged_pages
            .as_ref()
            .and_then(|pages| pages.get(&(index as u32)))
            .filter(|page_md| tagged_page_has_content(page_md))
        {
            plans.push(PdfPageMarkdownPlan::Tagged(page_md.clone()));
            continue;
        }
        let text = page.text().map_err(|e| e.to_string())?;
        let links = collect_page_links(&page);
        let lines = polish_heuristic_lines(lines_from_pdfium_text(&text));
        if lines.is_empty() {
            let trimmed = text.all().trim().to_string();
            if trimmed.is_empty() {
                plans.push(PdfPageMarkdownPlan::Scanned);
            } else {
                let needs_ocr_supplement = page_needs_ocr_supplement(&[], &trimmed);
                plans.push(PdfPageMarkdownPlan::Plain { text: trimmed, needs_ocr_supplement });
            }
        } else {
            let needs_ocr_supplement = page_needs_ocr_supplement(&lines, "");
            plans.push(PdfPageMarkdownPlan::Heuristic { lines, links, needs_ocr_supplement });
        }
    }
    Ok(plans)
}

pub fn pdf_to_markdown(
    path: &Path,
    image_sink: Option<&MarkdownImageSink<'_>>,
    stats: Option<&mut OcrExportStats>,
) -> Result<String, String> {
    let lopdf_doc = Document::load(path).map_err(|e| e.to_string())?;
    let tagged_pages = tagged_markdown_by_page(&lopdf_doc);

    let mut scratch_stats = OcrExportStats::default();
    let stats = if let Some(stats) = stats { stats } else { &mut scratch_stats };
    stats.available = resolve_tesseract().is_some();
    stats.language = ocr_language();

    let page_plans = extract_pdf_page_markdown_plans(path, &tagged_pages)?;

    let mut markdown = String::from("# PDF to Markdown Conversion\n\n");
    for (index, plan) in page_plans.iter().enumerate() {
        markdown.push_str(&format!("## Page {}\n\n", index + 1));

        match plan {
            PdfPageMarkdownPlan::Tagged(page_md) => markdown.push_str(page_md),
            PdfPageMarkdownPlan::Scanned => {
                append_scanned_page_markdown(&mut markdown, path, index as u32, image_sink, stats)?;
            }
            PdfPageMarkdownPlan::Plain { text, needs_ocr_supplement } => {
                markdown.push_str(&plain_text_to_markdown(&autolink_inline_text(text)));
                if image_sink.is_some() && *needs_ocr_supplement {
                    append_page_ocr_supplement(&mut markdown, path, index as u32, stats)?;
                }
            }
            PdfPageMarkdownPlan::Heuristic { lines, links, needs_ocr_supplement } => {
                markdown.push_str(&format_markdown_lines(lines, links));
                if image_sink.is_some() && *needs_ocr_supplement {
                    append_page_ocr_supplement(&mut markdown, path, index as u32, stats)?;
                }
            }
        }

        if let Some(sink) = image_sink {
            let mut image_seq = 0u32;
            markdown.push_str(&append_page_embedded_images(
                &lopdf_doc,
                (index + 1) as u32,
                sink,
                &mut image_seq,
                stats,
            )?);
        }
    }
    Ok(markdown)
}

pub fn pdf_plain_text_pages(path: &Path) -> Result<(Vec<String>, u32), String> {
    let lopdf_doc = Document::load(path).map_err(|e| e.to_string())?;
    let tagged_pages = tagged_markdown_by_page(&lopdf_doc);
    let pdfium = get_pdfium()?;
    let document = pdfium.load_pdf_from_file(path, None).map_err(|e| e.to_string())?;
    let mut pages = Vec::new();
    let mut scanned_pages = 0u32;
    for (index, page) in document.pages().iter().enumerate() {
        let page_text = if let Some(page_md) = tagged_pages
            .as_ref()
            .and_then(|pages| pages.get(&(index as u32)))
            .filter(|page_md| tagged_page_has_content(page_md))
        {
            crate::pdf::summary::strip_markdown_for_summary_page(page_md)
        } else {
            let text = page.text().map_err(|e| e.to_string())?;
            let lines = polish_heuristic_lines(lines_from_pdfium_text(&text));
            if lines.is_empty() {
                let all_text = text.all();
                let trimmed = all_text.trim();
                if trimmed.is_empty() {
                    scanned_pages += 1;
                    String::new()
                } else {
                    trimmed.to_string()
                }
            } else {
                lines.iter().map(|line| line.text.as_str()).collect::<Vec<_>>().join("\n")
            }
        };
        pages.push(page_text);
    }
    Ok((pages, scanned_pages))
}
