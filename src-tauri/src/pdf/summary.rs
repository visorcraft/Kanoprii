use serde::Serialize;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfIntelligentExtraction {
    pub headings: Vec<String>,
    pub emails: Vec<String>,
    pub urls: Vec<String>,
    pub dates: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfSummaryResult {
    pub page_count: u32,
    pub word_count: u32,
    pub title_guess: Option<String>,
    pub overview: String,
    pub key_points: Vec<String>,
    pub extraction: PdfIntelligentExtraction,
    pub scanned_pages: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummarySaveResult {
    pub summary: PdfSummaryResult,
    pub summary_path: String,
    pub written: bool,
    pub conflict: bool,
}

const SUMMARY_STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "had", "has", "have", "he", "her", "his", "in",
    "is", "it", "its", "of", "on", "or", "that", "the", "their", "they", "this", "to", "was", "were", "will", "with",
];

fn is_summary_stopword(word: &str) -> bool {
    SUMMARY_STOPWORDS.contains(&word)
}

fn normalize_inline_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_markdown_for_summary(markdown: &str) -> String {
    markdown
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with("## Page ")
                && !trimmed.starts_with("# PDF to Markdown")
                && !trimmed.starts_with('|')
                && !trimmed.starts_with("![")
                && trimmed != "_(no extractable text on this page)_"
        })
        .map(|line| line.trim_start_matches('#').trim())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let trimmed = current.trim();
            if trimmed.len() > 8 && trimmed.chars().any(|c| c.is_alphabetic()) {
                sentences.push(normalize_inline_text(trimmed));
            }
            current.clear();
        }
    }
    let tail = current.trim();
    if tail.len() > 8 && tail.chars().any(|c| c.is_alphabetic()) {
        sentences.push(normalize_inline_text(tail));
    }
    sentences
}

fn count_words(text: &str) -> u32 {
    text.split_whitespace().filter(|word| !word.is_empty()).count() as u32
}

fn collect_term_frequencies(sentences: &[String]) -> HashMap<String, u32> {
    let mut freq = HashMap::new();
    for sentence in sentences {
        for word in sentence
            .split(|c: char| !c.is_alphanumeric())
            .map(str::to_ascii_lowercase)
            .filter(|word| word.len() > 2 && !is_summary_stopword(word))
        {
            *freq.entry(word).or_insert(0) += 1;
        }
    }
    freq
}

fn score_sentence_for_summary(sentence: &str, index: usize, total: usize, term_freq: &HashMap<String, u32>) -> f32 {
    let words: Vec<&str> = sentence.split_whitespace().collect();
    let word_count = words.len();
    if !(4..=60).contains(&word_count) {
        return 0.0;
    }
    let mut score = 0.0f32;
    if index < total / 5 {
        score += 1.5;
    }
    if (12..=40).contains(&word_count) {
        score += 1.0;
    }
    for word in words {
        let key = word.to_ascii_lowercase();
        if let Some(count) = term_freq.get(&key) {
            score += (*count as f32).sqrt();
        }
    }
    if sentence.chars().filter(|c| c.is_uppercase()).count() > 2 {
        score += 0.5;
    }
    score
}

fn extractive_overview(sentences: &[String], term_freq: &HashMap<String, u32>, max_sentences: usize) -> String {
    if sentences.is_empty() {
        return String::new();
    }
    let total = sentences.len();
    let mut ranked: Vec<(usize, f32)> = sentences
        .iter()
        .enumerate()
        .map(|(index, sentence)| (index, score_sentence_for_summary(sentence, index, total, term_freq)))
        .filter(|(_, score)| *score > 0.0)
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut picked = ranked.into_iter().take(max_sentences).map(|(index, _)| index).collect::<Vec<_>>();
    picked.sort_unstable();
    picked.into_iter().map(|index| sentences[index].clone()).collect::<Vec<_>>().join(" ")
}

fn looks_like_heading_line(line: &str) -> bool {
    let text = line.trim();
    if text.is_empty() || text.len() > 120 {
        return false;
    }
    let words = text.split_whitespace().count();
    if words > 14 {
        return false;
    }
    text.chars().next().is_some_and(|ch| ch.is_uppercase()) && !text.ends_with('.')
}

