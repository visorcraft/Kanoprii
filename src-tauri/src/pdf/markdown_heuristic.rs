use pdfium_render::prelude::*;

#[derive(Clone, Debug)]
pub struct MarkdownTextCell {
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct MarkdownTextLine {
    pub cells: Vec<MarkdownTextCell>,
    pub text: String,
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub height: f32,
}

#[derive(Clone, Debug)]
struct MarkdownGlyph {
    ch: char,
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    height: f32,
}

impl MarkdownGlyph {
    fn center_y(&self) -> f32 {
        (self.top + self.bottom) / 2.0
    }

    fn width(&self) -> f32 {
        (self.right - self.left).max(0.1)
    }
}

#[derive(Clone, Debug)]
struct MarkdownGlyphLine {
    glyphs: Vec<MarkdownGlyph>,
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    height: f32,
}

impl MarkdownGlyphLine {
    fn new(glyph: MarkdownGlyph) -> Self {
        Self {
            left: glyph.left,
            right: glyph.right,
            bottom: glyph.bottom,
            top: glyph.top,
            height: glyph.height,
            glyphs: vec![glyph],
        }
    }

    fn center_y(&self) -> f32 {
        (self.top + self.bottom) / 2.0
    }

    fn push(&mut self, glyph: MarkdownGlyph) {
        self.left = self.left.min(glyph.left);
        self.right = self.right.max(glyph.right);
        self.bottom = self.bottom.min(glyph.bottom);
        self.top = self.top.max(glyph.top);
        self.height = self.height.max(glyph.height);
        self.glyphs.push(glyph);
    }
}

pub fn normalize_inline_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn median_glyph_width(glyphs: &[MarkdownGlyph]) -> f32 {
    let mut widths = glyphs.iter().map(MarkdownGlyph::width).filter(|width| *width > 0.0).collect::<Vec<_>>();
    if widths.is_empty() {
        return 4.0;
    }
    widths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    widths[widths.len() / 2].max(1.0)
}

fn text_line_from_glyph_line(mut line: MarkdownGlyphLine) -> Option<MarkdownTextLine> {
    line.glyphs.sort_by(|a, b| a.left.partial_cmp(&b.left).unwrap_or(std::cmp::Ordering::Equal));
    let average_width = median_glyph_width(&line.glyphs);
    let word_gap = (average_width * 1.15).max(2.0);
    let cell_gap = (line.height * 2.6).max(average_width * 4.0).max(18.0);
    let mut cells: Vec<MarkdownTextCell> = Vec::new();
    let mut current_text = String::new();
    let mut previous_right: Option<f32> = None;

    for glyph in line.glyphs {
        let gap = previous_right.map(|right| glyph.left - right).unwrap_or(0.0);
        if gap > cell_gap && !current_text.trim().is_empty() {
            cells.push(MarkdownTextCell { text: normalize_inline_text(&current_text) });
            current_text.clear();
        }

        if glyph.ch.is_whitespace() {
            if !current_text.is_empty() && !current_text.ends_with(' ') {
                current_text.push(' ');
            }
            previous_right = Some(glyph.right);
            continue;
        }

        if gap > word_gap && !current_text.is_empty() && !current_text.ends_with(' ') {
            current_text.push(' ');
        }
        current_text.push(glyph.ch);
        previous_right = Some(glyph.right);
    }

    if !current_text.trim().is_empty() {
        cells.push(MarkdownTextCell { text: normalize_inline_text(&current_text) });
    }

    if cells.is_empty() {
        return None;
    }

    let text = cells.iter().map(|cell| cell.text.as_str()).collect::<Vec<_>>().join(" ");
    Some(MarkdownTextLine {
        cells,
        text,
        left: line.left,
        right: line.right,
        bottom: line.bottom,
        top: line.top,
        height: line.height,
    })
}

/// Glyphs whose raw character code *might* be a decorative bullet from a symbol
/// font. Cheap pre-check so we only pay the per-glyph `font_name()` FFI cost for
/// plausible candidates instead of for every character on the page.
pub fn is_symbol_glyph_candidate(ch: char) -> bool {
    let code = ch as u32;
    // Some PDFs map symbol-font glyphs into the Private Use Area (0xF000 + code).
    let base = if (0xF000..=0xF0FF).contains(&code) { code - 0xF000 } else { code };
    (0x6C..=0x77).contains(&base) || base == 0xA7 || base == 0xB7
}

/// Office documents routinely draw list bullets with a Wingdings/Webdings glyph
/// (e.g. Wingdings `n` = ▪). PDF text extraction surfaces the raw glyph code, so
/// the bullet otherwise leaks into the Markdown as a stray letter. When the glyph
/// comes from a known dingbat font, translate the common shape glyphs to `•` so
/// the bullet detector and list formatter treat the line as a list item. Gated on
/// the font name, so ordinary text (e.g. the letter `n` in Arial) is untouched.
pub fn map_symbol_glyph(font_name: &str, ch: char) -> char {
    let font = font_name.to_ascii_lowercase();
    let is_dingbat = font.contains("wingding") || font.contains("webding") || font.contains("dingbat");
    let is_symbol = font.contains("symbol");
    if !is_dingbat && !is_symbol {
        return ch;
    }
    let code = ch as u32;
    let base = if (0xF000..=0xF0FF).contains(&code) { code - 0xF000 } else { code };
    // Geometric shapes (squares/circles/diamonds in 0x6C–0x77) are dingbat list
    // bullets; 0xA7/0xB7 are the small-square / middle-dot bullets the Symbol
    // font shares. Symbol-font letters are Greek glyphs, so never rewrite those.
    if is_dingbat && ((0x6C..=0x77).contains(&base) || base == 0xA7 || base == 0xB7) {
        return '•';
    }
    if is_symbol && (base == 0xA7 || base == 0xB7) {
        return '•';
    }
    ch
}

pub fn lines_from_pdfium_text(text: &PdfPageText<'_>) -> Vec<MarkdownTextLine> {
    let mut glyphs = Vec::new();

    for text_char in text.chars().iter() {
        let Some(mut ch) = text_char.unicode_char() else {
            continue;
        };
        if ch.is_control() {
            continue;
        }
        // Translate dingbat-font bullet glyphs (e.g. Wingdings `n` = ▪) to `•`.
        if is_symbol_glyph_candidate(ch) {
            ch = map_symbol_glyph(&text_char.font_name(), ch);
        }

        let bounds = text_char.loose_bounds().or_else(|_| text_char.tight_bounds());
        let Ok(bounds) = bounds else {
            continue;
        };

        let left = bounds.left().value;
        let right = bounds.right().value;
        let bottom = bounds.bottom().value;
        let top = bounds.top().value;
        let right = if right <= left && ch.is_whitespace() { left + 0.1 } else { right };
        let height = bounds.height().value.max(1.0);
        if !left.is_finite() || !right.is_finite() || !bottom.is_finite() || !top.is_finite() || right <= left {
            continue;
        }

        glyphs.push(MarkdownGlyph { ch, left, right, bottom, top, height });
    }

    glyphs.sort_by(|a, b| {
        b.center_y()
            .partial_cmp(&a.center_y())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.left.partial_cmp(&b.left).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut glyph_lines: Vec<MarkdownGlyphLine> = Vec::new();
    for glyph in glyphs {
        let maybe_line = glyph_lines.last_mut().filter(|line| {
            let tolerance = (line.height.max(glyph.height) * 0.65).max(2.0);
            (line.center_y() - glyph.center_y()).abs() <= tolerance
        });

        if let Some(line) = maybe_line {
            line.push(glyph);
        } else {
            glyph_lines.push(MarkdownGlyphLine::new(glyph));
        }
    }

    glyph_lines.into_iter().filter_map(text_line_from_glyph_line).collect()
}

fn median_line_height(lines: &[MarkdownTextLine]) -> f32 {
    let mut heights: Vec<f32> = lines.iter().map(|line| line.height).filter(|height| *height > 0.0).collect();
    if heights.is_empty() {
        return 12.0;
    }
    heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    heights[heights.len() / 2].max(1.0)
}

fn line_gap_after(lines: &[MarkdownTextLine], index: usize) -> f32 {
    lines.get(index + 1).map(|next| (lines[index].bottom - next.top).max(0.0)).unwrap_or(0.0)
}

fn line_gap_before(lines: &[MarkdownTextLine], index: usize) -> f32 {
    if index == 0 {
        return 0.0;
    }
    (lines[index - 1].bottom - lines[index].top).max(0.0)
}

fn page_width(lines: &[MarkdownTextLine]) -> f32 {
    let left = lines.iter().map(|line| line.left).fold(f32::INFINITY, f32::min);
    let right = lines.iter().map(|line| line.right).fold(f32::NEG_INFINITY, f32::max);
    if left.is_finite() && right.is_finite() {
        (right - left).max(1.0)
    } else {
        1.0
    }
}

fn line_center_y(line: &MarkdownTextLine) -> f32 {
    (line.top + line.bottom) / 2.0
}

#[derive(Clone, Debug)]
pub struct MarkdownPageLink {
    pub uri: String,
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
}

pub fn collect_page_links(page: &PdfPage<'_>) -> Vec<MarkdownPageLink> {
    let mut links = Vec::new();
    for link in page.links().iter() {
        let Ok(rect) = link.rect() else {
            continue;
        };
        let Some(action) = link.action() else {
            continue;
        };
        let Some(uri_action) = action.as_uri_action() else {
            continue;
        };
        let Ok(uri) = uri_action.uri() else {
            continue;
        };
        links.push(MarkdownPageLink {
            uri,
            left: rect.left().value,
            right: rect.right().value,
            bottom: rect.bottom().value,
            top: rect.top().value,
        });
    }
    links
}

fn line_overlaps_link(line: &MarkdownTextLine, link: &MarkdownPageLink) -> bool {
    line.left < link.right && line.right > link.left && line.bottom < link.top && line.top > link.bottom
}

fn trim_trailing_link_punctuation(token: &str) -> (&str, &str) {
    let mut end = token.len();
    while end > 0 {
        let ch = token[..end].chars().last().unwrap();
        if matches!(ch, '.' | ',' | ';' | ')' | ']' | '}' | '"' | '!') {
            end -= ch.len_utf8();
        } else {
            break;
        }
    }
    token.split_at(end)
}

fn autolink_token(token: &str) -> String {
    if token.contains("](") {
        return token.to_string();
    }
    let trimmed = token.trim_end();
    let trailing_ws = &token[trimmed.len()..];
    let (core, suffix) = trim_trailing_link_punctuation(trimmed);
    if core.starts_with("http://") || core.starts_with("https://") {
        return format!("[{core}]({core}){suffix}{trailing_ws}");
    }
    if core.contains('@') && core.contains('.') && !core.starts_with('@') {
        return format!("[{core}](mailto:{core}){suffix}{trailing_ws}");
    }
    token.to_string()
}

pub fn autolink_inline_text(text: &str) -> String {
    if text.contains("](") {
        return text.to_string();
    }
    text.split_inclusive(char::is_whitespace).map(autolink_token).collect::<Vec<_>>().join("")
}

pub fn apply_links_to_text(text: &str, line: &MarkdownTextLine, links: &[MarkdownPageLink]) -> String {
    let linked = links
        .iter()
        .find(|link| line_overlaps_link(line, link))
        .map(|link| {
            if text == link.uri || text.starts_with("http://") || text.starts_with("https://") {
                format!("[{text}]({text})")
            } else {
                format!("[{text}]({})", link.uri)
            }
        })
        .unwrap_or_else(|| text.to_string());
    autolink_inline_text(&linked)
}

fn sort_lines_vertical(lines: &mut [MarkdownTextLine]) {
    lines.sort_by(|a, b| {
        b.top
            .partial_cmp(&a.top)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.left.partial_cmp(&b.left).unwrap_or(std::cmp::Ordering::Equal))
    });
}

fn detect_column_split(lines: &[MarkdownTextLine], page_w: f32) -> Option<f32> {
    if lines.len() < 4 {
        return None;
    }
    let mut lefts: Vec<f32> = lines.iter().map(|line| line.left).collect();
    lefts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut best_gap = 0.0f32;
    let mut split = None;
    for window in lefts.windows(2) {
        let gap = window[1] - window[0];
        if gap > best_gap {
            best_gap = gap;
            split = Some((window[0] + window[1]) / 2.0);
        }
    }
    if best_gap > page_w * 0.15 {
        split
    } else {
        None
    }
}

pub fn sort_lines_reading_order(mut lines: Vec<MarkdownTextLine>) -> Vec<MarkdownTextLine> {
    if lines.len() < 2 {
        return lines;
    }
    let page_w = page_width(&lines);
    if let Some(split_x) = detect_column_split(&lines, page_w) {
        let mut left_col: Vec<MarkdownTextLine> =
            lines.iter().filter(|line| (line.left + line.right) / 2.0 < split_x).cloned().collect();
        let mut right_col: Vec<MarkdownTextLine> =
            lines.iter().filter(|line| (line.left + line.right) / 2.0 >= split_x).cloned().collect();
        if !left_col.is_empty() && !right_col.is_empty() {
            sort_lines_vertical(&mut left_col);
            sort_lines_vertical(&mut right_col);
            return left_col.into_iter().chain(right_col).collect();
        }
    }
    sort_lines_vertical(&mut lines);
    lines
}

fn is_probable_header_footer_text(text: &str) -> bool {
    if is_page_marker(text) {
        return true;
    }
    let trimmed = text.trim();
    if trimmed.len() > 80 {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("confidential")
        || lower.starts_with("draft")
        || (lower.contains("copyright") && trimmed.split_whitespace().count() <= 10)
}

fn is_header_footer_band_line(line: &MarkdownTextLine, min_bottom: f32, max_top: f32, page_height: f32) -> bool {
    let center = line_center_y(line);
    let in_header_band = center > max_top - page_height * 0.1;
    let in_footer_band = center < min_bottom + page_height * 0.1;
    (in_header_band || in_footer_band) && line.text.split_whitespace().count() <= 12
}

pub fn strip_header_footer_lines(lines: Vec<MarkdownTextLine>) -> Vec<MarkdownTextLine> {
    if lines.len() < 3 {
        return lines;
    }
    let min_bottom = lines.iter().map(|line| line.bottom).fold(f32::INFINITY, f32::min);
    let max_top = lines.iter().map(|line| line.top).fold(f32::NEG_INFINITY, f32::max);
    let page_height = (max_top - min_bottom).max(1.0);
    lines
        .into_iter()
        .filter(|line| {
            let text = line.text.trim();
            if text.is_empty() {
                return false;
            }
            if is_probable_header_footer_text(text) {
                return false;
            }
            !(is_header_footer_band_line(line, min_bottom, max_top, page_height) && is_page_marker(text))
        })
        .collect()
}

pub fn polish_heuristic_lines(lines: Vec<MarkdownTextLine>) -> Vec<MarkdownTextLine> {
    sort_lines_reading_order(strip_header_footer_lines(lines))
}

fn is_page_marker(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.len() <= 8
        && trimmed.chars().any(|ch| ch.is_ascii_digit())
        && trimmed.chars().all(|ch| ch.is_ascii_digit() || ch == '-' || ch.is_whitespace())
    {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("page ") && trimmed.len() < 28 && trimmed.chars().filter(|ch| ch.is_ascii_digit()).count() >= 1
    {
        return true;
    }
    if lower.contains(" of ") && trimmed.len() < 28 {
        let numeric_tokens =
            trimmed.split_whitespace().filter(|part| part.chars().all(|ch| ch.is_ascii_digit())).count();
        if numeric_tokens >= 2 {
            return true;
        }
    }
    false
}

fn is_bullet_line(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("• ")
        || trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit() && trimmed.contains(". "))
}

fn is_toc_title(text: &str) -> bool {
    text.trim().eq_ignore_ascii_case("table of contents") || text.trim().eq_ignore_ascii_case("contents")
}

fn trim_toc_leader(title: &str) -> String {
    let mut title = title.trim();
    loop {
        let trimmed = title.trim_end_matches('.').trim_end();
        if trimmed.len() == title.len() {
            break;
        }
        title = trimmed;
    }
    title.to_string()
}

fn parse_toc_entry(text: &str) -> Option<(String, String)> {
    let trimmed = text.trim();
    if trimmed.len() < 4 {
        return None;
    }

    if let Some(index) = trimmed.rfind("Page ") {
        let title = trim_toc_leader(&trimmed[..index]);
        let page = trimmed[index + 5..].trim();
        if !title.is_empty() && !page.is_empty() && page.chars().all(|ch| ch.is_ascii_digit()) {
            return Some((title, page.to_string()));
        }
    }

    let mut parts = trimmed.rsplitn(2, char::is_whitespace);
    let page = parts.next()?.trim();
    let title = parts.next()?.trim();
    if page.chars().all(|ch| ch.is_ascii_digit()) && title.contains("...") {
        let title = trim_toc_leader(title);
        if !title.is_empty() {
            return Some((title, page.to_string()));
        }
    }

    None
}

fn escape_table_cell(text: &str) -> String {
    normalize_inline_text(text).replace('\\', "\\\\").replace('|', "\\|")
}

pub fn markdown_table(headers: &[String], rows: &[Vec<String>]) -> String {
    let header = headers.iter().map(|cell| escape_table_cell(cell)).collect::<Vec<_>>().join(" | ");
    let separator = headers.iter().map(|_| "---").collect::<Vec<_>>().join(" | ");
    let mut output = format!("| {} |\n| {} |\n", header, separator);

    for row in rows {
        let cells = (0..headers.len())
            .map(|index| escape_table_cell(row.get(index).map(String::as_str).unwrap_or("")))
            .collect::<Vec<_>>()
            .join(" | ");
        output.push_str(&format!("| {} |\n", cells));
    }

    output
}

fn toc_table(rows: &[(String, String)]) -> String {
    let rows = rows.iter().map(|(title, page)| vec![title.clone(), page.clone()]).collect::<Vec<_>>();
    markdown_table(&["Section".to_string(), "Page".to_string()], &rows)
}

fn column_table_block(lines: &[MarkdownTextLine], start: usize) -> Option<(usize, String)> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut index = start;
    let mut expected_columns = 0;

    while let Some(line) = lines.get(index) {
        if line.cells.len() < 2 || line.cells.len() > 8 || is_page_marker(&line.text) {
            break;
        }
        if expected_columns == 0 {
            expected_columns = line.cells.len();
        }
        if line.cells.len().abs_diff(expected_columns) > 1 {
            break;
        }
        rows.push(line.cells.iter().map(|cell| cell.text.clone()).collect());
        index += 1;
    }

    if rows.len() < 2 {
        return None;
    }

    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    if column_count < 2 {
        return None;
    }

    let headers = rows.remove(0);
    let headers = (0..column_count)
        .map(|index| headers.get(index).cloned().unwrap_or_else(|| format!("Column {}", index + 1)))
        .collect::<Vec<_>>();

    Some((index - start, markdown_table(&headers, &rows)))
}

fn probable_heading_level(
    line: &MarkdownTextLine,
    body_height: f32,
    width: f32,
    gap_before: f32,
    gap_after: f32,
) -> Option<usize> {
    let text = line.text.trim();
    if text.is_empty()
        || is_page_marker(text)
        || is_bullet_line(text)
        || parse_toc_entry(text).is_some()
        || text.len() > 90
        || !text.chars().any(char::is_alphabetic)
        || (text.ends_with('-') && !text.ends_with("--"))
    {
        return None;
    }

    let words = text.split_whitespace().count();
    if words > 12 {
        return None;
    }

    let relative_height = line.height / body_height.max(1.0);
    let has_heading_spacing = gap_before > body_height * 0.75 || gap_after > body_height * 0.75;
    let first_line = gap_before <= f32::EPSILON;
    let starts_like_title = text.chars().next().is_some_and(|ch| ch.is_uppercase());
    let sentence_like = text.ends_with('.') && words > 4;
    let narrow = (line.right - line.left) < width * 0.75;
    let strong_heading = relative_height >= 1.45 || (!sentence_like && starts_like_title && first_line && words <= 10);

    if strong_heading {
        Some(3)
    } else if !sentence_like && (relative_height >= 1.2 || (starts_like_title && has_heading_spacing && narrow)) {
        Some(4)
    } else {
        None
    }
}

fn ends_sentence(text: &str) -> bool {
    text.ends_with('.') || text.ends_with('!') || text.ends_with('?') || text.ends_with(':') || text.ends_with(';')
}

pub fn merge_wrapped_line_pair(previous: &str, next: &str) -> String {
    let prev = previous.trim_end();
    let next = next.trim_start();
    if prev.is_empty() {
        return next.to_string();
    }
    if next.is_empty() {
        return prev.to_string();
    }
    if prev.ends_with('-') && !prev.ends_with("--") {
        let base = prev.trim_end_matches('-');
        if next.chars().next().is_some_and(|ch| ch.is_ascii_lowercase()) {
            return format!("{base}{next}");
        }
    }
    if !ends_sentence(prev) && next.chars().next().is_some_and(|ch| ch.is_ascii_lowercase()) {
        return format!("{prev} {next}");
    }
    format!("{prev} {next}")
}

fn merge_paragraph_lines(parts: &[String]) -> String {
    if parts.is_empty() {
        return String::new();
    }
    let mut merged = parts[0].clone();
    for part in parts.iter().skip(1) {
        merged = merge_wrapped_line_pair(&merged, part);
    }
    normalize_inline_text(&merged)
}

fn should_end_paragraph(text: &str, gap_after: f32, body_height: f32, next_line: Option<&MarkdownTextLine>) -> bool {
    if ends_sentence(text.trim()) {
        return true;
    }
    if gap_after > body_height * 0.9 {
        return true;
    }
    if let Some(next) = next_line {
        let next_text = next.text.trim();
        if gap_after < body_height * 0.55
            && !next_text.is_empty()
            && next_text.chars().next().is_some_and(|ch| ch.is_ascii_lowercase())
        {
            return false;
        }
    }
    false
}

fn flush_paragraph(
    output: &mut String,
    paragraph: &mut Vec<String>,
    links: &[MarkdownPageLink],
    line: &MarkdownTextLine,
) {
    if paragraph.is_empty() {
        return;
    }
    let merged = merge_paragraph_lines(paragraph);
    let merged = apply_links_to_text(&merged, line, links);
    output.push_str(&merged);
    output.push_str("\n\n");
    paragraph.clear();
}

pub fn format_markdown_lines(lines: &[MarkdownTextLine], links: &[MarkdownPageLink]) -> String {
    if lines.is_empty() {
        return "_(no extractable text on this page)_\n\n".to_string();
    }

    let body_height = median_line_height(lines);
    let width = page_width(lines);
    let mut output = String::new();
    let mut paragraph: Vec<String> = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = &lines[index];
        let text = line.text.trim();
        if text.is_empty() || is_page_marker(text) {
            index += 1;
            continue;
        }

        if is_toc_title(text) {
            flush_paragraph(&mut output, &mut paragraph, links, line);
            output.push_str("### Table of Contents\n\n");
            index += 1;

            let mut toc_rows = Vec::new();
            while let Some(line) = lines.get(index) {
                if let Some(row) = parse_toc_entry(&line.text) {
                    toc_rows.push(row);
                    index += 1;
                } else {
                    break;
                }
            }
            if !toc_rows.is_empty() {
                output.push_str(&toc_table(&toc_rows));
                output.push('\n');
            }
            continue;
        }

        if let Some((consumed, table)) = column_table_block(lines, index) {
            flush_paragraph(&mut output, &mut paragraph, links, line);
            output.push_str(&table);
            output.push('\n');
            index += consumed;
            continue;
        }

        if let Some((title, page)) = parse_toc_entry(text) {
            flush_paragraph(&mut output, &mut paragraph, links, line);
            output.push_str(&toc_table(&[(title, page)]));
            output.push('\n');
            index += 1;
            continue;
        }

        let gap_before = line_gap_before(lines, index);
        let gap_after = line_gap_after(lines, index);
        if let Some(level) = probable_heading_level(line, body_height, width, gap_before, gap_after) {
            flush_paragraph(&mut output, &mut paragraph, links, line);
            let heading = apply_links_to_text(text, line, links);
            output.push_str(&format!("{} {}\n\n", "#".repeat(level), heading));
            index += 1;
            continue;
        }

        if is_bullet_line(text) {
            flush_paragraph(&mut output, &mut paragraph, links, line);
            let bullet = text.trim_start_matches(['•', '*']).trim_start();
            let bullet = bullet.trim_start_matches("- ").trim();
            let bullet = apply_links_to_text(bullet, line, links);
            output.push_str(&format!("- {bullet}\n"));
            if gap_after > body_height * 0.8 {
                output.push('\n');
            }
            index += 1;
            continue;
        }

        paragraph.push(text.to_string());
        let next_line = lines.get(index + 1);
        if should_end_paragraph(text, gap_after, body_height, next_line) {
            flush_paragraph(&mut output, &mut paragraph, links, line);
        }
        index += 1;
    }

    if let Some(line) = lines.last() {
        flush_paragraph(&mut output, &mut paragraph, links, line);
    }
    if output.trim().is_empty() {
        "_(no extractable text on this page)_\n\n".to_string()
    } else {
        output
    }
}
