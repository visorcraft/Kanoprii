use crate::pdf::coords::pdf_rect_to_viewer_px;
use crate::pdf::edit_types::RgbColor;
use pdfium_render::prelude::*;
use std::path::Path;

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageTextRun {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub font_family: String,
    pub font_size: f64,
    pub bold: bool,
    pub italic: bool,
    pub color: RgbColor,
}

pub fn group_chars_into_runs(chars: Vec<(char, [f64; 4], f64, String, RgbColor)>) -> Vec<PageTextRun> {
    if chars.is_empty() {
        return Vec::new();
    }

    let advances: Vec<f64> = chars.iter().map(|(_, b, _, _, _)| (b[2] - b[0]).max(0.1)).filter(|w| *w > 0.0).collect();
    let mut sorted = advances.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_advance = sorted.get(sorted.len() / 2).copied().unwrap_or(4.0).max(1.0);
    let gap_threshold = median_advance * 1.5;

    let mut runs: Vec<PageTextRun> = Vec::new();
    let mut run_text = String::new();
    let mut run_bounds: Option<[f64; 4]> = None;
    let mut run_font_size = 12.0;
    let mut run_font_name = String::new();
    let mut run_color = RgbColor { r: 0.0, g: 0.0, b: 0.0 };
    let mut prev: Option<(char, [f64; 4])> = None;

    let flush = |text: &mut String,
                 bounds: &mut Option<[f64; 4]>,
                 font_size: f64,
                 font_name: &str,
                 color: &RgbColor,
                 runs: &mut Vec<PageTextRun>| {
        if text.is_empty() {
            return;
        }
        let Some([left, top, right, bottom]) = *bounds else {
            text.clear();
            return;
        };
        let w = (right - left).max(1.0);
        let h = (bottom - top).max(1.0);
        let (font_family, bold, italic) = crate::pdf::fonts::editable_font_style(font_name);
        runs.push(PageTextRun {
            text: std::mem::take(text),
            x: left,
            y: top,
            w,
            h,
            font_family: font_family.to_string(),
            font_size,
            bold,
            italic,
            color: color.clone(),
        });
        *bounds = None;
    };

    for (ch, bounds, font_size, font_name, color) in chars {
        let style_changed = !run_text.is_empty()
            && ((font_size - run_font_size).abs() > 0.1 || font_name != run_font_name || color != run_color);
        if let Some((_, prev_bounds)) = prev {
            let baseline_jump = (bounds[1] - prev_bounds[1]).abs();
            let char_h = (prev_bounds[3] - prev_bounds[1]).max((bounds[3] - bounds[1]).max(1.0));
            let horizontal_gap = bounds[0] - prev_bounds[2];
            if style_changed || baseline_jump > char_h * 0.4 || horizontal_gap > gap_threshold {
                flush(&mut run_text, &mut run_bounds, run_font_size, &run_font_name, &run_color, &mut runs);
            }
        }

        if run_text.is_empty() {
            run_font_size = font_size;
            run_font_name = font_name;
            run_color = color;
        }

        run_text.push(ch);
        run_bounds = Some(match run_bounds {
            None => bounds,
            Some([l, t, r, b]) => [l.min(bounds[0]), t.min(bounds[1]), r.max(bounds[2]), b.max(bounds[3])],
        });
        prev = Some((ch, bounds));
    }
    flush(&mut run_text, &mut run_bounds, run_font_size, &run_font_name, &run_color, &mut runs);
    runs
}

pub fn get_page_text_layout(pdfium: &Pdfium, path: &Path, page_index: u32) -> Result<Vec<PageTextRun>, String> {
    if !path.is_file() {
        return Err("File not found".to_string());
    }
    let document = pdfium.load_pdf_from_file(path, None).map_err(|e| e.to_string())?;
    if page_index > i32::MAX as u32 {
        return Err("Page index out of range".to_string());
    }
    let page = document.pages().get(page_index as i32).map_err(|e| e.to_string())?;
    let page_w = page.width().value;
    let page_h = page.height().value;
    let text = page.text().map_err(|e| e.to_string())?;

    let mut chars = Vec::new();
    for text_char in text.chars().iter() {
        let Some(ch) = text_char.unicode_char() else {
            continue;
        };
        if ch.is_control() {
            continue;
        }
        let bounds = text_char.loose_bounds().or_else(|_| text_char.tight_bounds());
        let Ok(bounds) = bounds else {
            continue;
        };
        let left = f64::from(bounds.left().value);
        let right = f64::from(bounds.right().value);
        let bottom = f64::from(bounds.bottom().value);
        let top = f64::from(bounds.top().value);
        if right <= left {
            continue;
        }
        let viewer = pdf_rect_to_viewer_px(left, bottom, right, top, page_w, page_h);
        let color = text_char.fill_color().unwrap_or(PdfColor::BLACK);
        chars.push((
            ch,
            viewer,
            f64::from(text_char.scaled_font_size().value),
            text_char.font_name(),
            RgbColor { r: f64::from(color.red()), g: f64::from(color.green()), b: f64::from(color.blue()) },
        ));
    }

    Ok(group_chars_into_runs(chars))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn black() -> RgbColor {
        RgbColor { r: 0.0, g: 0.0, b: 0.0 }
    }

    #[test]
    fn group_chars_empty_input() {
        assert!(group_chars_into_runs(vec![]).is_empty());
    }

    #[test]
    fn group_chars_one_baseline_merges() {
        let chars = vec![
            ('H', [10.0, 20.0, 18.0, 32.0], 12.0, "Helvetica".into(), black()),
            ('i', [19.0, 20.0, 24.0, 32.0], 12.0, "Helvetica".into(), black()),
        ];
        let runs = group_chars_into_runs(chars);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "Hi");
    }

    #[test]
    fn group_chars_baseline_jump_splits() {
        let chars = vec![
            ('A', [10.0, 20.0, 20.0, 40.0], 12.0, "Helvetica".into(), black()),
            ('B', [22.0, 80.0, 32.0, 100.0], 12.0, "Helvetica".into(), black()),
        ];
        let runs = group_chars_into_runs(chars);
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn group_chars_large_gap_splits() {
        let chars = vec![
            ('a', [10.0, 20.0, 20.0, 32.0], 12.0, "Helvetica".into(), black()),
            ('b', [80.0, 20.0, 90.0, 32.0], 12.0, "Helvetica".into(), black()),
        ];
        let runs = group_chars_into_runs(chars);
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn group_chars_style_change_splits_and_reports_style() {
        let chars = vec![
            ('A', [10.0, 20.0, 20.0, 32.0], 12.0, "Helvetica".into(), black()),
            ('B', [21.0, 20.0, 31.0, 32.0], 14.0, "Times-BoldItalic".into(), RgbColor { r: 255.0, g: 0.0, b: 0.0 }),
        ];
        let runs = group_chars_into_runs(chars);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[1].font_family, "Times");
        assert_eq!(runs[1].font_size, 14.0);
        assert!(runs[1].bold);
        assert!(runs[1].italic);
        assert_eq!(runs[1].color.r, 255.0);
    }
}
