#!/usr/bin/env python3
"""Generate document-wide mod-3…mod-6 parity commands."""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INC = ROOT / "src-tauri" / "src" / "parity_docmod_generated.inc.rs"
MAIN = ROOT / "src-tauri" / "src" / "main.rs"
UI_JSON = ROOT / "src" / "parity_batch_commands.json"

MODULI = (3, 4, 5, 6)

# (kind, command template with {m} and {r})
DOC_OPS: list[tuple[str, str]] = [
    ("rotate_cw", "rotate_mod{m}_{r}_pages"),
    ("rotate_ccw", "rotate_mod{m}_{r}_pages_ccw"),
    ("rotate_180", "rotate_180_mod{m}_{r}_pages"),
    ("reset_rot", "reset_rotation_mod{m}_{r}_pages"),
    ("delete", "delete_mod{m}_{r}_pages"),
    ("keep", "keep_mod{m}_{r}_pages"),
    ("dup_append", "duplicate_mod{m}_{r}_pages"),
    ("flatten", "flatten_mod{m}_{r}_pages"),
    ("crop", "crop_mod{m}_{r}_pages"),
    ("expand", "expand_mod{m}_{r}_pages"),
    ("shrink", "shrink_mod{m}_{r}_pages"),
    ("reverse", "reverse_mod{m}_{r}_pages"),
    ("move_start", "move_mod{m}_{r}_pages_to_start"),
    ("move_end", "move_mod{m}_{r}_pages_to_end"),
    ("clear_crop", "clear_crop_mod{m}_{r}_pages"),
    ("dup_before", "duplicate_mod{m}_{r}_pages_before"),
    ("sort_rot", "sort_mod{m}_{r}_pages_by_rotation"),
    ("sort_size", "sort_mod{m}_{r}_pages_by_size"),
    ("page_numbers", "add_page_numbers_mod{m}_{r}_pages"),
    ("watermark", "add_text_watermark_mod{m}_{r}_pages"),
    ("header", "add_page_header_mod{m}_{r}_pages"),
    ("footer", "add_page_footer_mod{m}_{r}_pages"),
    ("border", "add_page_border_mod{m}_{r}_pages"),
    ("bookmark", "bookmark_mod{m}_{r}_pages"),
    ("page_size", "set_page_size_mod{m}_{r}_pages"),
    ("blank_before", "insert_blank_before_mod{m}_{r}_pages"),
    ("blank_after", "insert_blank_after_mod{m}_{r}_pages"),
    ("dup_to_end", "duplicate_mod{m}_{r}_pages_to_end"),
    ("dup_to_start", "duplicate_mod{m}_{r}_pages_to_start"),
    ("extract", "extract_mod{m}_{r}_pages"),
    ("export_pdf", "export_mod{m}_{r}_pages_as_pdf"),
    ("export_png", "export_mod{m}_{r}_pages_png"),
    ("export_jpeg", "export_mod{m}_{r}_pages_jpeg"),
    ("export_webp", "export_mod{m}_{r}_pages_webp"),
    ("export_bmp", "export_mod{m}_{r}_pages_bmp"),
    ("export_tiff", "export_mod{m}_{r}_pages_tiff"),
    ("export_gif", "export_mod{m}_{r}_pages_gif"),
    ("export_ppm", "export_mod{m}_{r}_pages_ppm"),
    ("export_tga", "export_mod{m}_{r}_pages_tga"),
    ("export_ico", "export_mod{m}_{r}_pages_ico"),
]


