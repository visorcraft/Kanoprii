use crate::pdf::markdown_heuristic::{markdown_table, normalize_inline_text};
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
struct TaggedBlock {
    kind: String,
    text: String,
    link_uri: Option<String>,
    children: Vec<TaggedBlock>,
}

fn decode_pdf_string(obj: &Object) -> Option<String> {
    match obj {
        Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).into_owned()),
        Object::Name(name) => Some(String::from_utf8_lossy(name).into_owned()),
        _ => None,
    }
}

fn struct_tree_root_id(doc: &Document) -> Result<ObjectId, String> {
    let catalog = doc.catalog().map_err(|e| e.to_string())?;
    catalog
        .get(b"StructTreeRoot")
        .map_err(|_| "missing StructTreeRoot".to_string())?
        .as_reference()
        .map_err(|_| "bad StructTreeRoot".to_string())
}

fn page_index_for_struct(doc: &Document, dict: &Dictionary) -> Option<u32> {
    let page_ref = dict.get(b"Pg").ok()?.as_reference().ok()?;
    doc.get_pages().iter().find_map(|(num, id)| (*id == page_ref).then_some(num.saturating_sub(1)))
}

fn struct_link_uri(doc: &Document, dict: &Dictionary) -> Option<String> {
    let annot_id = match dict.get(b"K").ok()? {
        Object::Reference(id) => Some(*id),
        Object::Dictionary(entry) => {
            if entry.get(b"Type").ok().and_then(|o| o.as_name().ok()) != Some(b"OBJR") {
                return None;
            }
            entry.get(b"Obj").ok()?.as_reference().ok()
        }
        Object::Array(items) => items.iter().find_map(|item| {
            let Object::Dictionary(entry) = item else {
                return None;
            };
            if entry.get(b"Type").ok().and_then(|o| o.as_name().ok()) != Some(b"OBJR") {
                return None;
            }
            entry.get(b"Obj").ok()?.as_reference().ok()
        }),
        _ => None,
    }?;
    let annot = doc.get_dictionary(annot_id).ok()?;
    if annot.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) != Some(b"Link") {
        return None;
    }
    let a = annot.get(b"A").ok()?.as_dict().ok()?;
    if a.get(b"S").ok().and_then(|o| o.as_name().ok()) != Some(b"URI") {
        return None;
    }
    decode_pdf_string(a.get(b"URI").ok()?)
}

fn struct_element_text(dict: &Dictionary) -> String {
    for key in [b"T".as_slice(), b"ActualText", b"Alt", b"E"] {
        if let Ok(obj) = dict.get(key) {
            if let Some(raw) = decode_pdf_string(obj) {
                let text = normalize_inline_text(&raw);
                if !text.is_empty() {
                    return text;
                }
            }
        }
    }
    String::new()
}

