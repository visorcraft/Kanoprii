use crate::pdf::text_lines::TextLine;

pub struct Paragraph {
    pub line_indices: Vec<usize>,
    pub bbox: [f64; 4], // left, bottom, right, top in PDF user space
}

const MAX_ALIGN_DELTA_PTS: f64 = 3.0;
const MIN_LEADING_RATIO: f64 = 0.8;
const MAX_LEADING_RATIO: f64 = 1.6;

pub fn group_lines_into_paragraphs(lines: &[TextLine]) -> Vec<Paragraph> {
    let mut sorted: Vec<(usize, &TextLine)> = lines.iter().enumerate().collect();
    sorted.sort_by(|a, b| b.1.transform[5].total_cmp(&a.1.transform[5]));

    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (idx, line) in sorted {
        let mut placed = false;
        for group in groups.iter_mut() {
            let last_idx = *group.last().unwrap();
            let last = &lines[last_idx];
            if lines_match_paragraph(last, line) {
                group.push(idx);
                placed = true;
                break;
            }
        }
        if !placed {
            groups.push(vec![idx]);
        }
    }

    groups
        .into_iter()
        .map(|indices| {
            let mut left = f64::MAX;
            let mut bottom = f64::MAX;
            let mut right = f64::MIN;
            let mut top = f64::MIN;
            for i in &indices {
                let [l, b, r, t] = lines[*i].bbox;
                left = left.min(l);
                bottom = bottom.min(b);
                right = right.max(r);
                top = top.max(t);
            }
            Paragraph { line_indices: indices, bbox: [left, bottom, right, top] }
        })
        .collect()
}

fn lines_match_paragraph(prev: &TextLine, next: &TextLine) -> bool {
    if prev.font_name != next.font_name {
        return false;
    }
    if (prev.font_size - next.font_size).abs() > 0.5 {
        return false;
    }
    let left_delta = (prev.transform[4] - next.transform[4]).abs();
    let prev_width = prev.bbox[2] - prev.bbox[0];
    let next_width = next.bbox[2] - next.bbox[0];
    let right_delta = ((prev.transform[4] + prev_width) - (next.transform[4] + next_width)).abs();
    let aligned = left_delta <= MAX_ALIGN_DELTA_PTS || right_delta <= MAX_ALIGN_DELTA_PTS;
    if !aligned {
        return false;
    }
    let leading = (prev.transform[5] - next.transform[5]).abs();
    let min_leading = prev.font_size * MIN_LEADING_RATIO;
    let max_leading = prev.font_size * MAX_LEADING_RATIO;
    leading >= min_leading && leading <= max_leading
}

pub fn find_paragraph_for_line(lines: &[TextLine], line_index: usize) -> Option<Paragraph> {
    let paragraphs = group_lines_into_paragraphs(lines);
    paragraphs.into_iter().find(|p| p.line_indices.contains(&line_index))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(x: f64, y: f64, font_size: f64, text: &str) -> TextLine {
        let width = text.len() as f64 * font_size * 0.5;
        TextLine {
            text: text.to_string(),
            transform: [1.0, 0.0, 0.0, 1.0, x, y],
            font_name: "F1".to_string(),
            font_size,
            bbox: [x, y - font_size * 0.2, x + width, y + font_size * 0.8],
        }
    }

    #[test]
    fn groups_two_left_aligned_lines() {
        let lines = vec![line(100.0, 700.0, 12.0, "First line"), line(100.0, 686.0, 12.0, "Second line")];
        let paragraphs = group_lines_into_paragraphs(&lines);
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].line_indices.len(), 2);
    }

    #[test]
    fn split_lines_with_mismatched_x() {
        let lines = vec![line(100.0, 700.0, 12.0, "First"), line(150.0, 686.0, 12.0, "Second")];
        let paragraphs = group_lines_into_paragraphs(&lines);
        assert_eq!(paragraphs.len(), 2);
    }

    #[test]
    fn split_lines_with_large_leading() {
        let lines = vec![line(100.0, 700.0, 12.0, "First"), line(100.0, 600.0, 12.0, "Second")];
        let paragraphs = group_lines_into_paragraphs(&lines);
        assert_eq!(paragraphs.len(), 2);
    }

    #[test]
    fn finds_paragraph_for_line_index() {
        let lines = vec![line(100.0, 700.0, 12.0, "First"), line(100.0, 686.0, 12.0, "Second")];
        let paragraph = find_paragraph_for_line(&lines, 1).unwrap();
        assert_eq!(paragraph.line_indices, vec![0, 1]);
    }
}