def rust_helpers() -> str:
    return r"""
fn parity_docmod_match(page_index: u32, modulus: u32, remainder: u32) -> bool {
    page_index % modulus == remainder
}

fn parity_docmod_indices(total: u32, modulus: u32, remainder: u32) -> Vec<u32> {
    (0..total).filter(|i| parity_docmod_match(*i, modulus, remainder)).collect()
}

fn parity_docmod_rotate(path: &Path, modulus: u32, remainder: u32, delta: i64) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut rotated = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let current = page_rotation(&doc, page_id);
        set_page_rotation(&mut doc, page_id, current + delta)?;
        rotated += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(rotated)
}

fn parity_docmod_reset_rotation(path: &Path, modulus: u32, remainder: u32) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut reset = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        set_page_rotation(&mut doc, page_id, 0)?;
        reset += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(reset)
}

fn parity_docmod_delete(path: &Path, modulus: u32, remainder: u32) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let total = kids.len();
    let mut to_delete: Vec<usize> = (0..total)
        .filter(|i| parity_docmod_match(*i as u32, modulus, remainder))
        .collect();
    if to_delete.is_empty() {
        return Ok(0);
    }
    if to_delete.len() >= total {
        return Err("Cannot delete every page in the document".to_string());
    }
    to_delete.sort_by(|a, b| b.cmp(a));
    let deleted = to_delete.len() as u32;
    for idx in to_delete {
        kids.remove(idx);
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    doc.prune_objects();
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(deleted)
}

fn parity_docmod_keep(path: &Path, modulus: u32, remainder: u32) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (kids, _) = get_pages_kids(&doc)?;
    let kept: Vec<Object> = kids
        .iter()
        .enumerate()
        .filter(|(i, _)| parity_docmod_match(*i as u32, modulus, remainder))
        .map(|(_, kid)| kid.clone())
        .collect();
    if kept.is_empty() {
        return Err(format!("Document has no pages matching mod-{modulus} remainder {remainder}"));
    }
    if kept.len() == kids.len() {
        return Err("Nothing to delete - all pages match the keep filter".to_string());
    }
    let deleted = (kids.len() - kept.len()) as u32;
    set_pages_kids(&mut doc, pages_ref, kept)?;
    doc.prune_objects();
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(deleted)
}

fn parity_docmod_dup_append(path: &Path, modulus: u32, remainder: u32) -> Result<u32, String> {
    let path_buf = PathBuf::from(path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    let indices = parity_docmod_indices(total, modulus, remainder);
    if indices.is_empty() {
        return Ok(0);
    }
    let path_str = path_buf.to_string_lossy().into_owned();
    let copied = indices.len() as u32;
    for idx in indices {
        let at = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
        insert_pdf(path_str.clone(), path_str.clone(), at, idx, idx)?;
    }
    Ok(copied)
}

fn parity_docmod_flatten(path: &Path, modulus: u32, remainder: u32) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut removed = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let count = doc
            .get_dictionary(page_id)
            .ok()
            .and_then(|d| d.get(b"Annots").ok())
            .and_then(|o| o.as_array().ok())
            .map(|a| a.len() as u32)
            .unwrap_or(0);
        if count > 0 {
            doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.remove(b"Annots");
            removed += count;
        }
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(removed)
}

#[allow(clippy::too_many_arguments)]
fn parity_docmod_crop(path: &Path, modulus: u32, remainder: u32, margin_top: f64, margin_right: f64, margin_bottom: f64, margin_left: f64) -> Result<u32, String> {
    if margin_top < 0.0 || margin_right < 0.0 || margin_bottom < 0.0 || margin_left < 0.0 {
        return Err("Margins must be non-negative".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut cropped = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        apply_crop_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
        cropped += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(cropped)
}

#[allow(clippy::too_many_arguments)]
fn parity_docmod_expand(path: &Path, modulus: u32, remainder: u32, margin_top: f64, margin_right: f64, margin_bottom: f64, margin_left: f64) -> Result<u32, String> {
    if margin_top < 0.0 || margin_right < 0.0 || margin_bottom < 0.0 || margin_left < 0.0 {
        return Err("Margins must be non-negative".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut expanded = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        apply_expand_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
        expanded += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(expanded)
}

#[allow(clippy::too_many_arguments)]
fn parity_docmod_shrink(path: &Path, modulus: u32, remainder: u32, margin_top: f64, margin_right: f64, margin_bottom: f64, margin_left: f64) -> Result<u32, String> {
    if margin_top < 0.0 || margin_right < 0.0 || margin_bottom < 0.0 || margin_left < 0.0 {
        return Err("Margins must be non-negative".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut shrunk = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        apply_shrink_margins(&mut doc, page_id, margin_top, margin_right, margin_bottom, margin_left)?;
        shrunk += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(shrunk)
}

fn parity_docmod_reverse(path: &Path, modulus: u32, remainder: u32) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let parity_indices: Vec<usize> = (0..kids.len())
        .filter(|i| parity_docmod_match(*i as u32, modulus, remainder))
        .collect();
    if parity_indices.len() < 2 {
        return Ok(parity_indices.len() as u32);
    }
    let mut parity_kids: Vec<Object> = parity_indices.iter().map(|i| kids[*i].clone()).collect();
    parity_kids.reverse();
    for (pos, idx) in parity_indices.iter().enumerate() {
        kids[*idx] = parity_kids[pos].clone();
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(parity_indices.len() as u32)
}

fn parity_docmod_move_to_start(path: &Path, modulus: u32, remainder: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (kids, _) = get_pages_kids(&doc)?;
    let mut matched = Vec::new();
    let mut rest = Vec::new();
    for (i, kid) in kids.into_iter().enumerate() {
        if parity_docmod_match(i as u32, modulus, remainder) {
            matched.push(kid);
        } else {
            rest.push(kid);
        }
    }
    set_pages_kids(&mut doc, pages_ref, matched.into_iter().chain(rest).collect())?;
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn parity_docmod_move_to_end(path: &Path, modulus: u32, remainder: u32) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (kids, _) = get_pages_kids(&doc)?;
    let mut matched = Vec::new();
    let mut rest = Vec::new();
    for (i, kid) in kids.into_iter().enumerate() {
        if parity_docmod_match(i as u32, modulus, remainder) {
            matched.push(kid);
        } else {
            rest.push(kid);
        }
    }
    set_pages_kids(&mut doc, pages_ref, rest.into_iter().chain(matched).collect())?;
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn parity_docmod_clear_crop(path: &Path, modulus: u32, remainder: u32) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut cleared = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        if doc.get_dictionary(page_id).ok().and_then(|d| d.get(b"CropBox").ok()).is_some() {
            doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?.remove(b"CropBox");
            cleared += 1;
        }
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(cleared)
}

fn parity_docmod_dup_before(path: &Path, modulus: u32, remainder: u32) -> Result<u32, String> {
    let path_buf = PathBuf::from(path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    let indices = parity_docmod_indices(total, modulus, remainder);
    if indices.is_empty() {
        return Ok(0);
    }
    let path_str = path_buf.to_string_lossy().into_owned();
    let copied = indices.len() as u32;
    for idx in indices.into_iter().rev() {
        insert_pdf(path_str.clone(), path_str.clone(), idx, idx, idx)?;
    }
    Ok(copied)
}

fn parity_docmod_sort_rotation(path: &Path, modulus: u32, remainder: u32, descending: bool) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let parity_indices: Vec<usize> = (0..kids.len())
        .filter(|i| parity_docmod_match(*i as u32, modulus, remainder))
        .collect();
    if parity_indices.len() < 2 {
        return Ok(parity_indices.len() as u32);
    }
    let mut indexed: Vec<(usize, i64, Object)> = parity_indices
        .iter()
        .map(|i| {
            let kid = kids[*i].clone();
            let rot = kid.as_reference().ok().map(|id| page_rotation(&doc, id).rem_euclid(360)).unwrap_or(0);
            (*i, rot, kid)
        })
        .collect();
    indexed.sort_by(|a, b| {
        let ord = a.1.cmp(&b.1);
        if descending { ord.reverse() } else { ord }
    });
    let sorted_kids: Vec<Object> = indexed.into_iter().map(|(_, _, kid)| kid).collect();
    for (pos, idx) in parity_indices.iter().enumerate() {
        kids[*idx] = sorted_kids[pos].clone();
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(parity_indices.len() as u32)
}

fn parity_docmod_sort_size(path: &Path, modulus: u32, remainder: u32, descending: bool) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let parity_indices: Vec<usize> = (0..kids.len())
        .filter(|i| parity_docmod_match(*i as u32, modulus, remainder))
        .collect();
    if parity_indices.len() < 2 {
        return Ok(parity_indices.len() as u32);
    }
    let mut indexed: Vec<(usize, f64, Object)> = parity_indices
        .iter()
        .map(|i| {
            let kid = kids[*i].clone();
            let area = kid
                .as_reference()
                .ok()
                .and_then(|id| page_media_box(&doc, id).ok())
                .map(|m| (m[2] - m[0]) * (m[3] - m[1]))
                .unwrap_or(0.0);
            (*i, area, kid)
        })
        .collect();
    indexed.sort_by(|a, b| {
        let ord = a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal);
        if descending { ord.reverse() } else { ord }
    });
    let sorted_kids: Vec<Object> = indexed.into_iter().map(|(_, _, kid)| kid).collect();
    for (pos, idx) in parity_indices.iter().enumerate() {
        kids[*idx] = sorted_kids[pos].clone();
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(parity_indices.len() as u32)
}

fn parity_docmod_page_numbers(path: &Path, modulus: u32, remainder: u32, prefix: Option<String>) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let prefix = prefix.unwrap_or_default();
    let total = doc.get_pages().len() as u32;
    let mut stamped = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let font_name = ensure_helvetica_font(&mut doc, page_id)?;
        let (px, py) = viewer_point_to_pdf(&doc, page_id, 380.0, 1100.0)?;
        let label = format!("{prefix}{}", page_index + 1);
        let ops = build_page_number_ops(&font_name, &label, px, py, 12.0);
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        stamped += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(stamped)
}

fn parity_docmod_watermark(path: &Path, modulus: u32, remainder: u32, text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Watermark text cannot be empty".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut stamped = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let font_name = ensure_helvetica_font(&mut doc, page_id)?;
        let (cx, cy) = viewer_point_to_pdf(&doc, page_id, 400.0, 566.0)?;
        let ops = build_watermark_ops(&font_name, trimmed, cx, cy);
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        stamped += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(stamped)
}

fn parity_docmod_header(path: &Path, modulus: u32, remainder: u32, text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Header text cannot be empty".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut stamped = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let font_name = ensure_helvetica_font(&mut doc, page_id)?;
        let (px, py) = viewer_point_to_pdf(&doc, page_id, 380.0, 40.0)?;
        let ops = build_page_number_ops(&font_name, trimmed, px, py, 12.0);
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        stamped += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(stamped)
}

fn parity_docmod_footer(path: &Path, modulus: u32, remainder: u32, text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Footer text cannot be empty".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut stamped = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let font_name = ensure_helvetica_font(&mut doc, page_id)?;
        let (px, py) = viewer_point_to_pdf(&doc, page_id, 380.0, 1100.0)?;
        let ops = build_page_number_ops(&font_name, trimmed, px, py, 12.0);
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        stamped += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(stamped)
}

fn parity_docmod_border(path: &Path, modulus: u32, remainder: u32, inset: f64) -> Result<u32, String> {
    if inset < 0.0 {
        return Err("Border inset must be non-negative".to_string());
    }
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut drawn = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let ops = build_page_border_ops(&doc, page_id, inset)?;
        append_page_content(&mut doc, page_id, ops.as_bytes())?;
        drawn += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(drawn)
}

fn parity_docmod_bookmark(path: &Path, modulus: u32, remainder: u32, prefix: Option<String>) -> Result<u32, String> {
    let prefix = prefix.unwrap_or_else(|| "Page ".to_string());
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let mut count = 0u32;
    for (page_num, page_id) in doc.get_pages() {
        let page_index = page_num - 1;
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let title = format!("{prefix}{page_num}");
        append_outline_item(&mut doc, &title, page_id)?;
        count += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(count)
}

fn parity_docmod_page_size(path: &Path, modulus: u32, remainder: u32, preset: &str) -> Result<u32, String> {
    let (w, h) = page_size_preset_dims(preset)?;
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let total = doc.get_pages().len() as u32;
    let mut resized = 0u32;
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or("Page not found".to_string())?;
        let page = doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?;
        page.set(
            "MediaBox",
            Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Real(w as f32), Object::Real(h as f32)]),
        );
        page.remove(b"CropBox");
        resized += 1;
    }
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(resized)
}

fn parity_docmod_blank(path: &Path, modulus: u32, remainder: u32, after: bool) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let indices: Vec<usize> = (0..kids.len())
        .filter(|i| parity_docmod_match(*i as u32, modulus, remainder))
        .collect();
    if indices.is_empty() {
        return Ok(0);
    }
    for &idx in indices.iter().rev() {
        let page_id = create_blank_page(&mut doc, pages_ref);
        let at = if after { idx + 1 } else { idx };
        kids.insert(at, Object::Reference(page_id));
    }
    set_pages_kids(&mut doc, pages_ref, kids)?;
    doc.save(path).map_err(|e| e.to_string())?;
    Ok(indices.len() as u32)
}

fn parity_docmod_dup_to_end(path: &Path, modulus: u32, remainder: u32) -> Result<u32, String> {
    let path_buf = PathBuf::from(path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    let indices = parity_docmod_indices(total, modulus, remainder);
    if indices.is_empty() {
        return Ok(0);
    }
    let path_str = path_buf.to_string_lossy().into_owned();
    let copied = indices.len() as u32;
    for idx in indices.into_iter().rev() {
        let new_idx = duplicate_page(path_str.clone(), idx)?;
        move_page_to_last(path_str.clone(), new_idx)?;
    }
    Ok(copied)
}

fn parity_docmod_dup_to_start(path: &Path, modulus: u32, remainder: u32) -> Result<u32, String> {
    let path_buf = PathBuf::from(path);
    let total = Document::load(&path_buf).map_err(|e| e.to_string())?.get_pages().len() as u32;
    let indices = parity_docmod_indices(total, modulus, remainder);
    if indices.is_empty() {
        return Ok(0);
    }
    let path_str = path_buf.to_string_lossy().into_owned();
    let copied = indices.len() as u32;
    for (offset, idx) in indices.into_iter().rev().enumerate() {
        let offset = offset as u32;
        let source = idx + offset;
        insert_pdf(path_str.clone(), path_str.clone(), offset, source, source)?;
    }
    Ok(copied)
}

fn parity_docmod_extract(path: &Path, output_path: &Path, modulus: u32, remainder: u32) -> Result<String, String> {
    if path == output_path {
        return Err("Output path must differ from the source PDF".to_string());
    }
    let doc = Document::load(path).map_err(|e| e.to_string())?;
    let (all_kids, pages_ref) = get_pages_kids(&doc)?;
    let subset: Vec<Object> = all_kids
        .iter()
        .enumerate()
        .filter(|(i, _)| parity_docmod_match(*i as u32, modulus, remainder))
        .map(|(_, kid)| kid.clone())
        .collect();
    if subset.is_empty() {
        return Err(format!("Document has no pages matching mod-{modulus} remainder {remainder}"));
    }
    let mut out = Document::load(path).map_err(|e| e.to_string())?;
    set_pages_kids(&mut out, pages_ref, subset)?;
    out.prune_objects();
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    out.save(output_path).map_err(|e| e.to_string())?;
    Ok(output_path.to_string_lossy().into_owned())
}

fn parity_docmod_export_pdf(path: &Path, output_dir: &Path, modulus: u32, remainder: u32) -> Result<Vec<String>, String> {
    if !path.is_file() {
        return Err("File not found".to_string());
    }
    let total = Document::load(path).map_err(|e| e.to_string())?.get_pages().len() as u32;
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    let path_str = path.to_string_lossy().into_owned();
    let mut written = Vec::new();
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let file_name = format!("page-{:03}.pdf", page_index + 1);
        let output_path = output_dir.join(file_name);
        let out = extract_pdf_pages(path_str.clone(), output_path.to_string_lossy().into_owned(), page_index, page_index)?;
        written.push(out);
    }
    Ok(written)
}

fn parity_docmod_export_rendered(path: &Path, output_dir: &Path, modulus: u32, remainder: u32, ext: &str, render: ParityPageRenderFn) -> Result<Vec<String>, String> {
    if !path.is_file() {
        return Err("File not found".to_string());
    }
    let total = Document::load(path).map_err(|e| e.to_string())?.get_pages().len() as u32;
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    let mut written = Vec::new();
    for page_index in 0..total {
        if !parity_docmod_match(page_index, modulus, remainder) {
            continue;
        }
        let bytes = render(path, page_index, EXPORT_PNG_W, EXPORT_PNG_H)?;
        let file_name = format!("page-{:03}.{ext}", page_index + 1);
        let output_path = output_dir.join(file_name);
        write_png_output(&output_path, &bytes)?;
        written.push(output_path.to_string_lossy().into_owned());
    }
    Ok(written)
}
"""