fn extract_key_points(sentences: &[String], headings: &[String], max_points: usize) -> Vec<String> {
    let mut points = BTreeSet::new();
    for heading in headings.iter().take(max_points) {
        points.insert(heading.clone());
    }
    for sentence in sentences {
        let trimmed = sentence.trim();
        if trimmed.starts_with("- ") || trimmed.starts_with("• ") {
            points.insert(trimmed.trim_start_matches(['-', '•', ' ']).to_string());
        } else if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
            if rest.starts_with('.') || rest.starts_with(')') {
                points.insert(trimmed.to_string());
            }
        }
        if points.len() >= max_points {
            break;
        }
    }
    if points.len() < max_points {
        let term_freq = collect_term_frequencies(sentences);
        let total = sentences.len();
        let mut ranked: Vec<(usize, f32)> = sentences
            .iter()
            .enumerate()
            .map(|(index, sentence)| (index, score_sentence_for_summary(sentence, index, total, &term_freq)))
            .filter(|(_, score)| *score > 0.0)
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (index, _) in ranked {
            let sentence = &sentences[index];
            if sentence.len() <= 160 {
                points.insert(sentence.clone());
            }
            if points.len() >= max_points {
                break;
            }
        }
    }
    points.into_iter().take(max_points).collect()
}

fn trim_token_edges(token: &str) -> String {
    token
        .trim_matches(|c: char| {
            !c.is_alphanumeric() && c != '@' && c != '.' && c != '_' && c != '-' && c != '/' && c != ':'
        })
        .to_string()
}

fn extract_emails(text: &str) -> Vec<String> {
    let mut emails = BTreeSet::new();
    for token in text.split_whitespace() {
        let cleaned = trim_token_edges(token);
        if cleaned.contains('@')
            && cleaned.contains('.')
            && !cleaned.starts_with('@')
            && cleaned.len() >= 5
            && cleaned.chars().all(|c| c.is_alphanumeric() || "@._-+".contains(c))
        {
            emails.insert(cleaned);
        }
    }
    emails.into_iter().collect()
}

fn extract_urls(text: &str) -> Vec<String> {
    let mut urls = BTreeSet::new();
    for token in text.split_whitespace() {
        let cleaned = trim_token_edges(token);
        if cleaned.starts_with("http://") || cleaned.starts_with("https://") || cleaned.starts_with("www.") {
            urls.insert(cleaned);
        }
    }
    urls.into_iter().collect()
}

fn looks_like_date_token(token: &str) -> bool {
    let token = trim_token_edges(token);
    if token.len() < 6 || token.len() > 32 {
        return false;
    }
    let digits = token.chars().filter(|c| c.is_ascii_digit()).count();
    if digits < 4 {
        return false;
    }
    let has_sep = token.contains('/') || token.contains('-') || token.contains('.');
    let month_names = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
        "jan",
        "feb",
        "mar",
        "apr",
        "jun",
        "jul",
        "aug",
        "sep",
        "oct",
        "nov",
        "dec",
    ];
    let lower = token.to_ascii_lowercase();
    month_names.iter().any(|month| lower.contains(month)) || has_sep
}

fn extract_dates(text: &str) -> Vec<String> {
    let mut dates = BTreeSet::new();
    for token in text.split_whitespace() {
        if looks_like_date_token(token) {
            dates.insert(trim_token_edges(token));
        }
    }
    dates.into_iter().collect()
}

pub fn intelligent_extract_from_text(text: &str) -> PdfIntelligentExtraction {
    let mut headings = BTreeSet::new();
    for line in text.lines() {
        if looks_like_heading_line(line) {
            headings.insert(normalize_inline_text(line));
        }
    }
    PdfIntelligentExtraction {
        headings: headings.into_iter().take(24).collect(),
        emails: extract_emails(text),
        urls: extract_urls(text),
        dates: extract_dates(text),
    }
}

fn guess_title(first_page: &str, headings: &[String]) -> Option<String> {
    if let Some(heading) = headings.first() {
        if heading.len() <= 120 {
            return Some(heading.clone());
        }
    }
    first_page
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && line.len() <= 120 && looks_like_heading_line(line))
        .map(normalize_inline_text)
}