fn struct_k_ids(k: &Object) -> Vec<ObjectId> {
    match k {
        Object::Reference(id) => vec![*id],
        Object::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Object::Reference(id) => Some(*id),
                Object::Dictionary(dict) => dict.get(b"Obj").ok().and_then(|obj| obj.as_reference().ok()),
                _ => None,
            })
            .collect(),
        Object::Dictionary(dict) => {
            dict.get(b"Obj").ok().and_then(|obj| obj.as_reference().ok()).map(|id| vec![id]).unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

fn parse_struct_element(
    doc: &Document,
    id: ObjectId,
    inherited_page: Option<u32>,
) -> Result<(Option<u32>, TaggedBlock), String> {
    let dict = doc.get_dictionary(id).map_err(|e| e.to_string())?;
    let page = page_index_for_struct(doc, dict).or(inherited_page);
    let kind = dict
        .get(b"S")
        .ok()
        .and_then(|obj| obj.as_name().ok())
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .unwrap_or_else(|| "Span".to_string());
    let text = struct_element_text(dict);
    let link_uri = if kind == "Link" { struct_link_uri(doc, dict) } else { None };
    let mut children = Vec::new();
    if let Ok(k) = dict.get(b"K") {
        for child_id in struct_k_ids(k) {
            let (_, child) = parse_struct_element(doc, child_id, page)?;
            children.push(child);
        }
    }
    Ok((page, TaggedBlock { kind, text, link_uri, children }))
}

fn is_struct_artifact(kind: &str) -> bool {
    matches!(kind, "Artifact" | "Pagination" | "Layout" | "PageHeader" | "PageFooter" | "Background" | "BatesNumber")
}

fn is_struct_container(kind: &str) -> bool {
    matches!(
        kind,
        "Document"
            | "Part"
            | "Art"
            | "Sect"
            | "Div"
            | "NonStruct"
            | "Private"
            | "Span"
            | "Form"
            | "DocumentFragment"
            | "Aside"
            | "Index"
            | "Reference"
            | "Annot"
    )
}

fn infer_page_from_block(block: &TaggedBlock) -> Option<u32> {
    for child in &block.children {
        if let Some(page) = infer_page_from_block(child) {
            return Some(page);
        }
    }
    None
}

fn block_has_content(block: &TaggedBlock) -> bool {
    !block.text.is_empty() || block.children.iter().any(block_has_content)
}

fn distribute_tagged_block(block: TaggedBlock, page: Option<u32>, out: &mut BTreeMap<u32, Vec<TaggedBlock>>) {
    if is_struct_artifact(block.kind.as_str()) {
        return;
    }
    let block_page = page.or_else(|| infer_page_from_block(&block));
    if is_struct_container(&block.kind) && block.text.is_empty() {
        if let Some(p) = block_page {
            for child in block.children {
                distribute_tagged_block(child, Some(p), out);
            }
        } else {
            for child in block.children {
                distribute_tagged_block(child, None, out);
            }
        }
        return;
    }
    if let Some(p) = block_page {
        out.entry(p).or_default().push(block);
    } else {
        for child in block.children {
            distribute_tagged_block(child, None, out);
        }
    }
}

fn tagged_heading_level(kind: &str) -> Option<usize> {
    match kind {
        "H" | "H1" | "Title" => Some(1),
        "H2" => Some(2),
        "H3" => Some(3),
        "H4" => Some(4),
        "H5" => Some(5),
        "H6" => Some(6),
        "H7" => Some(6),
        "H8" | "H9" | "H10" => Some(6),
        _ => None,
    }
}

const TAGGED_INLINE_KINDS: &[&str] =
    &["Span", "Em", "Strong", "Code", "Link", "Sub", "Lbl", "LBody", "Reference", "Underline", "Span"];

fn tagged_join_inline(block: &TaggedBlock) -> String {
    let mut parts = Vec::new();
    if !block.text.is_empty() {
        parts.push(block.text.clone());
    }
    for child in &block.children {
        if TAGGED_INLINE_KINDS.contains(&child.kind.as_str()) || child.link_uri.is_some() {
            let child_text = tagged_format_inline(child);
            if !child_text.is_empty() {
                parts.push(child_text);
            }
        }
    }
    normalize_inline_text(&parts.join(" "))
}

fn tagged_format_inline(block: &TaggedBlock) -> String {
    let inner = if block.children.is_empty() && !block.text.is_empty() {
        block.text.clone()
    } else {
        tagged_join_inline(block)
    };
    if inner.is_empty() {
        return String::new();
    }
    match block.kind.as_str() {
        "Strong" => format!("**{inner}**"),
        "Em" => format!("*{inner}*"),
        "Code" => format!("`{inner}`"),
        "Link" => block.link_uri.as_ref().map(|uri| format!("[{inner}]({uri})")).unwrap_or(inner),
        _ => inner,
    }
}

fn tagged_block_text(block: &TaggedBlock) -> String {
    if matches!(block.kind.as_str(), "Link" | "Strong" | "Em" | "Code") {
        tagged_format_inline(block)
    } else {
        tagged_join_inline(block)
    }
}

fn tagged_codeblock_text(block: &TaggedBlock) -> String {
    if !block.text.is_empty() {
        return block.text.clone();
    }
    block.children.iter().map(tagged_block_text).filter(|line| !line.is_empty()).collect::<Vec<_>>().join("\n")
}

fn tagged_list_label(block: &TaggedBlock) -> Option<String> {
    for child in &block.children {
        if child.kind == "Lbl" {
            let label = tagged_join_inline(child);
            if !label.is_empty() {
                return Some(label);
            }
        }
    }
    None
}

fn tagged_list_item_text(block: &TaggedBlock) -> String {
    if !block.text.is_empty() {
        return block.text.clone();
    }
    for child in &block.children {
        if matches!(child.kind.as_str(), "LBody" | "Lbl") {
            if child.kind == "Lbl" {
                continue;
            }
            let text = tagged_block_text(child);
            if !text.is_empty() {
                return text;
            }
        }
    }
    tagged_block_text(block)
}

fn is_ordered_list_label(label: &str) -> bool {
    let trimmed = label.trim();
    trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit()) && (trimmed.contains('.') || trimmed.contains(')'))
}