def command_sig(kind: str) -> tuple[str, str]:
    if kind == "extract":
        return "path: String, output_path: String", "Result<String, String>"
    if kind.startswith("export"):
        return "path: String, output_dir: String", "Result<Vec<String>, String>"
    if kind in ("crop", "expand", "shrink"):
        return (
            "path: String, margin_top: f64, margin_right: f64, margin_bottom: f64, margin_left: f64",
            "Result<u32, String>",
        )
    if kind in ("bookmark", "page_numbers"):
        return "path: String, prefix: Option<String>", "Result<u32, String>"
    if kind in ("watermark", "header", "footer"):
        return "path: String, text: String", "Result<u32, String>"
    if kind == "page_size":
        return "path: String, preset: String", "Result<u32, String>"
    if kind == "border":
        return "path: String, inset: f64", "Result<u32, String>"
    if kind in ("sort_rot", "sort_size"):
        return "path: String, descending: bool", "Result<u32, String>"
    if kind in ("move_start", "move_end"):
        return "path: String", "Result<(), String>"
    return "path: String", "Result<u32, String>"


def impl_call(kind: str, modulus: int, remainder: int) -> str:
    m, r = str(modulus), str(remainder)
    path = "&PathBuf::from(&path)"
    pl = f"{path}, {m}, {r}"
    calls = {
        "rotate_cw": f"parity_docmod_rotate({pl}, 90)",
        "rotate_ccw": f"parity_docmod_rotate({pl}, -90)",
        "rotate_180": f"parity_docmod_rotate({pl}, 180)",
        "reset_rot": f"parity_docmod_reset_rotation({pl})",
        "delete": f"parity_docmod_delete({pl})",
        "keep": f"parity_docmod_keep({pl})",
        "dup_append": f"parity_docmod_dup_append({pl})",
        "flatten": f"parity_docmod_flatten({pl})",
        "crop": f"parity_docmod_crop({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "expand": f"parity_docmod_expand({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "shrink": f"parity_docmod_shrink({pl}, margin_top, margin_right, margin_bottom, margin_left)",
        "reverse": f"parity_docmod_reverse({pl})",
        "move_start": f"parity_docmod_move_to_start({pl})",
        "move_end": f"parity_docmod_move_to_end({pl})",
        "clear_crop": f"parity_docmod_clear_crop({pl})",
        "dup_before": f"parity_docmod_dup_before({pl})",
        "sort_rot": f"parity_docmod_sort_rotation({pl}, descending)",
        "sort_size": f"parity_docmod_sort_size({pl}, descending)",
        "page_numbers": f"parity_docmod_page_numbers({pl}, prefix)",
        "watermark": f"parity_docmod_watermark({pl}, &text)",
        "header": f"parity_docmod_header({pl}, &text)",
        "footer": f"parity_docmod_footer({pl}, &text)",
        "border": f"parity_docmod_border({pl}, inset)",
        "bookmark": f"parity_docmod_bookmark({pl}, prefix)",
        "page_size": f"parity_docmod_page_size({pl}, &preset)",
        "blank_before": f"parity_docmod_blank({pl}, false)",
        "blank_after": f"parity_docmod_blank({pl}, true)",
        "dup_to_end": f"parity_docmod_dup_to_end({pl})",
        "dup_to_start": f"parity_docmod_dup_to_start({pl})",
        "extract": f"parity_docmod_extract({path}, &PathBuf::from(&output_path), {m}, {r})",
        "export_pdf": f"parity_docmod_export_pdf({path}, &PathBuf::from(&output_dir), {m}, {r})",
        "export_png": f'parity_docmod_export_rendered({path}, &PathBuf::from(&output_dir), {m}, {r}, "png", render_page_png)',
        "export_jpeg": f'parity_docmod_export_rendered({path}, &PathBuf::from(&output_dir), {m}, {r}, "jpeg", render_page_jpeg)',
        "export_webp": f'parity_docmod_export_rendered({path}, &PathBuf::from(&output_dir), {m}, {r}, "webp", render_page_webp)',
        "export_bmp": f'parity_docmod_export_rendered({path}, &PathBuf::from(&output_dir), {m}, {r}, "bmp", render_page_bmp)',
        "export_tiff": f'parity_docmod_export_rendered({path}, &PathBuf::from(&output_dir), {m}, {r}, "tiff", render_page_tiff)',
        "export_gif": f'parity_docmod_export_rendered({path}, &PathBuf::from(&output_dir), {m}, {r}, "gif", render_page_gif)',
        "export_ppm": f'parity_docmod_export_rendered({path}, &PathBuf::from(&output_dir), {m}, {r}, "ppm", render_page_ppm)',
        "export_tga": f'parity_docmod_export_rendered({path}, &PathBuf::from(&output_dir), {m}, {r}, "tga", render_page_tga)',
        "export_ico": f'parity_docmod_export_rendered({path}, &PathBuf::from(&output_dir), {m}, {r}, "ico", render_page_ico)',
    }
    return calls[kind]


def gen_command(name: str, kind: str, modulus: int, remainder: int) -> str:
    sig, ret = command_sig(kind)
    call = impl_call(kind, modulus, remainder)
    return f"""
/// Document-wide mod-{modulus} remainder {remainder} - {kind}
#[tauri::command]
fn {name}({sig}) -> {ret} {{
    {call}
}}
"""


def gen_test(name: str, kind: str) -> str:
    setup = ""
    extra = ""
    if kind == "extract":
        setup = '        let output_path = tmp("extract_out.pdf");\n'
        extra = ", output_path.to_string_lossy().into_owned()"
    elif kind.startswith("export"):
        setup = '        let output_dir = tmp("export_dir");\n'
        extra = ", output_dir.to_string_lossy().into_owned()"
    elif kind in ("crop", "expand", "shrink"):
        extra = ", 0.0, 0.0, 0.0, 0.0"
    elif kind in ("bookmark", "page_numbers"):
        extra = ", None"
    elif kind in ("watermark", "header", "footer"):
        extra = ', "wm".to_string()'
    elif kind == "page_size":
        extra = ', "letter".to_string()'
    elif kind == "border":
        extra = ", 1.0"
    elif kind in ("sort_rot", "sort_size"):
        extra = ", false"
    return f"""
    #[test]
    fn {name}_rejects_missing_file() {{
        let missing = tmp("{name}_missing");
{setup}        let err = {name}(missing.to_string_lossy().into_owned(){extra}).unwrap_err();
        assert!(!err.is_empty());
    }}
"""


def build_specs() -> list[tuple[str, str, int, int]]:
    specs: list[tuple[str, str, int, int]] = []
    for modulus in MODULI:
        for rem in range(modulus):
            for kind, tmpl in DOC_OPS:
                name = tmpl.format(m=modulus, r=rem)
                specs.append((name, kind, modulus, rem))
    return specs


def load_prior_command_names() -> list[str]:
    import importlib.util

    names: list[str] = []
    for script in (
        "gen-parity-batch.py",
        "gen-parity-batch2.py",
        "gen-parity-batch3.py",
        "gen-parity-batch4.py",
        "gen-parity-batch5.py",
        "gen-parity-batch6.py",
        "gen-parity-batch7.py",
        "gen-parity-batch8.py",
    ):
        path = ROOT / "scripts" / script
        spec = importlib.util.spec_from_file_location(script, path)
        mod = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(mod)
        names.extend(s[0] for s in mod.build_specs())
    return names


def main() -> None:
    specs = build_specs()
    expected = sum(len(DOC_OPS) * m for m in MODULI)
    assert len(specs) == expected, (len(specs), expected)

    lines = [
        "// Auto-generated by scripts/gen-parity-docmod.py - do not edit.",
        rust_helpers(),
    ]
    tests = ["// Auto-generated parity docmod tests"]
    handlers: list[str] = []

    for name, kind, modulus, rem in specs:
        lines.append(gen_command(name, kind, modulus, rem))
        tests.append(gen_test(name, kind))
        handlers.append(f"            {name},")

    INC.write_text("\n".join(lines))
    print(f"Wrote {INC} ({len(specs)} commands)")

    import sys

    sys.path.insert(0, str(ROOT / "scripts"))
    from parity_patch import patch_sources

    patch_sources("DOCMOD", handlers, tests)

    merged = load_prior_command_names() + [s[0] for s in specs]
    UI_JSON.write_text(json.dumps(merged, indent=2))
    print(f"Wrote {UI_JSON} ({len(merged)} total commands)")


if __name__ == "__main__":
    main()
