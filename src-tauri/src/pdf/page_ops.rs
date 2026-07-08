use crate::pdf::merge_split::insert_pdf;
use crate::pdf::page_decor::create_blank_page;
use crate::pdf::page_tree::{delete_kids_in_range, flatten_pages, get_pages_kids, set_pages_kids};
use lopdf::{Document, Object};
use std::path::Path;

pub fn delete_page(path: &Path, page_index: u32) -> Result<(), String> {
    crate::pdf::io::mutate_pdf(path, |doc| {
        let total = doc.get_pages().len();
        if total <= 1 {
            return Err("Cannot delete the only page in the document".to_string());
        }
        let idx = page_index as usize;
        if idx >= total {
            return Err("Page index out of bounds".to_string());
        }
        doc.delete_pages(&[page_index + 1]);
        Ok(())
    })
}

pub fn move_page(path: &Path, from_index: u32, to_index: u32) -> Result<(), String> {
    crate::pdf::io::mutate_pdf(path, |doc| {
        let pages_ref = flatten_pages(doc)?;
        let (mut kids, _) = get_pages_kids(doc)?;

        let from = from_index as usize;
        let to = to_index as usize;
        if from >= kids.len() || to >= kids.len() {
            return Err("Index out of bounds".to_string());
        }
        if from == to {
            return Ok(());
        }

        let moved = kids.remove(from);
        kids.insert(to, moved);
        set_pages_kids(doc, pages_ref, kids)?;
        Ok(())
    })
}

pub fn move_page_between_documents(
    source_path: &Path,
    dest_path: &Path,
    source_index: u32,
    dest_index: u32,
) -> Result<(), String> {
    if source_path == dest_path {
        let total = crate::pdf::io::page_count(source_path)? as u32;
        if total == 0 {
            return Err("Document has no pages".to_string());
        }
        let target = if dest_index >= total { total - 1 } else { dest_index };
        return move_page(source_path, source_index, target);
    }

    let source_count = crate::pdf::io::page_count(source_path)? as u32;
    if source_count <= 1 {
        return Err("Cannot move the only page in the source document".to_string());
    }
    if source_index >= source_count {
        return Err("Source page index out of bounds".to_string());
    }

    let dest_count = crate::pdf::io::page_count(dest_path)? as u32;
    if dest_index > dest_count {
        return Err("Destination insert index out of bounds".to_string());
    }

    insert_pdf(dest_path, source_path, dest_index, source_index, source_index)?;
    delete_page(source_path, source_index)?;
    Ok(())
}

pub fn duplicate_page(path: &Path, page_index: u32) -> Result<u32, String> {
    let page_count = crate::pdf::io::page_count(path)?;
    let idx = page_index as usize;
    if idx >= page_count {
        return Err("Page index out of bounds".to_string());
    }
    insert_pdf(path, path, page_index + 1, page_index, page_index)?;
    Ok(page_index + 1)
}

pub fn merge_pdf(path: &Path, merge_path: &Path, merge_start: u32, merge_end: u32) -> Result<u32, String> {
    if path == merge_path {
        return Err("Cannot merge a PDF into itself".to_string());
    }

    let at_index = Document::load(path).map_err(|e| e.to_string())?.get_pages().len() as u32;
    let source_count = Document::load(merge_path).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if source_count == 0 {
        return Err("Source PDF has no pages".to_string());
    }
    if merge_start > merge_end || merge_end >= source_count {
        return Err("Invalid merge page range".to_string());
    }

    insert_pdf(path, merge_path, at_index, merge_start, merge_end)?;
    Ok(merge_end - merge_start + 1)
}

pub fn reverse_pages(path: &Path) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    kids.reverse();
    set_pages_kids(&mut doc, pages_ref, kids)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(())
}

pub fn add_blank_page(path: &Path, at_index: u32) -> Result<u32, String> {
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let at = at_index as usize;
    if at > kids.len() {
        return Err("Insert index out of bounds".to_string());
    }
    let page_id = create_blank_page(&mut doc, pages_ref);
    kids.insert(at, Object::Reference(page_id));
    set_pages_kids(&mut doc, pages_ref, kids)?;
    crate::pdf::io::save_atomic(&mut doc, path)?;
    Ok(at_index)
}

pub fn delete_page_range(path: &Path, start_page: u32, end_page: u32) -> Result<u32, String> {
    crate::pdf::io::mutate_pdf(path, |doc| delete_kids_in_range(doc, start_page, end_page))
}

pub fn duplicate_page_range(path: &Path, start_page: u32, end_page: u32) -> Result<u32, String> {
    let total = Document::load(path).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if start_page >= total || end_page >= total || start_page > end_page {
        return Err(format!("Invalid page range: {start_page}-{end_page}"));
    }
    let count = end_page - start_page + 1;
    insert_pdf(path, path, end_page + 1, start_page, end_page)?;
    Ok(count)
}

pub fn move_page_to_first(path: &Path, page_index: u32) -> Result<(), String> {
    if page_index == 0 {
        return Ok(());
    }
    move_page(path, page_index, 0)
}

pub fn move_page_to_last(path: &Path, page_index: u32) -> Result<(), String> {
    let last = Document::load(path).map_err(|e| e.to_string())?.get_pages().len() as u32;
    if last == 0 {
        return Err("Document has no pages".to_string());
    }
    let last_index = last - 1;
    if page_index == last_index {
        return Ok(());
    }
    if page_index >= last {
        return Err("Page index out of bounds".to_string());
    }
    move_page(path, page_index, last_index)
}