fn ordered_list_prefix(label: &str) -> Option<String> {
    let digits: String = label.trim().chars().take_while(|ch| ch.is_ascii_digit()).collect();
    digits.parse::<usize>().ok().map(|index| format!("{index}."))
}

fn tagged_table_row_cells(row: &TaggedBlock) -> Vec<String> {
    row.children
        .iter()
        .filter(|cell| {
            matches!(
                cell.kind.as_str(),
                "TD" | "TH" | "TableDataCell" | "TableHeaderCell" | "TableHeader" | "TableData"
            )
        })
        .map(tagged_block_text)
        .filter(|cell| !cell.is_empty())
        .collect()
}

fn tagged_table_row_is_header(row: &TaggedBlock) -> bool {
    row.children.iter().any(|cell| matches!(cell.kind.as_str(), "TH" | "TableHeaderCell" | "TableHeader"))
}

fn collect_tagged_table_rows(children: &[TaggedBlock]) -> Vec<(bool, Vec<String>)> {
    let mut rows = Vec::new();
    for child in children {
        match child.kind.as_str() {
            "TR" | "TableRow" => {
                let cells = tagged_table_row_cells(child);
                if !cells.is_empty() {
                    rows.push((tagged_table_row_is_header(child), cells));
                }
            }
            "THead" | "TBody" | "TFoot" | "TableHeaderGroup" | "TableBodyGroup" | "TableFooterGroup" | "Table" => {
                rows.extend(collect_tagged_table_rows(&child.children));
            }
            "Caption" | "Figure" | "Image" => {}
            _ if is_struct_container(child.kind.as_str()) => rows.extend(collect_tagged_table_rows(&child.children)),
            _ => {}
        }
    }
    rows
}

fn format_tagged_table(children: &[TaggedBlock]) -> String {
    let rows = collect_tagged_table_rows(children);
    if rows.is_empty() {
        return String::new();
    }
    if rows.len() == 1 {
        let headers: Vec<String> = (0..rows[0].1.len()).map(|index| format!("Column {}", index + 1)).collect();
        return markdown_table(&headers, &[rows[0].1.clone()]);
    }
    let header_index = rows.iter().position(|(is_header, _)| *is_header).unwrap_or(0);
    let headers = rows[header_index].1.clone();
    let body_rows: Vec<Vec<String>> = rows
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != header_index)
        .map(|(_, (_, cells))| cells.clone())
        .collect();
    markdown_table(&headers, &body_rows)
}

