use crate::pdf::fonts::dedup_fonts_after_insert;
use crate::pdf::form_merge::merge_acroform_after_insert;
use crate::pdf::import::import_object;
use crate::pdf::page_tree::{flatten_pages, get_pages_kids, set_pages_kids};
use lopdf::{Object, ObjectId};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn split_pdf(path: &Path, page_ranges: Vec<(u32, u32)>) -> Result<Vec<String>, String> {
    let mut doc = crate::pdf::render::cached_document(path).map_err(|e| e.to_string())?;

    if page_ranges.is_empty() {
        return Err("At least one page range is required".to_string());
    }

    let (all_kids, pages_ref) = get_pages_kids(&doc)?;
    let total_pages = all_kids.len() as u32;

    for (start, end) in &page_ranges {
        if *start >= total_pages || *end >= total_pages || *start > *end {
            return Err(format!("Invalid page range: {}-{}", start, end));
        }
    }

    let mut output_paths = Vec::new();

    for (i, (start, end)) in page_ranges.iter().enumerate() {
        let range_kids: Vec<Object> = all_kids[*start as usize..=*end as usize].to_vec();
        set_pages_kids(&mut doc, pages_ref, range_kids)?;

        // Drop the pages (and their now-orphaned content/resources) that aren't
        // part of this range so each split file is actually smaller rather than
        // a full copy with a trimmed page list.
        doc.prune_objects();

        let output_path = path.with_file_name(format!(
            "{}_part{}.pdf",
            path.file_stem().unwrap_or_else(|| std::ffi::OsStr::new("document")).to_string_lossy(),
            i + 1
        ));
        crate::pdf::io::save_atomic(&mut doc, &output_path)?;
        output_paths.push(output_path.to_string_lossy().to_string());

        doc = crate::pdf::render::cached_document(path).map_err(|e| e.to_string())?;
    }

    Ok(output_paths)
}

pub fn extract_pdf_pages(path: &Path, output_path: &Path, start_page: u32, end_page: u32) -> Result<String, String> {
    if path == output_path {
        return Err("Output path must differ from the source PDF".to_string());
    }

    let mut doc = crate::pdf::render::cached_document(path).map_err(|e| e.to_string())?;
    let (all_kids, pages_ref) = get_pages_kids(&doc)?;
    let total_pages = all_kids.len() as u32;
    if start_page >= total_pages || end_page >= total_pages || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }

    let range_kids: Vec<Object> = all_kids[start_page as usize..=end_page as usize].to_vec();
    set_pages_kids(&mut doc, pages_ref, range_kids)?;
    doc.prune_objects();

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    crate::pdf::io::save_atomic(&mut doc, output_path)?;
    Ok(output_path.to_string_lossy().into_owned())
}

pub fn insert_pdf(
    path: &Path,
    insert_path: &Path,
    at_index: u32,
    insert_start: u32,
    insert_end: u32,
) -> Result<(), String> {
    let mut doc = crate::pdf::render::cached_document(path).map_err(|e| e.to_string())?;
    let insert_doc = crate::pdf::render::cached_document(insert_path).map_err(|e| e.to_string())?;

    // Flatten the destination so /Kids is a flat leaf list we can index into.
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut source_kids, _) = get_pages_kids(&doc)?;

    // Resolve source pages through their (possibly nested) tree, in page order.
    let source_pages: Vec<ObjectId> = insert_doc.get_pages().into_values().collect();
    let insert_start = insert_start as usize;
    let insert_end = insert_end as usize;
    if insert_start > insert_end || insert_end >= source_pages.len() {
        return Err("Invalid insert page range".to_string());
    }
    let at = at_index as usize;
    if at > source_kids.len() {
        return Err("Insert index out of bounds".to_string());
    }

    // Deep-copy the selected pages (and their content/resources) into `doc` so
    // the saved file is self-contained - the old code copied bare references that
    // dangled. `remap` is shared so resources common to several pages are copied
    // once.
    let mut remap = BTreeMap::new();
    let new_page_ids: Vec<ObjectId> = source_pages[insert_start..=insert_end]
        .iter()
        .map(|&src_page| import_object(&mut doc, &insert_doc, src_page, pages_ref, &mut remap))
        .collect();
    for (offset, page_id) in new_page_ids.iter().enumerate() {
        source_kids.insert(at + offset, Object::Reference(*page_id));
    }

    set_pages_kids(&mut doc, pages_ref, source_kids)?;
    merge_acroform_after_insert(&mut doc, &insert_doc, &new_page_ids, &remap)?;
    dedup_fonts_after_insert(&mut doc, &new_page_ids)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    crate::pdf::render::invalidate_document_cache(path);
    Ok(())
}