pub fn build_pdf_summary(pages: &[String], scanned_pages: u32) -> PdfSummaryResult {
    let page_count = pages.len() as u32;
    let full_text = pages.iter().filter(|page| !page.trim().is_empty()).cloned().collect::<Vec<_>>().join("\n\n");
    let word_count = count_words(&full_text);
    let extraction = intelligent_extract_from_text(&full_text);
    let sentences = split_sentences(&full_text);
    let term_freq = collect_term_frequencies(&sentences);
    let overview = if sentences.is_empty() {
        if scanned_pages > 0 {
            format!(
                "No extractable text was found. {scanned_pages} page(s) appear scanned or image-only (use Markdown export with Tesseract OCR for those pages)."
            )
        } else {
            "No extractable text was found in this document.".to_string()
        }
    } else {
        extractive_overview(&sentences, &term_freq, 4)
    };
    let key_points = extract_key_points(&sentences, &extraction.headings, 8);
    let title_guess = guess_title(pages.first().map(String::as_str).unwrap_or_default(), &extraction.headings);
    PdfSummaryResult { page_count, word_count, title_guess, overview, key_points, extraction, scanned_pages }
}

pub fn summary_markdown_path(pdf_path: &Path) -> PathBuf {
    pdf_path.with_extension("summary.md")
}

pub fn summary_to_markdown(summary: &PdfSummaryResult) -> String {
    let mut output = String::from("# Document Summary\n\n");
    if let Some(title) = &summary.title_guess {
        output.push_str(&format!("**Title guess:** {title}\n\n"));
    }
    output.push_str(&format!(
        "**Pages:** {} · **Words:** {} · **Scanned/image-only pages:** {}\n\n",
        summary.page_count, summary.word_count, summary.scanned_pages
    ));
    output.push_str("## Overview\n\n");
    output.push_str(&summary.overview);
    output.push_str("\n\n## Key points\n\n");
    if summary.key_points.is_empty() {
        output.push_str("_(none)_\n\n");
    } else {
        for point in &summary.key_points {
            output.push_str(&format!("- {point}\n"));
        }
        output.push('\n');
    }
    output.push_str("## Extracted headings\n\n");
    if summary.extraction.headings.is_empty() {
        output.push_str("_(none)_\n\n");
    } else {
        for heading in &summary.extraction.headings {
            output.push_str(&format!("- {heading}\n"));
        }
        output.push('\n');
    }
    output.push_str("## Emails\n\n");
    if summary.extraction.emails.is_empty() {
        output.push_str("_(none)_\n\n");
    } else {
        for email in &summary.extraction.emails {
            output.push_str(&format!("- {email}\n"));
        }
        output.push('\n');
    }
    output.push_str("## URLs\n\n");
    if summary.extraction.urls.is_empty() {
        output.push_str("_(none)_\n\n");
    } else {
        for url in &summary.extraction.urls {
            output.push_str(&format!("- {url}\n"));
        }
        output.push('\n');
    }
    output.push_str("## Dates\n\n");
    if summary.extraction.dates.is_empty() {
        output.push_str("_(none)_\n");
    } else {
        for date in &summary.extraction.dates {
            output.push_str(&format!("- {date}\n"));
        }
    }
    output
}

pub fn save_summary_file(
    pdf_path: &Path,
    summary: &PdfSummaryResult,
    overwrite: bool,
) -> Result<SummarySaveResult, String> {
    let target = summary_markdown_path(pdf_path);
    if target.exists() && !overwrite {
        return Ok(SummarySaveResult {
            summary: summary.clone(),
            summary_path: target.to_string_lossy().into_owned(),
            written: false,
            conflict: true,
        });
    }
    fs::write(&target, summary_to_markdown(summary)).map_err(|e| e.to_string())?;
    Ok(SummarySaveResult {
        summary: summary.clone(),
        summary_path: target.to_string_lossy().into_owned(),
        written: true,
        conflict: false,
    })
}

pub(crate) fn strip_markdown_for_summary_page(markdown: &str) -> String {
    strip_markdown_for_summary(markdown)
}