fn format_tagged_block(block: &TaggedBlock, list_depth: usize, out: &mut String) {
    let kind = block.kind.as_str();
    if is_struct_artifact(kind) {
        return;
    }
    if let Some(level) = tagged_heading_level(kind) {
        let text = tagged_block_text(block);
        if !text.is_empty() {
            out.push_str(&format!("{} {}\n\n", "#".repeat(level), text));
        }
        return;
    }

    match kind {
        "P" | "Paragraph" => {
            let text = tagged_block_text(block);
            if !text.is_empty() {
                out.push_str(&format!("{text}\n\n"));
            }
        }
        "L" | "List" => {
            for child in &block.children {
                format_tagged_block(child, list_depth, out);
            }
        }
        "LI" | "ListItem" => {
            let text = tagged_list_item_text(block);
            if !text.is_empty() {
                let indent = "  ".repeat(list_depth);
                let label = tagged_list_label(block);
                if label.as_deref().is_some_and(is_ordered_list_label) {
                    let prefix = label.as_deref().and_then(ordered_list_prefix).unwrap_or_else(|| "1.".to_string());
                    out.push_str(&format!("{indent}{prefix} {text}\n"));
                } else {
                    out.push_str(&format!("{indent}- {text}\n"));
                }
            }
            for child in &block.children {
                if matches!(child.kind.as_str(), "L" | "List") || !matches!(child.kind.as_str(), "LBody" | "Lbl") {
                    format_tagged_block(child, list_depth + 1, out);
                }
            }
            if list_depth == 0 {
                out.push('\n');
            }
        }
        "Table" => {
            let table = format_tagged_table(&block.children);
            if !table.is_empty() {
                out.push_str(&table);
                out.push('\n');
            }
        }
        "BlockQuote" | "Quote" => {
            let text = tagged_block_text(block);
            if !text.is_empty() {
                for line in text.lines() {
                    out.push_str(&format!("> {line}\n"));
                }
                out.push('\n');
            }
        }
        "Caption" => {
            let text = tagged_block_text(block);
            if !text.is_empty() {
                out.push_str(&format!("*{text}*\n\n"));
            }
        }
        "Figure" | "Image" | "Illustration" => {
            let alt = tagged_block_text(block);
            let caption =
                block.children.iter().find(|child| child.kind == "Caption").map(tagged_block_text).unwrap_or_default();
            let label = if !caption.is_empty() {
                caption.clone()
            } else if !alt.is_empty() {
                alt.clone()
            } else {
                "Figure".to_string()
            };
            out.push_str(&format!("![{label}]({label})\n\n"));
            if !caption.is_empty() && caption != alt {
                out.push_str(&format!("*{caption}*\n\n"));
            }
        }
        "Code" => {
            let text = tagged_codeblock_text(block);
            if !text.is_empty() {
                out.push_str(&format!("```\n{text}\n```\n\n"));
            }
        }
        "Subtitle" => {
            let text = tagged_block_text(block);
            if !text.is_empty() {
                out.push_str(&format!("### {text}\n\n"));
            }
        }
        "Note" | "Footnote" => {
            let text = tagged_block_text(block);
            if !text.is_empty() {
                out.push_str(&format!("> **Note:** {text}\n\n"));
            }
        }
        "TOC" => {
            for child in &block.children {
                format_tagged_block(child, list_depth, out);
            }
            if list_depth == 0 {
                out.push('\n');
            }
        }
        "TOCI" => {
            let text = tagged_block_text(block);
            if !text.is_empty() {
                let indent = "  ".repeat(list_depth);
                out.push_str(&format!("{indent}- {text}\n"));
            }
            for child in &block.children {
                if matches!(child.kind.as_str(), "TOC" | "TOCI") {
                    format_tagged_block(child, list_depth + 1, out);
                }
            }
        }
        "Formula" => {
            let text = tagged_block_text(block);
            if !text.is_empty() {
                out.push_str(&format!("`${text}`\n\n"));
            }
        }
        "BibEntry" => {
            let text = tagged_block_text(block);
            if !text.is_empty() {
                out.push_str(&format!("- {text}\n"));
            }
        }
        _ if is_struct_container(kind) => {
            for child in &block.children {
                format_tagged_block(child, list_depth, out);
            }
        }
        _ => {
            let text = tagged_block_text(block);
            if !text.is_empty() {
                out.push_str(&format!("{text}\n\n"));
            } else {
                for child in &block.children {
                    format_tagged_block(child, list_depth, out);
                }
            }
        }
    }
}

fn format_tagged_blocks(blocks: &[TaggedBlock]) -> String {
    let mut output = String::new();
    for block in blocks {
        format_tagged_block(block, 0, &mut output);
    }
    if output.trim().is_empty() {
        "_(no extractable text on this page)_\n\n".to_string()
    } else {
        output
    }
}

pub fn tagged_page_has_content(md: &str) -> bool {
    let trimmed = md.trim();
    !trimmed.is_empty() && trimmed != "_(no extractable text on this page)_"
}

/// When the PDF catalog carries `/StructTreeRoot`, map 0-based page indices to
/// Markdown derived from structure types (`/H1`, `/P`, `/L`, `/Table`, …).
pub fn tagged_markdown_by_page(doc: &Document) -> Option<BTreeMap<u32, String>> {
    let root_id = struct_tree_root_id(doc).ok()?;
    let root = doc.get_dictionary(root_id).ok()?;
    let k = root.get(b"K").ok()?;
    let mut page_blocks: BTreeMap<u32, Vec<TaggedBlock>> = BTreeMap::new();
    for child_id in struct_k_ids(k) {
        let (page, block) = parse_struct_element(doc, child_id, None).ok()?;
        distribute_tagged_block(block, page, &mut page_blocks);
    }
    if !page_blocks.values().any(|blocks| blocks.iter().any(block_has_content)) {
        return None;
    }
    Some(page_blocks.into_iter().map(|(page, blocks)| (page, format_tagged_blocks(&blocks))).collect())
}

pub fn plain_text_to_markdown(text: &str) -> String {
    let normalized = text.lines().map(str::trim).filter(|line| !line.is_empty()).collect::<Vec<_>>().join("\n");
    if normalized.is_empty() {
        "_(no extractable text on this page)_\n\n".to_string()
    } else {
        format!("{normalized}\n\n")
    }
}
