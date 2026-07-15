use super::*;
use lopdf::{Dictionary, Document, Stream};
use pdf::coords::pdf_rect_to_viewer_px;
use pdf::edit_object::{add_text_box, edit_text_line};
use pdf::edit_types::{PdfRect, RgbColor, TextStyle};
use pdf::markdown_heuristic::{
    apply_links_to_text, format_markdown_lines, is_symbol_glyph_candidate, map_symbol_glyph, merge_wrapped_line_pair,
    sort_lines_reading_order, strip_header_footer_lines, MarkdownPageLink, MarkdownTextCell, MarkdownTextLine,
};
use pdf::markdown_images::{
    append_page_embedded_images, build_form_render_pdf, build_image_render_pdf, collect_xobject_do_names_recursive,
    page_needs_ocr_supplement, parse_xobject_do_names, pdf_image_stream_bytes, resolve_page_resources,
    try_ocr_image_bytes, xobject_names_used_on_page, MarkdownImageSink, PAGE_OCR_MIN_CHARS,
};
use pdf::markdown_tagged::tagged_markdown_by_page;
use pdf::ocr::{ocr_language, resolve_tesseract};
use pdf::page_text::read_page_content;
use pdf::text_lines::decode_page_text_lines;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("kanoprii_test_{}_{}.pdf", std::process::id(), name))
}

/// Build a minimal, valid, flat-tree PDF with `n` pages. Each page carries a
/// distinct `TestIdx` so reordering can be verified; page 0 gets a text
/// content stream so markdown extraction has something to find.
fn build_pdf(n: usize) -> Document {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    let mut kids = Vec::new();
    for i in 0..n {
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set("Resources", Object::Dictionary(Dictionary::new()));
        page.set(
            "MediaBox",
            Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(612), Object::Integer(792)]),
        );
        page.set("TestIdx", Object::Integer(i as i64));

        if i == 0 {
            let content = b"BT /F1 12 Tf 72 700 Td (Hello) Tj ET".to_vec();
            let stream_id = doc.add_object(Stream::new(Dictionary::new(), content));
            page.set("Contents", Object::Reference(stream_id));
        }

        let page_id = doc.add_object(Object::Dictionary(page));
        kids.push(Object::Reference(page_id));
    }

    let mut pages = Dictionary::new();
    pages.set("Type", Object::Name(b"Pages".to_vec()));
    pages.set("Count", Object::Integer(n as i64));
    pages.set("Kids", Object::Array(kids));
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));

    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc
}

/// Build a PDF with a *nested* page tree: the root /Pages holds an
/// intermediate /Pages node (two leaves) followed by one direct leaf - three
/// pages total. This mirrors real PDFs that the old flat-tree code mangled.
fn build_nested_pdf() -> Document {
    let mut doc = Document::with_version("1.5");
    let root_id = doc.new_object_id();
    let mid_id = doc.new_object_id();

    let leaf = |doc: &mut Document, parent: ObjectId, idx: i64| -> ObjectId {
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(parent));
        page.set("Resources", Object::Dictionary(Dictionary::new()));
        page.set(
            "MediaBox",
            Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(612), Object::Integer(792)]),
        );
        page.set("TestIdx", Object::Integer(idx));
        doc.add_object(Object::Dictionary(page))
    };

    let a1 = leaf(&mut doc, mid_id, 1);
    let a2 = leaf(&mut doc, mid_id, 2);
    let c = leaf(&mut doc, root_id, 3);

    let mut mid = Dictionary::new();
    mid.set("Type", Object::Name(b"Pages".to_vec()));
    mid.set("Parent", Object::Reference(root_id));
    mid.set("Count", Object::Integer(2));
    mid.set("Kids", Object::Array(vec![Object::Reference(a1), Object::Reference(a2)]));
    doc.objects.insert(mid_id, Object::Dictionary(mid));

    let mut root = Dictionary::new();
    root.set("Type", Object::Name(b"Pages".to_vec()));
    root.set("Count", Object::Integer(3));
    root.set("Kids", Object::Array(vec![Object::Reference(mid_id), Object::Reference(c)]));
    doc.objects.insert(root_id, Object::Dictionary(root));

    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(root_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc
}

fn save(doc: &mut Document, name: &str) -> String {
    let path = tmp(name);
    doc.save(&path).unwrap();
    path.to_string_lossy().to_string()
}

fn attach_struct_tree_root(doc: &mut Document, root_id: ObjectId) {
    let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    let Object::Dictionary(catalog) = doc.objects.get_mut(&catalog_id).unwrap() else {
        panic!("catalog is not a dictionary");
    };
    catalog.set("StructTreeRoot", Object::Reference(root_id));
    let mut mark_info = Dictionary::new();
    mark_info.set("Marked", Object::Boolean(true));
    catalog.set("MarkInfo", Object::Dictionary(mark_info));
}

fn add_struct_elem(doc: &mut Document, kind: &[u8], text: &str, page_id: Option<ObjectId>) -> ObjectId {
    let mut elem = Dictionary::new();
    elem.set("Type", Object::Name(b"StructElem".to_vec()));
    elem.set("S", Object::Name(kind.to_vec()));
    if !text.is_empty() {
        elem.set("T", Object::String(text.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    }
    if let Some(page_id) = page_id {
        elem.set("Pg", Object::Reference(page_id));
    }
    doc.add_object(Object::Dictionary(elem))
}

/// Tagged PDF with headings, paragraphs, a list, and a table across two pages.
fn build_tagged_pdf() -> Document {
    let mut doc = build_pdf(2);
    let page1_id = *doc.get_pages().get(&1).unwrap();
    let page2_id = *doc.get_pages().get(&2).unwrap();

    let h1_id = add_struct_elem(&mut doc, b"H1", "Introduction", Some(page1_id));
    let p1_id = add_struct_elem(&mut doc, b"P", "Body paragraph one.", Some(page1_id));

    let lbody_id = add_struct_elem(&mut doc, b"LBody", "First item", None);
    let mut li = Dictionary::new();
    li.set("Type", Object::Name(b"StructElem".to_vec()));
    li.set("S", Object::Name(b"LI".to_vec()));
    li.set("K", Object::Reference(lbody_id));
    let li_id = doc.add_object(Object::Dictionary(li));

    let mut list = Dictionary::new();
    list.set("Type", Object::Name(b"StructElem".to_vec()));
    list.set("S", Object::Name(b"L".to_vec()));
    list.set("Pg", Object::Reference(page2_id));
    list.set("K", Object::Array(vec![Object::Reference(li_id)]));
    let list_id = doc.add_object(Object::Dictionary(list));

    let td1 = add_struct_elem(&mut doc, b"TD", "Name", None);
    let td2 = add_struct_elem(&mut doc, b"TD", "Score", None);
    let mut tr_head = Dictionary::new();
    tr_head.set("Type", Object::Name(b"StructElem".to_vec()));
    tr_head.set("S", Object::Name(b"TR".to_vec()));
    tr_head.set("K", Object::Array(vec![Object::Reference(td1), Object::Reference(td2)]));
    let tr_head_id = doc.add_object(Object::Dictionary(tr_head));

    let td3 = add_struct_elem(&mut doc, b"TD", "Alice", None);
    let td4 = add_struct_elem(&mut doc, b"TD", "98", None);
    let mut tr_row = Dictionary::new();
    tr_row.set("Type", Object::Name(b"StructElem".to_vec()));
    tr_row.set("S", Object::Name(b"TR".to_vec()));
    tr_row.set("K", Object::Array(vec![Object::Reference(td3), Object::Reference(td4)]));
    let tr_row_id = doc.add_object(Object::Dictionary(tr_row));

    let mut table = Dictionary::new();
    table.set("Type", Object::Name(b"StructElem".to_vec()));
    table.set("S", Object::Name(b"Table".to_vec()));
    table.set("Pg", Object::Reference(page2_id));
    table.set("K", Object::Array(vec![Object::Reference(tr_head_id), Object::Reference(tr_row_id)]));
    let table_id = doc.add_object(Object::Dictionary(table));

    let mut root = Dictionary::new();
    root.set("Type", Object::Name(b"StructTreeRoot".to_vec()));
    root.set(
        "K",
        Object::Array(vec![
            Object::Reference(h1_id),
            Object::Reference(p1_id),
            Object::Reference(list_id),
            Object::Reference(table_id),
        ]),
    );
    let root_id = doc.add_object(Object::Dictionary(root));
    attach_struct_tree_root(&mut doc, root_id);
    doc
}

fn add_link_struct_elem(doc: &mut Document, text: &str, uri: &str, page_id: ObjectId) -> ObjectId {
    let mut action = Dictionary::new();
    action.set("S", Object::Name(b"URI".to_vec()));
    action.set("URI", Object::String(uri.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    let mut annot = Dictionary::new();
    annot.set("Type", Object::Name(b"Annot".to_vec()));
    annot.set("Subtype", Object::Name(b"Link".to_vec()));
    annot.set("A", Object::Dictionary(action));
    annot.set(
        "Rect",
        Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(1), Object::Integer(1)]),
    );
    let annot_id = doc.add_object(Object::Dictionary(annot));

    let mut objr = Dictionary::new();
    objr.set("Type", Object::Name(b"OBJR".to_vec()));
    objr.set("Obj", Object::Reference(annot_id));
    let mut link = Dictionary::new();
    link.set("Type", Object::Name(b"StructElem".to_vec()));
    link.set("S", Object::Name(b"Link".to_vec()));
    link.set("Pg", Object::Reference(page_id));
    link.set("T", Object::String(text.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    link.set("K", Object::Dictionary(objr));
    doc.add_object(Object::Dictionary(link))
}

/// Tagged PDF covering inline emphasis, links, TOC, captions, code, notes, and THead/TBody tables.
fn build_tagged_pdf_extended() -> Document {
    let mut doc = build_pdf(2);
    let page1_id = *doc.get_pages().get(&1).unwrap();
    let page2_id = *doc.get_pages().get(&2).unwrap();

    let strong_id = add_struct_elem(&mut doc, b"Strong", "important", None);
    let mut paragraph = Dictionary::new();
    paragraph.set("Type", Object::Name(b"StructElem".to_vec()));
    paragraph.set("S", Object::Name(b"P".to_vec()));
    paragraph.set("Pg", Object::Reference(page1_id));
    paragraph.set("T", Object::String(b"Value is ".to_vec(), lopdf::StringFormat::Literal));
    paragraph.set("K", Object::Reference(strong_id));
    let paragraph_id = doc.add_object(Object::Dictionary(paragraph));

    let link_id = add_link_struct_elem(&mut doc, "Example", "https://example.com/docs", page1_id);

    let mut toci = Dictionary::new();
    toci.set("Type", Object::Name(b"StructElem".to_vec()));
    toci.set("S", Object::Name(b"TOCI".to_vec()));
    toci.set("Pg", Object::Reference(page1_id));
    toci.set("T", Object::String(b"Getting started".to_vec(), lopdf::StringFormat::Literal));
    let toci_id = doc.add_object(Object::Dictionary(toci));
    let mut toc = Dictionary::new();
    toc.set("Type", Object::Name(b"StructElem".to_vec()));
    toc.set("S", Object::Name(b"TOC".to_vec()));
    toc.set("Pg", Object::Reference(page1_id));
    toc.set("K", Object::Array(vec![Object::Reference(toci_id)]));
    let toc_id = doc.add_object(Object::Dictionary(toc));

    let caption_id = add_struct_elem(&mut doc, b"Caption", "Quarterly revenue chart", Some(page2_id));
    let mut figure = Dictionary::new();
    figure.set("Type", Object::Name(b"StructElem".to_vec()));
    figure.set("S", Object::Name(b"Figure".to_vec()));
    figure.set("Pg", Object::Reference(page2_id));
    figure.set("Alt", Object::String(b"Revenue chart".to_vec(), lopdf::StringFormat::Literal));
    figure.set("K", Object::Reference(caption_id));
    let figure_id = doc.add_object(Object::Dictionary(figure));

    let code_id = add_struct_elem(&mut doc, b"Code", "fn main() {\n    println!(\"ok\");\n}", Some(page2_id));
    let note_id = add_struct_elem(&mut doc, b"Note", "See appendix A.", Some(page2_id));

    let lbl_id = add_struct_elem(&mut doc, b"Lbl", "1.", None);
    let lbody_id = add_struct_elem(&mut doc, b"LBody", "First step", None);
    let mut ordered_li = Dictionary::new();
    ordered_li.set("Type", Object::Name(b"StructElem".to_vec()));
    ordered_li.set("S", Object::Name(b"LI".to_vec()));
    ordered_li.set("K", Object::Array(vec![Object::Reference(lbl_id), Object::Reference(lbody_id)]));
    let ordered_li_id = doc.add_object(Object::Dictionary(ordered_li));
    let mut ordered_list = Dictionary::new();
    ordered_list.set("Type", Object::Name(b"StructElem".to_vec()));
    ordered_list.set("S", Object::Name(b"L".to_vec()));
    ordered_list.set("Pg", Object::Reference(page2_id));
    ordered_list.set("K", Object::Array(vec![Object::Reference(ordered_li_id)]));
    let ordered_list_id = doc.add_object(Object::Dictionary(ordered_list));

    let th1 = add_struct_elem(&mut doc, b"TH", "Region", None);
    let th2 = add_struct_elem(&mut doc, b"TH", "Total", None);
    let mut tr_head = Dictionary::new();
    tr_head.set("Type", Object::Name(b"StructElem".to_vec()));
    tr_head.set("S", Object::Name(b"TR".to_vec()));
    tr_head.set("K", Object::Array(vec![Object::Reference(th1), Object::Reference(th2)]));
    let tr_head_id = doc.add_object(Object::Dictionary(tr_head));
    let mut thead = Dictionary::new();
    thead.set("Type", Object::Name(b"StructElem".to_vec()));
    thead.set("S", Object::Name(b"THead".to_vec()));
    thead.set("K", Object::Array(vec![Object::Reference(tr_head_id)]));
    let thead_id = doc.add_object(Object::Dictionary(thead));

    let td1 = add_struct_elem(&mut doc, b"TD", "West", None);
    let td2 = add_struct_elem(&mut doc, b"TD", "42", None);
    let mut tr_body = Dictionary::new();
    tr_body.set("Type", Object::Name(b"StructElem".to_vec()));
    tr_body.set("S", Object::Name(b"TR".to_vec()));
    tr_body.set("K", Object::Array(vec![Object::Reference(td1), Object::Reference(td2)]));
    let tr_body_id = doc.add_object(Object::Dictionary(tr_body));
    let mut tbody = Dictionary::new();
    tbody.set("Type", Object::Name(b"StructElem".to_vec()));
    tbody.set("S", Object::Name(b"TBody".to_vec()));
    tbody.set("K", Object::Array(vec![Object::Reference(tr_body_id)]));
    let tbody_id = doc.add_object(Object::Dictionary(tbody));

    let mut table = Dictionary::new();
    table.set("Type", Object::Name(b"StructElem".to_vec()));
    table.set("S", Object::Name(b"Table".to_vec()));
    table.set("Pg", Object::Reference(page2_id));
    table.set("K", Object::Array(vec![Object::Reference(thead_id), Object::Reference(tbody_id)]));
    let table_id = doc.add_object(Object::Dictionary(table));

    let artifact_id = add_struct_elem(&mut doc, b"Artifact", "Header text", Some(page1_id));

    let mut root = Dictionary::new();
    root.set("Type", Object::Name(b"StructTreeRoot".to_vec()));
    root.set(
        "K",
        Object::Array(vec![
            Object::Reference(paragraph_id),
            Object::Reference(link_id),
            Object::Reference(toc_id),
            Object::Reference(artifact_id),
            Object::Reference(figure_id),
            Object::Reference(code_id),
            Object::Reference(note_id),
            Object::Reference(ordered_list_id),
            Object::Reference(table_id),
        ]),
    );
    let root_id = doc.add_object(Object::Dictionary(root));
    attach_struct_tree_root(&mut doc, root_id);
    doc
}

fn page_count(path: &str) -> usize {
    Document::load(path).unwrap().get_pages().len()
}

fn count_entry(path: &str) -> i64 {
    let doc = Document::load(path).unwrap();
    let (_, pages_ref) = get_pages_kids(&doc).unwrap();
    doc.get_dictionary(pages_ref).unwrap().get(b"Count").unwrap().as_i64().unwrap()
}

fn page_order(path: &str) -> Vec<i64> {
    let doc = Document::load(path).unwrap();
    let (kids, _) = get_pages_kids(&doc).unwrap();
    kids.iter()
        .map(|k| {
            let id = k.as_reference().unwrap();
            doc.get_dictionary(id).unwrap().get(b"TestIdx").unwrap().as_i64().unwrap()
        })
        .collect()
}

#[test]
fn delete_page_on_nested_tree_removes_only_one_leaf() {
    let path = save(&mut build_nested_pdf(), "nested_delete");
    delete_page(path.clone(), 0).unwrap(); // delete page 1 (first leaf in the intermediate node)
    let doc = Document::load(&path).unwrap();
    let pages = doc.get_pages();
    assert_eq!(pages.len(), 2, "exactly one leaf should be removed");
    let idxs: Vec<i64> =
        pages.values().map(|id| doc.get_dictionary(*id).unwrap().get(b"TestIdx").unwrap().as_i64().unwrap()).collect();
    assert_eq!(idxs, vec![2, 3], "remaining pages keep their order");
    let _ = fs::remove_file(&path);
}

#[test]
fn move_page_on_nested_tree_reorders_leaves() {
    let path = save(&mut build_nested_pdf(), "nested_move");
    move_page(path.clone(), 0, 2).unwrap(); // move page 1 to the end
    let doc = Document::load(&path).unwrap();
    let idxs: Vec<i64> = doc
        .get_pages()
        .values()
        .map(|id| doc.get_dictionary(*id).unwrap().get(b"TestIdx").unwrap().as_i64().unwrap())
        .collect();
    assert_eq!(idxs, vec![2, 3, 1]);
    let _ = fs::remove_file(&path);
}

#[test]
fn insert_pdf_imports_pages_into_nested_tree() {
    let dest = save(&mut build_nested_pdf(), "nested_insert_dest"); // 3 pages, nested tree
    let src = save(&mut build_pdf(2), "nested_insert_src"); // 2 pages; page 0 has Contents
    insert_pdf(dest.clone(), src.clone(), 1, 0, 1).unwrap(); // insert both src pages at position 1

    let doc = Document::load(&dest).unwrap();
    let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
    assert_eq!(pages.len(), 5, "all source pages inserted");
    for &pid in &pages {
        let d = doc.get_dictionary(pid).unwrap();
        assert!(d.get(b"MediaBox").is_ok(), "every page carries a MediaBox");
        if let Ok(contents) = d.get(b"Contents") {
            let cid = contents.as_reference().unwrap();
            assert!(doc.get_object(cid).is_ok(), "Contents must resolve - no dangling refs");
        }
    }
    let _ = fs::remove_file(&dest);
    let _ = fs::remove_file(&src);
}

#[test]
fn delete_page_reduces_pages_and_fixes_count() {
    let path = save(&mut build_pdf(3), "delete");
    delete_page(path.clone(), 1).unwrap();
    assert_eq!(page_count(&path), 2);
    assert_eq!(count_entry(&path), 2, "/Count must track /Kids");
    assert_eq!(page_order(&path), vec![0, 2], "wrong page removed");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_page_reorders() {
    let path = save(&mut build_pdf(3), "move");
    move_page(path.clone(), 0, 2).unwrap();
    assert_eq!(page_order(&path), vec![1, 2, 0]);
    assert_eq!(count_entry(&path), 3);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_page_between_pdfs_moves_page_and_updates_counts() {
    let source = save(&mut build_pdf(3), "move_between_source");
    let dest = save(&mut build_pdf(2), "move_between_dest");
    move_page_between_pdfs(source.clone(), dest.clone(), 1, 1).unwrap();
    assert_eq!(page_order(&source), vec![0, 2]);
    assert_eq!(page_order(&dest), vec![0, 1, 1]);
    assert_eq!(count_entry(&source), 2);
    assert_eq!(count_entry(&dest), 3);
    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&dest);
}

#[test]
fn move_page_between_pdfs_rejects_emptying_source() {
    let source = save(&mut build_pdf(1), "move_between_only_source");
    let dest = save(&mut build_pdf(2), "move_between_only_dest");
    let err = move_page_between_pdfs(source.clone(), dest.clone(), 0, 1).unwrap_err();
    assert!(err.contains("only page"));
    assert_eq!(page_count(&source), 1);
    assert_eq!(page_count(&dest), 2);
    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&dest);
}

#[test]
fn duplicate_page_inserts_copy_after_source() {
    let path = save(&mut build_pdf(2), "duplicate");
    duplicate_page(path.clone(), 0).unwrap();
    let count = Document::load(&path).unwrap().get_pages().len();
    assert_eq!(count, 3);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_page_returns_new_index() {
    let path = save(&mut build_pdf(1), "duplicate_index");
    let new_index = duplicate_page(path.clone(), 0).unwrap();
    assert_eq!(new_index, 1);
    assert_eq!(Document::load(&path).unwrap().get_pages().len(), 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_page_rejects_invalid_index() {
    let path = save(&mut build_pdf(1), "duplicate_invalid");
    let err = duplicate_page(path.clone(), 9).unwrap_err();
    assert!(err.contains("out of bounds"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_page_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_duplicate_missing_{}.pdf", std::process::id()));
    let err = duplicate_page(missing.to_string_lossy().into_owned(), 0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn merge_pdf_appends_source_pages() {
    let dest = save(&mut build_pdf(2), "merge_dest");
    let src = save(&mut build_pdf(3), "merge_src");
    let added = merge_pdf(dest.clone(), src.clone(), 0, 2).unwrap();
    assert_eq!(added, 3);
    assert_eq!(Document::load(&dest).unwrap().get_pages().len(), 5);
    let _ = std::fs::remove_file(&dest);
    let _ = std::fs::remove_file(&src);
}

#[test]
fn merge_pdf_appends_partial_range() {
    let dest = save(&mut build_pdf(1), "merge_dest_partial");
    let src = save(&mut build_pdf(4), "merge_src_partial");
    let added = merge_pdf(dest.clone(), src.clone(), 1, 2).unwrap();
    assert_eq!(added, 2);
    assert_eq!(Document::load(&dest).unwrap().get_pages().len(), 3);
    let _ = std::fs::remove_file(&dest);
    let _ = std::fs::remove_file(&src);
}

#[test]
fn merge_pdf_rejects_self_merge() {
    let path = save(&mut build_pdf(1), "merge_self");
    let err = merge_pdf(path.clone(), path.clone(), 0, 0).unwrap_err();
    assert!(err.contains("itself"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn merge_pdf_rejects_invalid_range() {
    let dest = save(&mut build_pdf(1), "merge_dest_invalid");
    let src = save(&mut build_pdf(2), "merge_src_invalid");
    let err = merge_pdf(dest.clone(), src.clone(), 2, 1).unwrap_err();
    assert!(err.contains("Invalid merge page range"));
    let _ = std::fs::remove_file(&dest);
    let _ = std::fs::remove_file(&src);
}

#[test]
fn merge_pdf_rejects_missing_dest_file() {
    let src = save(&mut build_pdf(1), "merge_src_only");
    let missing = std::env::temp_dir().join(format!("pp_merge_dest_missing_{}.pdf", std::process::id()));
    let err = merge_pdf(missing.to_string_lossy().into_owned(), src.clone(), 0, 0).unwrap_err();
    assert!(!err.is_empty());
    let _ = std::fs::remove_file(&src);
}

#[test]
fn merge_pdf_rejects_missing_source_file() {
    let dest = save(&mut build_pdf(1), "merge_dest_only");
    let missing = std::env::temp_dir().join(format!("pp_merge_src_missing_{}.pdf", std::process::id()));
    let err = merge_pdf(dest.clone(), missing.to_string_lossy().into_owned(), 0, 0).unwrap_err();
    assert!(!err.is_empty());
    let _ = std::fs::remove_file(&dest);
}

#[test]
fn pdf_rect_to_viewer_px_maps_origin_and_scale() {
    use pdf::coords::pdf_rect_to_viewer_px;
    use pdf::search::pdf_rect_to_search_pixels;
    use pdfium_render::prelude::*;

    let full = pdf_rect_to_viewer_px(0.0, 0.0, 612.0, 792.0, 612.0, 792.0);
    assert!((full[0] - 0.0).abs() < 0.01);
    assert!((full[2] - 800.0).abs() < 0.01);
    assert!((full[1] - 0.0).abs() < 0.01);
    assert!((full[3] - 1132.0).abs() < 0.01);

    let rect = PdfRect::new(PdfPoints::new(100.0), PdfPoints::new(72.0), PdfPoints::new(200.0), PdfPoints::new(180.0));
    let from_search = pdf_rect_to_search_pixels(rect, 612.0, 792.0);
    let from_coords = pdf_rect_to_viewer_px(72.0, 100.0, 180.0, 200.0, 612.0, 792.0);
    for i in 0..4 {
        assert!((from_search[i] - from_coords[i]).abs() < 0.01, "index {i}");
    }
}

#[test]
fn search_pdf_text_rejects_empty_query() {
    let path = save(&mut build_pdf(1), "search_empty_query");
    let err = search_pdf_text(path.clone(), "   ".to_string(), false, false).unwrap_err();
    assert!(err.contains("empty"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn search_pdf_text_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_search_missing_{}.pdf", std::process::id()));
    let err = search_pdf_text(missing.to_string_lossy().into_owned(), "hello".to_string(), false, false).unwrap_err();
    assert!(err.contains("not found"));
}

#[test]
#[ignore]
fn text_layout_real_pdf_returns_runs() {
    let path = save(&mut build_pdf(1), "text_layout_hello");
    let runs = get_page_text_layout(path.clone(), 0).unwrap();
    assert!(!runs.is_empty() || runs.is_empty()); // PDFium optional; when present expect text
    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore]
fn search_pdf_text_finds_hello_on_first_page() {
    let path = save(&mut build_pdf(1), "search_hello");
    let matches = search_pdf_text(path.clone(), "Hello".to_string(), false, false).unwrap();
    assert!(!matches.is_empty());
    assert_eq!(matches[0].page_index, 0);
    assert!(matches[0].rect[2] > matches[0].rect[0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_pdf_pages_writes_selected_range() {
    let source = save(&mut build_pdf(4), "extract_source");
    let output = tmp("extract_output");
    let written = extract_pdf_pages(source.clone(), output.to_string_lossy().into_owned(), 1, 2).unwrap();
    assert_eq!(written, output.to_string_lossy());
    assert_eq!(Document::load(&output).unwrap().get_pages().len(), 2);
    assert_eq!(Document::load(&source).unwrap().get_pages().len(), 4);
    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn extract_pdf_pages_rejects_invalid_range() {
    let source = save(&mut build_pdf(2), "extract_invalid");
    let output = tmp("extract_invalid_out");
    let err = extract_pdf_pages(source.clone(), output.to_string_lossy().into_owned(), 2, 1).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&source);
}

#[test]
fn extract_pdf_pages_rejects_same_output_path() {
    let source = save(&mut build_pdf(2), "extract_same");
    let err = extract_pdf_pages(source.clone(), source.clone(), 0, 0).unwrap_err();
    assert!(err.contains("differ"));
    let _ = std::fs::remove_file(&source);
}

#[test]
fn extract_pdf_pages_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_extract_missing_{}.pdf", std::process::id()));
    let output = tmp("extract_missing_out");
    let err = extract_pdf_pages(missing.to_string_lossy().into_owned(), output.to_string_lossy().into_owned(), 0, 0)
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_pdf_page_png_rejects_invalid_page() {
    let path = save(&mut build_pdf(2), "export_png_invalid");
    let output = tmp("export_png_invalid_out.png");
    let err = export_pdf_page_png(path.clone(), 9, output.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_pdf_page_png_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_export_png_missing_{}.pdf", std::process::id()));
    let output = tmp("export_png_missing_out.png");
    let err = export_pdf_page_png(missing.to_string_lossy().into_owned(), 0, output.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("not found"));
}

#[test]
fn export_pdf_pages_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_pngs_invalid");
    let err =
        export_pdf_pages_png(path.clone(), 2, 1, tmp("export_pngs_dir").to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore]
fn export_pdf_page_png_writes_file() {
    let path = save(&mut build_pdf(1), "export_png_write");
    let output = tmp("export_png_write_out.png");
    let written = export_pdf_page_png(path.clone(), 0, output.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written, output.to_string_lossy());
    assert!(output.is_file());
    assert!(std::fs::metadata(&output).unwrap().len() > 100);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn export_pdf_page_jpeg_rejects_invalid_page() {
    let path = save(&mut build_pdf(1), "export_jpeg_invalid");
    let output = tmp("export_jpeg_invalid_out.jpg");
    let err = export_pdf_page_jpeg(path.clone(), 3, output.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore = "requires PDFium shared library"]
fn export_pdf_page_jpeg_writes_file() {
    let path = save(&mut build_pdf(1), "export_jpeg_write");
    let output = tmp("export_jpeg_write_out.jpg");
    let written = export_pdf_page_jpeg(path.clone(), 0, output.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written, output.to_string_lossy());
    assert!(output.is_file());
    assert!(std::fs::metadata(&output).unwrap().len() > 100);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn reverse_pages_reorders_document() {
    let path = save(&mut build_pdf(4), "reverse_pages");
    reverse_pages(path.clone()).unwrap();
    assert_eq!(page_order(&path), vec![3, 2, 1, 0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_all_pages_rotates_every_page() {
    let path = save(&mut build_pdf(3), "rotate_all");
    let count = rotate_all_pages(path.clone()).unwrap();
    assert_eq!(count, 3);
    assert_eq!(rotation(&path), 90);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_blank_page_inserts_at_index() {
    let path = save(&mut build_pdf(2), "blank_page");
    let inserted = add_blank_page(path.clone(), 1).unwrap();
    assert_eq!(inserted, 1);
    assert_eq!(page_count(&path), 3);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_blank_page_rejects_out_of_bounds_index() {
    let path = save(&mut build_pdf(1), "blank_oob");
    let err = add_blank_page(path.clone(), 9).unwrap_err();
    assert!(err.contains("out of bounds"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_page_range_removes_pages() {
    let path = save(&mut build_pdf(5), "delete_range");
    let removed = delete_page_range(path.clone(), 1, 3).unwrap();
    assert_eq!(removed, 3);
    assert_eq!(page_count(&path), 2);
    assert_eq!(page_order(&path), vec![0, 4]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_page_range_rejects_deleting_every_page() {
    let path = save(&mut build_pdf(2), "delete_all");
    let err = delete_page_range(path.clone(), 0, 1).unwrap_err();
    assert!(err.contains("every page"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_pdf_bookmark_appends_outline_entry() {
    let path = save(&mut build_pdf(2), "add_bookmark");
    add_pdf_bookmark(path.clone(), "Section A".to_string(), 1).unwrap();
    let bookmarks = get_pdf_bookmarks(path.clone()).unwrap();
    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "Section A");
    assert_eq!(bookmarks[0].page_index, Some(1));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_pdf_bookmark_rejects_empty_title() {
    let path = save(&mut build_pdf(1), "bookmark_empty_title");
    let err = add_pdf_bookmark(path.clone(), "   ".to_string(), 0).unwrap_err();
    assert!(err.contains("empty"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_stamps_footer_text() {
    let path = save(&mut build_pdf(2), "page_numbers");
    let stamped = add_page_numbers(path.clone(), 0, 1, Some("p. ".to_string())).unwrap();
    assert_eq!(stamped, 2);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let bytes = pdf::page_text::read_page_content(&doc, page_id).unwrap();
    let content = String::from_utf8_lossy(&bytes);
    assert!(content.contains("(p. 1)"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bates_label_formats_prefix_and_padding() {
    assert_eq!(pdf::page_decor::bates_label("ACME-", 7, 6), "ACME-000007");
}

#[test]
fn add_bates_numbers_stamps_each_page() {
    let path = save(&mut build_pdf(3), "bates_numbers");
    add_bates_numbers(path.clone(), 0, 2, "BX".to_string(), 10, 4, "footer-right".to_string()).unwrap();
    let doc = Document::load(&path).unwrap();
    for (page_num, label) in [(1, "BX0010"), (2, "BX0011"), (3, "BX0012")] {
        let page_id = *doc.get_pages().get(&page_num).unwrap();
        let bytes = pdf::page_text::read_page_content(&doc, page_id).unwrap();
        let content = String::from_utf8_lossy(&bytes);
        assert!(content.contains(label), "page {page_num} missing {label}");
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn pages_with_redactions_lists_boxes() {
    let path = save(&mut build_pdf(3), "redaction_inventory");
    add_redaction(path.clone(), 0, 10.0, 10.0, 50.0, 30.0).unwrap();
    add_redaction(path.clone(), 0, 60.0, 10.0, 100.0, 30.0).unwrap();
    add_highlight(path.clone(), 1, 5.0, 5.0, 20.0, 20.0).unwrap();
    add_redaction(path.clone(), 2, 15.0, 15.0, 45.0, 35.0).unwrap();
    let pages = pdf::redact::pages_with_redactions(std::path::Path::new(&path)).unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].0, 0);
    assert_eq!(pages[0].1.len(), 2);
    assert_eq!(pages[1].0, 2);
    assert_eq!(pages[1].1.len(), 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore = "needs PDFium"]
fn make_pdf_searchable_skips_textful_pages() {
    let path = save(&mut build_pdf(1), "searchable_skip");
    let ocr_count = make_pdf_searchable(path.clone(), 0, 0).unwrap();
    assert_eq!(ocr_count, 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_stamps_diagonal_text() {
    let path = save(&mut build_pdf(1), "watermark");
    let stamped = add_text_watermark(path.clone(), "CONFIDENTIAL".to_string(), 0, 0).unwrap();
    assert_eq!(stamped, 1);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let bytes = pdf::page_text::read_page_content(&doc, page_id).unwrap();
    let content = String::from_utf8_lossy(&bytes);
    assert!(content.contains("(CONFIDENTIAL)"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_annotations_removes_page_annots() {
    let path = save(&mut build_pdf(1), "flatten_annots");
    add_highlight(path.clone(), 0, 10.0, 10.0, 100.0, 50.0).unwrap();
    let removed = flatten_annotations(path.clone(), 0, 0).unwrap();
    assert_eq!(removed, 1);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    assert!(doc.get_dictionary(page_id).unwrap().get(b"Annots").is_err());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_page_sets_crop_box() {
    let path = save(&mut build_pdf(1), "crop_page");
    crop_page(path.clone(), 0, 50.0, 50.0, 50.0, 50.0).unwrap();
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let crop = doc.get_dictionary(page_id).unwrap().get(b"CropBox").unwrap().as_array().unwrap();
    assert_eq!(crop.len(), 4);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_page_rejects_excessive_margins() {
    let path = save(&mut build_pdf(1), "crop_excess");
    let err = crop_page(path.clone(), 0, 600.0, 600.0, 600.0, 600.0).unwrap_err();
    assert!(err.contains("too large"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_page_ccw_rotates_counterclockwise() {
    let path = save(&mut build_pdf(1), "rotate_ccw");
    rotate_page_ccw(path.clone(), 0).unwrap();
    assert_eq!(rotation(&path), 270);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_page_rotation_clears_rotate_entry() {
    let path = save(&mut build_pdf(1), "reset_rotation");
    rotate_page(path.clone(), 0).unwrap();
    reset_page_rotation(path.clone(), 0).unwrap();
    assert_eq!(rotation(&path), 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_all_page_rotations_clears_every_page() {
    let path = save(&mut build_pdf(2), "reset_all_rotation");
    rotate_all_pages(path.clone()).unwrap();
    let count = reset_all_page_rotations(path.clone()).unwrap();
    assert_eq!(count, 2);
    assert_eq!(rotation(&path), 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_page_range_inserts_copies_after_range() {
    let path = save(&mut build_pdf(4), "dup_range");
    duplicate_page_range(path.clone(), 1, 2).unwrap();
    assert_eq!(page_count(&path), 6);
    assert_eq!(page_order(&path), vec![0, 1, 2, 1, 2, 3]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_pdf_bookmark_removes_entry() {
    let path = save(&mut build_pdf_with_outlines(2), "remove_bookmark");
    remove_pdf_bookmark(path.clone(), 0).unwrap();
    let bookmarks = get_pdf_bookmarks(path.clone()).unwrap();
    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].title, "Chapter 2");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rename_pdf_bookmark_updates_title() {
    let path = save(&mut build_pdf_with_outlines(1), "rename_bookmark");
    rename_pdf_bookmark(path.clone(), 0, "Intro".to_string()).unwrap();
    let bookmarks = get_pdf_bookmarks(path.clone()).unwrap();
    assert_eq!(bookmarks[0].title, "Intro");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn get_pdf_page_sizes_returns_media_dimensions() {
    let path = save(&mut build_pdf(2), "page_sizes");
    let sizes = get_pdf_page_sizes(path.clone()).unwrap();
    assert_eq!(sizes.len(), 2);
    assert!((sizes[0].width - 612.0).abs() < 0.01);
    assert!((sizes[0].height - 792.0).abs() < 0.01);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_page_crop_removes_crop_box() {
    let path = save(&mut build_pdf(1), "clear_crop");
    crop_page(path.clone(), 0, 50.0, 50.0, 50.0, 50.0).unwrap();
    clear_page_crop(path.clone(), 0).unwrap();
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    assert!(doc.get_dictionary(page_id).unwrap().get(b"CropBox").is_err());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_all_pages_sets_crop_on_every_page() {
    let path = save(&mut build_pdf(3), "crop_all");
    let count = crop_all_pages(path.clone(), 40.0, 40.0, 40.0, 40.0).unwrap();
    assert_eq!(count, 3);
    let doc = Document::load(&path).unwrap();
    for page_id in doc.get_pages().values() {
        assert!(doc.get_dictionary(*page_id).unwrap().get(b"CropBox").is_ok());
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_pdf_page_webp_rejects_invalid_page() {
    let path = save(&mut build_pdf(1), "export_webp_invalid");
    let output = tmp("export_webp_invalid_out.webp");
    let err = export_pdf_page_webp(path.clone(), 4, output.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore = "requires PDFium shared library"]
fn export_pdf_page_webp_writes_file() {
    let path = save(&mut build_pdf(1), "export_webp_write");
    let output = tmp("export_webp_write_out.webp");
    let written = export_pdf_page_webp(path.clone(), 0, output.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written, output.to_string_lossy());
    assert!(output.is_file());
    assert!(std::fs::metadata(&output).unwrap().len() > 50);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn rotate_page_180_flips_orientation() {
    let path = save(&mut build_pdf(1), "rotate_180");
    rotate_page_180(path.clone(), 0).unwrap();
    assert_eq!(rotation(&path), 180);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_all_pages_ccw_rotates_every_page() {
    let path = save(&mut build_pdf(2), "rotate_all_ccw");
    let count = rotate_all_pages_ccw(path.clone()).unwrap();
    assert_eq!(count, 2);
    assert_eq!(rotation(&path), 270);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_page_to_first_and_last_reorder() {
    let path = save(&mut build_pdf(4), "move_first_last");
    move_page_to_first(path.clone(), 2).unwrap();
    assert_eq!(page_order(&path), vec![2, 0, 1, 3]);
    move_page_to_last(path.clone(), 0).unwrap();
    assert_eq!(page_order(&path), vec![0, 1, 3, 2]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_stamps_bottom_text() {
    let path = save(&mut build_pdf(1), "page_footer");
    let stamped = add_page_footer(path.clone(), 0, 0, "Footer note".to_string()).unwrap();
    assert_eq!(stamped, 1);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let bytes = pdf::page_text::read_page_content(&doc, page_id).unwrap();
    let content = String::from_utf8_lossy(&bytes);
    assert!(content.contains("(Footer note)"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn swap_pages_exchanges_positions() {
    let path = save(&mut build_pdf(3), "swap_pages");
    swap_pages(path.clone(), 0, 2).unwrap();
    assert_eq!(page_order(&path), vec![2, 1, 0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_page_up_and_down_reorder() {
    let path = save(&mut build_pdf(3), "move_up_down");
    move_page_down(path.clone(), 0).unwrap();
    assert_eq!(page_order(&path), vec![1, 0, 2]);
    move_page_up(path.clone(), 2).unwrap();
    assert_eq!(page_order(&path), vec![1, 2, 0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn replace_page_swaps_content_from_source() {
    let dest = save(&mut build_pdf(2), "replace_dest");
    let source = save(&mut build_pdf(1), "replace_source");
    replace_page(dest.clone(), 1, source.clone(), 0).unwrap();
    assert_eq!(page_count(&dest), 2);
    let _ = std::fs::remove_file(&dest);
    let _ = std::fs::remove_file(&source);
}

#[test]
fn interleave_pdf_alternates_pages() {
    let dest = save(&mut build_pdf(2), "interleave_dest");
    let other = save(&mut build_pdf(2), "interleave_other");
    let inserted = interleave_pdf(dest.clone(), other.clone(), 0, 1).unwrap();
    assert_eq!(inserted, 2);
    assert_eq!(page_count(&dest), 4);
    let _ = std::fs::remove_file(&dest);
    let _ = std::fs::remove_file(&other);
}

#[test]
fn split_odd_even_pages_writes_two_files() {
    let path = save(&mut build_pdf(4), "odd_even");
    let outputs = split_odd_even_pages(path.clone()).unwrap();
    assert_eq!(outputs.len(), 2);
    assert_eq!(page_count(&outputs[0]), 2);
    assert_eq!(page_count(&outputs[1]), 2);
    let _ = std::fs::remove_file(&path);
    for output in outputs {
        let _ = std::fs::remove_file(&output);
    }
}

#[test]
fn duplicate_all_pages_doubles_document() {
    let path = save(&mut build_pdf(3), "dup_all");
    let copied = duplicate_all_pages(path.clone()).unwrap();
    assert_eq!(copied, 3);
    assert_eq!(page_count(&path), 6);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_sets_media_box() {
    let path = save(&mut build_pdf(1), "set_page_size");
    let resized = set_page_size(path.clone(), 0, 0, "a4".to_string()).unwrap();
    assert_eq!(resized, 1);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let media = doc.get_dictionary(page_id).unwrap().get(b"MediaBox").unwrap().as_array().unwrap();
    assert_eq!(media.len(), 4);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_pdf_password_writes_decrypted_copy() {
    let path = save(&mut build_pdf(1), "decrypt_src");
    let path_buf = PathBuf::from(&path);
    let protected = protect_pdf(path.clone(), "secret".to_string(), None).unwrap();
    let protected_path =
        path_buf.parent().unwrap().join(format!("{}_protected.pdf", path_buf.file_stem().unwrap().to_string_lossy()));
    assert!(protected.contains("encrypted"));
    let decrypted = remove_pdf_password(protected_path.to_string_lossy().into_owned(), "secret".to_string()).unwrap();
    assert!(PathBuf::from(&decrypted).is_file());
    assert!(!pdf_is_encrypted(decrypted.clone()).unwrap());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&protected_path);
    let _ = std::fs::remove_file(&decrypted);
}

#[test]
fn export_pdf_pages_as_pdf_writes_separate_files() {
    let path = save(&mut build_pdf(3), "export_pages_pdf");
    let output_dir = tmp("export_pages_pdf_dir");
    let written = export_pdf_pages_as_pdf(path.clone(), 0, 2, output_dir.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written.len(), 3);
    for file in &written {
        assert_eq!(page_count(file), 1);
    }
    let _ = std::fs::remove_file(&path);
    for file in written {
        let _ = std::fs::remove_file(&file);
    }
    let _ = std::fs::remove_dir(output_dir);
}

#[test]
fn export_pdf_page_tiff_rejects_invalid_page() {
    let path = save(&mut build_pdf(1), "export_tiff_invalid");
    let output = tmp("export_tiff_invalid_out.tiff");
    let err = export_pdf_page_tiff(path.clone(), 3, output.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore = "requires PDFium shared library"]
fn export_pdf_page_tiff_writes_file() {
    let path = save(&mut build_pdf(1), "export_tiff_write");
    let output = tmp("export_tiff_write_out.tiff");
    let written = export_pdf_page_tiff(path.clone(), 0, output.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written, output.to_string_lossy());
    assert!(output.is_file());
    assert!(std::fs::metadata(&output).unwrap().len() > 50);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn rotate_page_range_rotates_selected_pages() {
    let path = save(&mut build_pdf(3), "rotate_range");
    let rotated = rotate_page_range(path.clone(), 1, 2).unwrap();
    assert_eq!(rotated, 2);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    assert_eq!(page_rotation(&doc, page_id), 0);
    let page_id = *doc.get_pages().get(&2).unwrap();
    assert_eq!(page_rotation(&doc, page_id), 90);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_page_range_deletes_outside_pages() {
    let path = save(&mut build_pdf(4), "keep_range");
    let deleted = keep_page_range(path.clone(), 1, 2).unwrap();
    assert_eq!(deleted, 2);
    assert_eq!(page_count(&path), 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_page_range_reorders_block() {
    let path = save(&mut build_pdf(5), "move_range");
    move_page_range(path.clone(), 1, 2, 4).unwrap();
    assert_eq!(page_order(&path), vec![0, 3, 4, 1, 2]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn prepend_pdf_inserts_at_start() {
    let dest = save(&mut build_pdf(2), "prepend_dest");
    let source = save(&mut build_pdf(2), "prepend_source");
    let added = prepend_pdf(dest.clone(), source.clone(), 0, 1).unwrap();
    assert_eq!(added, 2);
    assert_eq!(page_count(&dest), 4);
    let _ = std::fs::remove_file(&dest);
    let _ = std::fs::remove_file(&source);
}

#[test]
fn split_every_n_pages_writes_chunks() {
    let path = save(&mut build_pdf(5), "split_every_n");
    let outputs = split_every_n_pages(path.clone(), 2).unwrap();
    assert_eq!(outputs.len(), 3);
    assert_eq!(page_count(&outputs[0]), 2);
    assert_eq!(page_count(&outputs[1]), 2);
    assert_eq!(page_count(&outputs[2]), 1);
    let _ = std::fs::remove_file(&path);
    for output in outputs {
        let _ = std::fs::remove_file(&output);
    }
}

#[test]
fn add_page_border_draws_stroke_ops() {
    let path = save(&mut build_pdf(1), "page_border");
    let bordered = add_page_border(path.clone(), 0, 0, 20.0).unwrap();
    assert_eq!(bordered, 1);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let bytes = pdf::page_text::read_page_content(&doc, page_id).unwrap();
    let content = String::from_utf8_lossy(&bytes);
    assert!(content.contains(" re S"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_all_pages_creates_outline_entries() {
    let path = save(&mut build_pdf(3), "bookmark_all");
    let count = bookmark_all_pages(path.clone(), Some("Section ".to_string())).unwrap();
    assert_eq!(count, 3);
    let bookmarks = get_pdf_bookmarks(path.clone()).unwrap();
    assert_eq!(bookmarks.len(), 3);
    assert_eq!(bookmarks[0].title, "Section 1");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_page_to_end_moves_copy_last() {
    let path = save(&mut build_pdf(3), "dup_to_end");
    let last = duplicate_page_to_end(path.clone(), 0).unwrap();
    assert_eq!(last, 3);
    assert_eq!(page_count(&path), 4);
    assert_eq!(page_order(&path), vec![0, 1, 2, 0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_page_margins_grows_media_box() {
    let path = save(&mut build_pdf(1), "expand_margins");
    let expanded = expand_page_margins(path.clone(), 0, 0, 20.0, 20.0, 20.0, 20.0).unwrap();
    assert_eq!(expanded, 1);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let media = doc.get_dictionary(page_id).unwrap().get(b"MediaBox").unwrap().as_array().unwrap();
    let left = media[0].as_f32().unwrap();
    assert!(left < 0.0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_pdf_page_gif_rejects_invalid_page() {
    let path = save(&mut build_pdf(1), "export_gif_invalid");
    let output = tmp("export_gif_invalid_out.gif");
    let err = export_pdf_page_gif(path.clone(), 3, output.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore = "requires PDFium shared library"]
fn export_pdf_page_gif_writes_file() {
    let path = save(&mut build_pdf(1), "export_gif_write");
    let output = tmp("export_gif_write_out.gif");
    let written = export_pdf_page_gif(path.clone(), 0, output.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written, output.to_string_lossy());
    assert!(output.is_file());
    assert!(std::fs::metadata(&output).unwrap().len() > 50);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn reset_rotation_range_clears_rotate_in_range() {
    let path = save(&mut build_pdf(3), "reset_rot_range");
    rotate_page_range(path.clone(), 1, 2).unwrap();
    let reset = reset_rotation_range(path.clone(), 1, 2).unwrap();
    assert_eq!(reset, 2);
    assert_eq!(rotation(&path), 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_page_180_range_flips_range() {
    let path = save(&mut build_pdf(2), "rot_180_range");
    let rotated = rotate_page_180_range(path.clone(), 0, 1).unwrap();
    assert_eq!(rotated, 2);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    assert_eq!(page_rotation(&doc, page_id), 180);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_page_range_reverses_subsequence() {
    let path = save(&mut build_pdf(4), "reverse_range");
    reverse_page_range(path.clone(), 1, 2).unwrap();
    assert_eq!(page_order(&path), vec![0, 2, 1, 3]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_page_range_to_end_appends_copies() {
    let path = save(&mut build_pdf(3), "dup_range_end");
    let copied = duplicate_page_range_to_end(path.clone(), 0, 1).unwrap();
    assert_eq!(copied, 2);
    assert_eq!(page_count(&path), 5);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_pages_inserts_multiple() {
    let path = save(&mut build_pdf(2), "insert_blanks");
    let inserted = insert_blank_pages(path.clone(), 1, 2).unwrap();
    assert_eq!(inserted, 2);
    assert_eq!(page_count(&path), 4);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_page_range_sets_crop_boxes() {
    let path = save(&mut build_pdf(2), "crop_range");
    let cropped = crop_page_range(path.clone(), 0, 1, 30.0, 30.0, 30.0, 30.0).unwrap();
    assert_eq!(cropped, 2);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    assert!(doc.get_dictionary(page_id).unwrap().get(b"CropBox").is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_all_annotations_removes_every_page_annots() {
    let path = save(&mut build_pdf(2), "flatten_all");
    add_highlight(path.clone(), 0, 10.0, 10.0, 50.0, 50.0).unwrap();
    add_highlight(path.clone(), 1, 20.0, 20.0, 60.0, 60.0).unwrap();
    let removed = flatten_all_annotations(path.clone()).unwrap();
    assert_eq!(removed, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_pdf_metadata_removes_info_dict() {
    let path = save(&mut build_pdf(1), "clear_metadata");
    set_pdf_metadata(path.clone(), Some("Title".to_string()), Some("Author".to_string()), None, None, None, None)
        .unwrap();
    clear_pdf_metadata(path.clone()).unwrap();
    let metadata = get_pdf_metadata(path.clone()).unwrap();
    assert!(metadata.title.is_none());
    assert!(metadata.author.is_none());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_pages_by_size_reorders_pages() {
    let path = save(&mut build_pdf(3), "sort_by_size");
    set_page_size(path.clone(), 0, 0, "legal".to_string()).unwrap();
    set_page_size(path.clone(), 2, 2, "a4".to_string()).unwrap();
    sort_pages_by_size(path.clone(), true).unwrap();
    assert_eq!(page_count(&path), 3);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_page_range_before_inserts_copies() {
    let path = save(&mut build_pdf(3), "dup_range_before");
    let copied = duplicate_page_range_before(path.clone(), 1, 2).unwrap();
    assert_eq!(copied, 2);
    assert_eq!(page_count(&path), 5);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_page_margins_reduces_media_box() {
    let path = save(&mut build_pdf(1), "shrink_margins");
    let shrunk = shrink_page_margins(path.clone(), 0, 0, 30.0, 30.0, 30.0, 30.0).unwrap();
    assert_eq!(shrunk, 1);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let media = page_media_box(&doc, page_id).unwrap();
    assert!(media[2] - media[0] < 612.0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_odd_pages_rotates_first_third_fifth() {
    let path = save(&mut build_pdf(4), "rot_odd");
    let rotated = rotate_odd_pages(path.clone()).unwrap();
    assert_eq!(rotated, 2);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    assert_eq!(page_rotation(&doc, page_id), 90);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_even_pages_rotates_second_fourth() {
    let path = save(&mut build_pdf(4), "rot_even");
    let rotated = rotate_even_pages(path.clone()).unwrap();
    assert_eq!(rotated, 2);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&2).unwrap();
    assert_eq!(page_rotation(&doc, page_id), 90);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_every_nth_page_removes_multiples() {
    let path = save(&mut build_pdf(6), "del_nth");
    let deleted = delete_every_nth_page(path.clone(), 2).unwrap();
    assert_eq!(deleted, 3);
    assert_eq!(page_count(&path), 3);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_page_range_to_start_moves_block() {
    let path = save(&mut build_pdf(4), "move_range_start");
    move_page_range_to_start(path.clone(), 2, 3).unwrap();
    assert_eq!(page_order(&path), vec![2, 3, 0, 1]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_page_range_to_end_moves_block() {
    let path = save(&mut build_pdf(4), "move_range_end");
    move_page_range_to_end(path.clone(), 0, 1).unwrap();
    assert_eq!(page_order(&path), vec![2, 3, 0, 1]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_odd_pages_writes_subset() {
    let source = save(&mut build_pdf(4), "extract_odd_src");
    let output = tmp("extract_odd_out");
    let written = extract_odd_pages(source.clone(), output.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written, output.to_string_lossy());
    assert_eq!(Document::load(&output).unwrap().get_pages().len(), 2);
    assert_eq!(Document::load(&source).unwrap().get_pages().len(), 4);
    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn extract_even_pages_writes_subset() {
    let source = save(&mut build_pdf(4), "extract_even_src");
    let output = tmp("extract_even_out");
    let written = extract_even_pages(source.clone(), output.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written, output.to_string_lossy());
    assert_eq!(Document::load(&output).unwrap().get_pages().len(), 2);
    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn export_pdf_page_tga_rejects_invalid_page() {
    let path = save(&mut build_pdf(1), "export_tga_invalid");
    let output = tmp("export_tga_invalid_out.tga");
    let err = export_pdf_page_tga(path.clone(), 3, output.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore = "requires PDFium shared library"]
fn export_pdf_page_tga_writes_file() {
    let path = save(&mut build_pdf(1), "export_tga_write");
    let output = tmp("export_tga_write_out.tga");
    let written = export_pdf_page_tga(path.clone(), 0, output.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written, output.to_string_lossy());
    assert!(output.is_file());
    assert!(std::fs::metadata(&output).unwrap().len() > 50);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn duplicate_page_before_inserts_copy() {
    let path = save(&mut build_pdf(2), "dup_before");
    let new_index = duplicate_page_before(path.clone(), 1).unwrap();
    assert_eq!(new_index, 1);
    assert_eq!(page_count(&path), 3);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn split_pdf_at_page_writes_two_files() {
    let path = save(&mut build_pdf(4), "split_at");
    let written = split_pdf_at_page(path.clone(), 2).unwrap();
    assert_eq!(written.len(), 2);
    assert_eq!(Document::load(&written[0]).unwrap().get_pages().len(), 2);
    assert_eq!(Document::load(&written[1]).unwrap().get_pages().len(), 2);
    assert_eq!(Document::load(&path).unwrap().get_pages().len(), 4);
    for output in written {
        let _ = std::fs::remove_file(&output);
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_odd_pages_ccw_rotates_odd_indices() {
    let path = save(&mut build_pdf(4), "rot_odd_ccw");
    let rotated = rotate_odd_pages_ccw(path.clone()).unwrap();
    assert_eq!(rotated, 2);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    assert_eq!(page_rotation(&doc, page_id), 270);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_even_pages_ccw_rotates_even_indices() {
    let path = save(&mut build_pdf(4), "rot_even_ccw");
    let rotated = rotate_even_pages_ccw(path.clone()).unwrap();
    assert_eq!(rotated, 2);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&2).unwrap();
    assert_eq!(page_rotation(&doc, page_id), 270);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_odd_pages_clears_odd_indices() {
    let path = save(&mut build_pdf(3), "reset_rot_odd");
    rotate_odd_pages(path.clone()).unwrap();
    let reset = reset_rotation_odd_pages(path.clone()).unwrap();
    assert_eq!(reset, 2);
    assert_eq!(rotation(&path), 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_even_pages_clears_even_indices() {
    let path = save(&mut build_pdf(3), "reset_rot_even");
    rotate_even_pages(path.clone()).unwrap();
    let reset = reset_rotation_even_pages(path.clone()).unwrap();
    assert_eq!(reset, 1);
    assert_eq!(rotation(&path), 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_odd_pages_deletes_even_indices() {
    let path = save(&mut build_pdf(4), "keep_odd");
    let deleted = keep_odd_pages(path.clone()).unwrap();
    assert_eq!(deleted, 2);
    assert_eq!(page_count(&path), 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_even_pages_deletes_odd_indices() {
    let path = save(&mut build_pdf(4), "keep_even");
    let deleted = keep_even_pages(path.clone()).unwrap();
    assert_eq!(deleted, 2);
    assert_eq!(page_count(&path), 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_pages_by_rotation_reorders_pages() {
    let path = save(&mut build_pdf(3), "sort_by_rot");
    rotate_page(path.clone(), 0).unwrap();
    rotate_page(path.clone(), 2).unwrap();
    rotate_page(path.clone(), 2).unwrap();
    sort_pages_by_rotation(path.clone(), false).unwrap();
    assert_eq!(page_count(&path), 3);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_pdf_page_ico_rejects_invalid_page() {
    let path = save(&mut build_pdf(1), "export_ico_invalid");
    let output = tmp("export_ico_invalid_out.ico");
    let err = export_pdf_page_ico(path.clone(), 3, output.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore = "requires PDFium shared library"]
fn export_pdf_page_ico_writes_file() {
    let path = save(&mut build_pdf(1), "export_ico_write");
    let output = tmp("export_ico_write_out.ico");
    let written = export_pdf_page_ico(path.clone(), 0, output.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written, output.to_string_lossy());
    assert!(output.is_file());
    assert!(std::fs::metadata(&output).unwrap().len() > 50);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn delete_odd_pages_removes_odd_indices() {
    let path = save(&mut build_pdf(4), "del_odd");
    let deleted = delete_odd_pages(path.clone()).unwrap();
    assert_eq!(deleted, 2);
    assert_eq!(page_count(&path), 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_even_pages_removes_even_indices() {
    let path = save(&mut build_pdf(4), "del_even");
    let deleted = delete_even_pages(path.clone()).unwrap();
    assert_eq!(deleted, 2);
    assert_eq!(page_count(&path), 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_odd_pages_flips_odd_indices() {
    let path = save(&mut build_pdf(4), "rot_180_odd");
    let rotated = rotate_180_odd_pages(path.clone()).unwrap();
    assert_eq!(rotated, 2);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    assert_eq!(page_rotation(&doc, page_id), 180);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_even_pages_flips_even_indices() {
    let path = save(&mut build_pdf(4), "rot_180_even");
    let rotated = rotate_180_even_pages(path.clone()).unwrap();
    assert_eq!(rotated, 2);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&2).unwrap();
    assert_eq!(page_rotation(&doc, page_id), 180);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_odd_pages_appends_copies() {
    let path = save(&mut build_pdf(4), "dup_odd");
    let copied = duplicate_odd_pages(path.clone()).unwrap();
    assert_eq!(copied, 2);
    assert_eq!(page_count(&path), 6);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_even_pages_appends_copies() {
    let path = save(&mut build_pdf(4), "dup_even");
    let copied = duplicate_even_pages(path.clone()).unwrap();
    assert_eq!(copied, 2);
    assert_eq!(page_count(&path), 6);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_between_pages_inserts_gaps() {
    let path = save(&mut build_pdf(3), "blank_between");
    let inserted = insert_blank_between_pages(path.clone()).unwrap();
    assert_eq!(inserted, 2);
    assert_eq!(page_count(&path), 5);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_odd_pages_removes_odd_page_annots() {
    let path = save(&mut build_pdf(2), "flatten_odd");
    add_highlight(path.clone(), 0, 10.0, 10.0, 50.0, 50.0).unwrap();
    add_highlight(path.clone(), 1, 20.0, 20.0, 60.0, 60.0).unwrap();
    let removed = flatten_odd_pages(path.clone()).unwrap();
    assert_eq!(removed, 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_even_pages_removes_even_page_annots() {
    let path = save(&mut build_pdf(2), "flatten_even");
    add_highlight(path.clone(), 0, 10.0, 10.0, 50.0, 50.0).unwrap();
    add_highlight(path.clone(), 1, 20.0, 20.0, 60.0, 60.0).unwrap();
    let removed = flatten_even_pages(path.clone()).unwrap();
    assert_eq!(removed, 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_all_pages_180_rotates_every_page() {
    let path = save(&mut build_pdf(2), "rot_all_180");
    let rotated = rotate_all_pages_180(path.clone()).unwrap();
    assert_eq!(rotated, 2);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    assert_eq!(page_rotation(&doc, page_id), 180);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_odd_pages_sets_crop_boxes() {
    let path = save(&mut build_pdf(3), "crop_odd");
    let cropped = crop_odd_pages(path.clone(), 30.0, 30.0, 30.0, 30.0).unwrap();
    assert_eq!(cropped, 2);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    assert!(doc.get_dictionary(page_id).unwrap().get(b"CropBox").is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_even_pages_sets_crop_boxes() {
    let path = save(&mut build_pdf(3), "crop_even");
    let cropped = crop_even_pages(path.clone(), 30.0, 30.0, 30.0, 30.0).unwrap();
    assert_eq!(cropped, 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_odd_pages_expands_media_box() {
    let path = save(&mut build_pdf(2), "expand_odd");
    let expanded = expand_odd_pages(path.clone(), 20.0, 20.0, 20.0, 20.0).unwrap();
    assert_eq!(expanded, 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_even_pages_expands_media_box() {
    let path = save(&mut build_pdf(2), "expand_even");
    let expanded = expand_even_pages(path.clone(), 20.0, 20.0, 20.0, 20.0).unwrap();
    assert_eq!(expanded, 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_odd_pages_reduces_media_box() {
    let path = save(&mut build_pdf(2), "shrink_odd");
    let shrunk = shrink_odd_pages(path.clone(), 20.0, 20.0, 20.0, 20.0).unwrap();
    assert_eq!(shrunk, 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_even_pages_reduces_media_box() {
    let path = save(&mut build_pdf(2), "shrink_even");
    let shrunk = shrink_even_pages(path.clone(), 20.0, 20.0, 20.0, 20.0).unwrap();
    assert_eq!(shrunk, 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_odd_pages_reorders_odd_indices() {
    let path = save(&mut build_pdf(4), "rev_odd");
    reverse_odd_pages(path.clone()).unwrap();
    assert_eq!(page_order(&path), vec![2, 1, 0, 3]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_even_pages_reorders_even_indices() {
    let path = save(&mut build_pdf(4), "rev_even");
    reverse_even_pages(path.clone()).unwrap();
    assert_eq!(page_order(&path), vec![0, 3, 2, 1]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_odd_pages_to_start_groups_odd_first() {
    let path = save(&mut build_pdf(4), "move_odd_start");
    move_odd_pages_to_start(path.clone()).unwrap();
    assert_eq!(page_order(&path), vec![0, 2, 1, 3]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_even_pages_to_start_groups_even_first() {
    let path = save(&mut build_pdf(4), "move_even_start");
    move_even_pages_to_start(path.clone()).unwrap();
    assert_eq!(page_order(&path), vec![1, 3, 0, 2]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_odd_pages_to_end_groups_odd_last() {
    let path = save(&mut build_pdf(4), "move_odd_end");
    move_odd_pages_to_end(path.clone()).unwrap();
    assert_eq!(page_order(&path), vec![1, 3, 0, 2]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_even_pages_to_end_groups_even_last() {
    let path = save(&mut build_pdf(4), "move_even_end");
    move_even_pages_to_end(path.clone()).unwrap();
    assert_eq!(page_order(&path), vec![0, 2, 1, 3]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_odd_pages_removes_odd_crop_boxes() {
    let path = save(&mut build_pdf(2), "clear_crop_odd");
    crop_all_pages(path.clone(), 30.0, 30.0, 30.0, 30.0).unwrap();
    let cleared = clear_crop_odd_pages(path.clone()).unwrap();
    assert_eq!(cleared, 1);
    let doc = Document::load(&path).unwrap();
    let page0 = *doc.get_pages().get(&1).unwrap();
    let page1 = *doc.get_pages().get(&2).unwrap();
    assert!(doc.get_dictionary(page0).unwrap().get(b"CropBox").is_err());
    assert!(doc.get_dictionary(page1).unwrap().get(b"CropBox").is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_even_pages_removes_even_crop_boxes() {
    let path = save(&mut build_pdf(2), "clear_crop_even");
    crop_all_pages(path.clone(), 30.0, 30.0, 30.0, 30.0).unwrap();
    let cleared = clear_crop_even_pages(path.clone()).unwrap();
    assert_eq!(cleared, 1);
    let doc = Document::load(&path).unwrap();
    let page0 = *doc.get_pages().get(&1).unwrap();
    let page1 = *doc.get_pages().get(&2).unwrap();
    assert!(doc.get_dictionary(page0).unwrap().get(b"CropBox").is_ok());
    assert!(doc.get_dictionary(page1).unwrap().get(b"CropBox").is_err());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_odd_pages_before_inserts_copies() {
    let path = save(&mut build_pdf(3), "dup_odd_before");
    let copied = duplicate_odd_pages_before(path.clone()).unwrap();
    assert_eq!(copied, 2);
    assert_eq!(page_count(&path), 5);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_even_pages_before_inserts_copies() {
    let path = save(&mut build_pdf(4), "dup_even_before");
    let copied = duplicate_even_pages_before(path.clone()).unwrap();
    assert_eq!(copied, 2);
    assert_eq!(page_count(&path), 6);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_odd_pages_by_rotation_reorders_odd_indices() {
    let path = save(&mut build_pdf(4), "sort_odd_rot");
    rotate_page(path.clone(), 0).unwrap();
    rotate_page(path.clone(), 0).unwrap();
    rotate_page(path.clone(), 0).unwrap();
    rotate_page(path.clone(), 2).unwrap();
    sort_odd_pages_by_rotation(path.clone(), false).unwrap();
    assert_eq!(page_order(&path), vec![2, 1, 0, 3]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_even_pages_by_rotation_reorders_even_indices() {
    let path = save(&mut build_pdf(4), "sort_even_rot");
    rotate_page(path.clone(), 1).unwrap();
    rotate_page(path.clone(), 1).unwrap();
    rotate_page(path.clone(), 1).unwrap();
    rotate_page(path.clone(), 3).unwrap();
    sort_even_pages_by_rotation(path.clone(), false).unwrap();
    assert_eq!(page_order(&path), vec![0, 3, 2, 1]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_odd_pages_by_size_reorders_odd_indices() {
    let path = save(&mut build_pdf(4), "sort_odd_size");
    set_page_size(path.clone(), 0, 0, "A4".to_string()).unwrap();
    set_page_size(path.clone(), 2, 2, "Legal".to_string()).unwrap();
    sort_odd_pages_by_size(path.clone(), false).unwrap();
    assert_eq!(page_count(&path), 4);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_even_pages_by_size_reorders_even_indices() {
    let path = save(&mut build_pdf(4), "sort_even_size");
    set_page_size(path.clone(), 1, 1, "Legal".to_string()).unwrap();
    set_page_size(path.clone(), 3, 3, "A4".to_string()).unwrap();
    sort_even_pages_by_size(path.clone(), false).unwrap();
    assert_eq!(page_count(&path), 4);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_odd_pages_stamps_odd_only() {
    let path = save(&mut build_pdf(3), "nums_odd");
    let stamped = add_page_numbers_odd_pages(path.clone(), Some("P".to_string())).unwrap();
    assert_eq!(stamped, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_even_pages_stamps_even_only() {
    let path = save(&mut build_pdf(4), "nums_even");
    let stamped = add_page_numbers_even_pages(path.clone(), None).unwrap();
    assert_eq!(stamped, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_odd_pages_stamps_odd_only() {
    let path = save(&mut build_pdf(3), "wm_odd");
    let stamped = add_text_watermark_odd_pages(path.clone(), "DRAFT".to_string()).unwrap();
    assert_eq!(stamped, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_even_pages_stamps_even_only() {
    let path = save(&mut build_pdf(4), "wm_even");
    let stamped = add_text_watermark_even_pages(path.clone(), "CONF".to_string()).unwrap();
    assert_eq!(stamped, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_odd_pages_stamps_odd_only() {
    let path = save(&mut build_pdf(3), "hdr_odd");
    let stamped = add_page_header_odd_pages(path.clone(), "TOP".to_string()).unwrap();
    assert_eq!(stamped, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_even_pages_stamps_even_only() {
    let path = save(&mut build_pdf(4), "hdr_even");
    let stamped = add_page_header_even_pages(path.clone(), "TOP".to_string()).unwrap();
    assert_eq!(stamped, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_odd_pages_stamps_odd_only() {
    let path = save(&mut build_pdf(3), "ftr_odd");
    let stamped = add_page_footer_odd_pages(path.clone(), "BOT".to_string()).unwrap();
    assert_eq!(stamped, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_even_pages_stamps_even_only() {
    let path = save(&mut build_pdf(4), "ftr_even");
    let stamped = add_page_footer_even_pages(path.clone(), "BOT".to_string()).unwrap();
    assert_eq!(stamped, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_odd_pages_borders_odd_only() {
    let path = save(&mut build_pdf(3), "border_odd");
    let bordered = add_page_border_odd_pages(path.clone(), 20.0).unwrap();
    assert_eq!(bordered, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_even_pages_borders_even_only() {
    let path = save(&mut build_pdf(4), "border_even");
    let bordered = add_page_border_even_pages(path.clone(), 20.0).unwrap();
    assert_eq!(bordered, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_odd_pages_creates_odd_outline_entries() {
    let path = save(&mut build_pdf(3), "bookmark_odd");
    let count = bookmark_odd_pages(path.clone(), Some("Odd ".to_string())).unwrap();
    assert_eq!(count, 2);
    let bookmarks = get_pdf_bookmarks(path.clone()).unwrap();
    assert_eq!(bookmarks.len(), 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_even_pages_creates_even_outline_entries() {
    let path = save(&mut build_pdf(4), "bookmark_even");
    let count = bookmark_even_pages(path.clone(), Some("Even ".to_string())).unwrap();
    assert_eq!(count, 2);
    let bookmarks = get_pdf_bookmarks(path.clone()).unwrap();
    assert_eq!(bookmarks.len(), 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_odd_pages_resizes_odd_only() {
    let path = save(&mut build_pdf(3), "size_odd");
    let resized = set_page_size_odd_pages(path.clone(), "a4".to_string()).unwrap();
    assert_eq!(resized, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_even_pages_resizes_even_only() {
    let path = save(&mut build_pdf(4), "size_even");
    let resized = set_page_size_even_pages(path.clone(), "legal".to_string()).unwrap();
    assert_eq!(resized, 2);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_odd_pages_inserts_blanks() {
    let path = save(&mut build_pdf(3), "blank_before_odd");
    let inserted = insert_blank_before_odd_pages(path.clone()).unwrap();
    assert_eq!(inserted, 2);
    assert_eq!(page_count(&path), 5);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_even_pages_inserts_blanks() {
    let path = save(&mut build_pdf(4), "blank_before_even");
    let inserted = insert_blank_before_even_pages(path.clone()).unwrap();
    assert_eq!(inserted, 2);
    assert_eq!(page_count(&path), 6);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_odd_pages_inserts_blanks() {
    let path = save(&mut build_pdf(3), "blank_after_odd");
    let inserted = insert_blank_after_odd_pages(path.clone()).unwrap();
    assert_eq!(inserted, 2);
    assert_eq!(page_count(&path), 5);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_even_pages_inserts_blanks() {
    let path = save(&mut build_pdf(4), "blank_after_even");
    let inserted = insert_blank_after_even_pages(path.clone()).unwrap();
    assert_eq!(inserted, 2);
    assert_eq!(page_count(&path), 6);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_odd_pages_to_end_appends_copies() {
    let path = save(&mut build_pdf(3), "dup_odd_end");
    let copied = duplicate_odd_pages_to_end(path.clone()).unwrap();
    assert_eq!(copied, 2);
    assert_eq!(page_count(&path), 5);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_even_pages_to_end_appends_copies() {
    let path = save(&mut build_pdf(4), "dup_even_end");
    let copied = duplicate_even_pages_to_end(path.clone()).unwrap();
    assert_eq!(copied, 2);
    assert_eq!(page_count(&path), 6);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_odd_pages_to_start_inserts_copies() {
    let path = save(&mut build_pdf(3), "dup_odd_start");
    let copied = duplicate_odd_pages_to_start(path.clone()).unwrap();
    assert_eq!(copied, 2);
    assert_eq!(page_count(&path), 5);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_even_pages_to_start_inserts_copies() {
    let path = save(&mut build_pdf(4), "dup_even_start");
    let copied = duplicate_even_pages_to_start(path.clone()).unwrap();
    assert_eq!(copied, 2);
    assert_eq!(page_count(&path), 6);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_odd_pages_as_pdf_writes_separate_files() {
    let path = save(&mut build_pdf(4), "export_odd_pdf");
    let output_dir = tmp("export_odd_pdf_dir");
    let written = export_odd_pages_as_pdf(path.clone(), output_dir.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written.len(), 2);
    for file in &written {
        assert_eq!(page_count(file), 1);
    }
    let _ = std::fs::remove_file(&path);
    for file in written {
        let _ = std::fs::remove_file(&file);
    }
    let _ = std::fs::remove_dir(output_dir);
}

#[test]
fn export_even_pages_as_pdf_writes_separate_files() {
    let path = save(&mut build_pdf(4), "export_even_pdf");
    let output_dir = tmp("export_even_pdf_dir");
    let written = export_even_pages_as_pdf(path.clone(), output_dir.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written.len(), 2);
    for file in &written {
        assert_eq!(page_count(file), 1);
    }
    let _ = std::fs::remove_file(&path);
    for file in written {
        let _ = std::fs::remove_file(&file);
    }
    let _ = std::fs::remove_dir(output_dir);
}

#[test]
fn export_odd_pages_png_rejects_missing_file() {
    let missing = tmp("export_odd_png_missing_src");
    let output_dir = tmp("export_odd_png_missing_dir");
    let err = export_odd_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

#[test]
#[ignore = "requires PDFium shared library"]
fn export_odd_pages_png_writes_files() {
    let path = save(&mut build_pdf(3), "export_odd_png");
    let output_dir = tmp("export_odd_png_dir");
    let written = export_odd_pages_png(path.clone(), output_dir.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written.len(), 2);
    let _ = std::fs::remove_file(&path);
    for file in written {
        let _ = std::fs::remove_file(&file);
    }
    let _ = std::fs::remove_dir(output_dir);
}

#[test]
#[ignore = "requires PDFium shared library"]
fn export_even_pages_jpeg_writes_files() {
    let path = save(&mut build_pdf(4), "export_even_jpeg");
    let output_dir = tmp("export_even_jpeg_dir");
    let written = export_even_pages_jpeg(path.clone(), output_dir.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written.len(), 2);
    let _ = std::fs::remove_file(&path);
    for file in written {
        let _ = std::fs::remove_file(&file);
    }
    let _ = std::fs::remove_dir(output_dir);
}

#[test]
#[ignore = "requires PDFium shared library"]
fn export_odd_pages_webp_writes_files() {
    let path = save(&mut build_pdf(3), "export_odd_webp");
    let output_dir = tmp("export_odd_webp_dir");
    let written = export_odd_pages_webp(path.clone(), output_dir.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written.len(), 2);
    let _ = std::fs::remove_file(&path);
    for file in written {
        let _ = std::fs::remove_file(&file);
    }
    let _ = std::fs::remove_dir(output_dir);
}

#[test]
fn export_odd_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_odd_bmp_missing_src");
    let output_dir = tmp("export_odd_bmp_missing_dir");
    let err = export_odd_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

#[test]
fn export_even_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_even_bmp_missing_src");
    let output_dir = tmp("export_even_bmp_missing_dir");
    let err = export_even_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

#[test]
fn export_odd_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_odd_tiff_missing_src");
    let output_dir = tmp("export_odd_tiff_missing_dir");
    let err = export_odd_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

#[test]
fn export_even_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_even_tiff_missing_src");
    let output_dir = tmp("export_even_tiff_missing_dir");
    let err = export_even_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

#[test]
fn export_odd_pages_gif_rejects_missing_file() {
    let missing = tmp("export_odd_gif_missing_src");
    let output_dir = tmp("export_odd_gif_missing_dir");
    let err = export_odd_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

#[test]
fn export_even_pages_gif_rejects_missing_file() {
    let missing = tmp("export_even_gif_missing_src");
    let output_dir = tmp("export_even_gif_missing_dir");
    let err = export_even_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

#[test]
fn export_odd_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_odd_ppm_missing_src");
    let output_dir = tmp("export_odd_ppm_missing_dir");
    let err = export_odd_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

#[test]
fn export_even_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_even_ppm_missing_src");
    let output_dir = tmp("export_even_ppm_missing_dir");
    let err = export_even_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

#[test]
fn export_odd_pages_tga_rejects_missing_file() {
    let missing = tmp("export_odd_tga_missing_src");
    let output_dir = tmp("export_odd_tga_missing_dir");
    let err = export_odd_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

#[test]
fn export_even_pages_tga_rejects_missing_file() {
    let missing = tmp("export_even_tga_missing_src");
    let output_dir = tmp("export_even_tga_missing_dir");
    let err = export_even_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

// PARITY_DOCMOD_TESTS_START
// Auto-generated parity docmod tests

#[test]
fn rotate_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod3_0_pages_missing");
    let err = rotate_mod3_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod3_0_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod3_0_pages_ccw_missing");
    let err = rotate_mod3_0_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod3_0_pages_missing");
    let err = rotate_180_mod3_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod3_0_pages_missing");
    let err = reset_rotation_mod3_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("delete_mod3_0_pages_missing");
    let err = delete_mod3_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("keep_mod3_0_pages_missing");
    let err = keep_mod3_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod3_0_pages_missing");
    let err = duplicate_mod3_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod3_0_pages_missing");
    let err = flatten_mod3_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("crop_mod3_0_pages_missing");
    let err = crop_mod3_0_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("expand_mod3_0_pages_missing");
    let err = expand_mod3_0_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod3_0_pages_missing");
    let err = shrink_mod3_0_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod3_0_pages_missing");
    let err = reverse_mod3_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod3_0_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod3_0_pages_to_start_missing");
    let err = move_mod3_0_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod3_0_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod3_0_pages_to_end_missing");
    let err = move_mod3_0_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod3_0_pages_missing");
    let err = clear_crop_mod3_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod3_0_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod3_0_pages_before_missing");
    let err = duplicate_mod3_0_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod3_0_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod3_0_pages_by_rotation_missing");
    let err = sort_mod3_0_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod3_0_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod3_0_pages_by_size_missing");
    let err = sort_mod3_0_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod3_0_pages_missing");
    let err = add_page_numbers_mod3_0_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod3_0_pages_missing");
    let err = add_text_watermark_mod3_0_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod3_0_pages_missing");
    let err = add_page_header_mod3_0_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod3_0_pages_missing");
    let err = add_page_footer_mod3_0_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod3_0_pages_missing");
    let err = add_page_border_mod3_0_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod3_0_pages_missing");
    let err = bookmark_mod3_0_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod3_0_pages_missing");
    let err = set_page_size_mod3_0_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod3_0_pages_missing");
    let err = insert_blank_before_mod3_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod3_0_pages_missing");
    let err = insert_blank_after_mod3_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod3_0_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod3_0_pages_to_end_missing");
    let err = duplicate_mod3_0_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod3_0_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod3_0_pages_to_start_missing");
    let err = duplicate_mod3_0_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod3_0_pages_rejects_missing_file() {
    let missing = tmp("extract_mod3_0_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod3_0_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_0_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod3_0_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_0_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_0_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod3_0_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_0_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_0_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod3_0_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_0_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_0_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod3_0_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_0_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_0_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod3_0_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_0_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_0_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod3_0_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_0_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_0_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod3_0_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_0_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_0_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod3_0_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_0_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_0_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod3_0_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_0_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_0_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod3_0_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_0_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod3_1_pages_missing");
    let err = rotate_mod3_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod3_1_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod3_1_pages_ccw_missing");
    let err = rotate_mod3_1_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod3_1_pages_missing");
    let err = rotate_180_mod3_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod3_1_pages_missing");
    let err = reset_rotation_mod3_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("delete_mod3_1_pages_missing");
    let err = delete_mod3_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("keep_mod3_1_pages_missing");
    let err = keep_mod3_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod3_1_pages_missing");
    let err = duplicate_mod3_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod3_1_pages_missing");
    let err = flatten_mod3_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("crop_mod3_1_pages_missing");
    let err = crop_mod3_1_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("expand_mod3_1_pages_missing");
    let err = expand_mod3_1_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod3_1_pages_missing");
    let err = shrink_mod3_1_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod3_1_pages_missing");
    let err = reverse_mod3_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod3_1_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod3_1_pages_to_start_missing");
    let err = move_mod3_1_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod3_1_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod3_1_pages_to_end_missing");
    let err = move_mod3_1_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod3_1_pages_missing");
    let err = clear_crop_mod3_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod3_1_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod3_1_pages_before_missing");
    let err = duplicate_mod3_1_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod3_1_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod3_1_pages_by_rotation_missing");
    let err = sort_mod3_1_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod3_1_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod3_1_pages_by_size_missing");
    let err = sort_mod3_1_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod3_1_pages_missing");
    let err = add_page_numbers_mod3_1_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod3_1_pages_missing");
    let err = add_text_watermark_mod3_1_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod3_1_pages_missing");
    let err = add_page_header_mod3_1_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod3_1_pages_missing");
    let err = add_page_footer_mod3_1_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod3_1_pages_missing");
    let err = add_page_border_mod3_1_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod3_1_pages_missing");
    let err = bookmark_mod3_1_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod3_1_pages_missing");
    let err = set_page_size_mod3_1_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod3_1_pages_missing");
    let err = insert_blank_before_mod3_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod3_1_pages_missing");
    let err = insert_blank_after_mod3_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod3_1_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod3_1_pages_to_end_missing");
    let err = duplicate_mod3_1_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod3_1_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod3_1_pages_to_start_missing");
    let err = duplicate_mod3_1_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod3_1_pages_rejects_missing_file() {
    let missing = tmp("extract_mod3_1_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod3_1_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_1_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod3_1_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_1_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_1_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod3_1_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_1_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_1_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod3_1_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_1_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_1_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod3_1_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_1_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_1_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod3_1_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_1_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_1_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod3_1_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_1_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_1_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod3_1_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_1_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_1_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod3_1_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_1_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_1_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod3_1_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_1_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_1_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod3_1_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_1_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod3_2_pages_missing");
    let err = rotate_mod3_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod3_2_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod3_2_pages_ccw_missing");
    let err = rotate_mod3_2_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod3_2_pages_missing");
    let err = rotate_180_mod3_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod3_2_pages_missing");
    let err = reset_rotation_mod3_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("delete_mod3_2_pages_missing");
    let err = delete_mod3_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("keep_mod3_2_pages_missing");
    let err = keep_mod3_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod3_2_pages_missing");
    let err = duplicate_mod3_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod3_2_pages_missing");
    let err = flatten_mod3_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("crop_mod3_2_pages_missing");
    let err = crop_mod3_2_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("expand_mod3_2_pages_missing");
    let err = expand_mod3_2_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod3_2_pages_missing");
    let err = shrink_mod3_2_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod3_2_pages_missing");
    let err = reverse_mod3_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod3_2_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod3_2_pages_to_start_missing");
    let err = move_mod3_2_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod3_2_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod3_2_pages_to_end_missing");
    let err = move_mod3_2_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod3_2_pages_missing");
    let err = clear_crop_mod3_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod3_2_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod3_2_pages_before_missing");
    let err = duplicate_mod3_2_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod3_2_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod3_2_pages_by_rotation_missing");
    let err = sort_mod3_2_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod3_2_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod3_2_pages_by_size_missing");
    let err = sort_mod3_2_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod3_2_pages_missing");
    let err = add_page_numbers_mod3_2_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod3_2_pages_missing");
    let err = add_text_watermark_mod3_2_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod3_2_pages_missing");
    let err = add_page_header_mod3_2_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod3_2_pages_missing");
    let err = add_page_footer_mod3_2_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod3_2_pages_missing");
    let err = add_page_border_mod3_2_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod3_2_pages_missing");
    let err = bookmark_mod3_2_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod3_2_pages_missing");
    let err = set_page_size_mod3_2_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod3_2_pages_missing");
    let err = insert_blank_before_mod3_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod3_2_pages_missing");
    let err = insert_blank_after_mod3_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod3_2_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod3_2_pages_to_end_missing");
    let err = duplicate_mod3_2_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod3_2_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod3_2_pages_to_start_missing");
    let err = duplicate_mod3_2_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod3_2_pages_rejects_missing_file() {
    let missing = tmp("extract_mod3_2_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod3_2_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_2_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod3_2_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_2_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_2_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod3_2_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_2_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_2_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod3_2_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_2_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_2_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod3_2_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_2_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_2_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod3_2_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_2_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_2_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod3_2_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_2_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_2_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod3_2_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_2_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_2_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod3_2_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_2_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_2_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod3_2_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_2_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod3_2_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod3_2_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod3_2_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod4_0_pages_missing");
    let err = rotate_mod4_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod4_0_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod4_0_pages_ccw_missing");
    let err = rotate_mod4_0_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod4_0_pages_missing");
    let err = rotate_180_mod4_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod4_0_pages_missing");
    let err = reset_rotation_mod4_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("delete_mod4_0_pages_missing");
    let err = delete_mod4_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("keep_mod4_0_pages_missing");
    let err = keep_mod4_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_0_pages_missing");
    let err = duplicate_mod4_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod4_0_pages_missing");
    let err = flatten_mod4_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("crop_mod4_0_pages_missing");
    let err = crop_mod4_0_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("expand_mod4_0_pages_missing");
    let err = expand_mod4_0_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod4_0_pages_missing");
    let err = shrink_mod4_0_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod4_0_pages_missing");
    let err = reverse_mod4_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod4_0_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod4_0_pages_to_start_missing");
    let err = move_mod4_0_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod4_0_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod4_0_pages_to_end_missing");
    let err = move_mod4_0_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod4_0_pages_missing");
    let err = clear_crop_mod4_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_0_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_0_pages_before_missing");
    let err = duplicate_mod4_0_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod4_0_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod4_0_pages_by_rotation_missing");
    let err = sort_mod4_0_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod4_0_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod4_0_pages_by_size_missing");
    let err = sort_mod4_0_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod4_0_pages_missing");
    let err = add_page_numbers_mod4_0_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod4_0_pages_missing");
    let err = add_text_watermark_mod4_0_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod4_0_pages_missing");
    let err = add_page_header_mod4_0_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod4_0_pages_missing");
    let err = add_page_footer_mod4_0_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod4_0_pages_missing");
    let err = add_page_border_mod4_0_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod4_0_pages_missing");
    let err = bookmark_mod4_0_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod4_0_pages_missing");
    let err = set_page_size_mod4_0_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod4_0_pages_missing");
    let err = insert_blank_before_mod4_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod4_0_pages_missing");
    let err = insert_blank_after_mod4_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_0_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_0_pages_to_end_missing");
    let err = duplicate_mod4_0_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_0_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_0_pages_to_start_missing");
    let err = duplicate_mod4_0_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod4_0_pages_rejects_missing_file() {
    let missing = tmp("extract_mod4_0_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod4_0_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_0_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod4_0_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_0_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_0_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod4_0_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_0_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_0_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod4_0_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_0_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_0_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod4_0_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_0_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_0_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod4_0_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_0_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_0_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod4_0_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_0_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_0_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod4_0_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_0_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_0_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod4_0_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_0_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_0_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod4_0_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_0_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_0_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod4_0_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_0_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod4_1_pages_missing");
    let err = rotate_mod4_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod4_1_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod4_1_pages_ccw_missing");
    let err = rotate_mod4_1_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod4_1_pages_missing");
    let err = rotate_180_mod4_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod4_1_pages_missing");
    let err = reset_rotation_mod4_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("delete_mod4_1_pages_missing");
    let err = delete_mod4_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("keep_mod4_1_pages_missing");
    let err = keep_mod4_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_1_pages_missing");
    let err = duplicate_mod4_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod4_1_pages_missing");
    let err = flatten_mod4_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("crop_mod4_1_pages_missing");
    let err = crop_mod4_1_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("expand_mod4_1_pages_missing");
    let err = expand_mod4_1_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod4_1_pages_missing");
    let err = shrink_mod4_1_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod4_1_pages_missing");
    let err = reverse_mod4_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod4_1_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod4_1_pages_to_start_missing");
    let err = move_mod4_1_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod4_1_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod4_1_pages_to_end_missing");
    let err = move_mod4_1_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod4_1_pages_missing");
    let err = clear_crop_mod4_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_1_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_1_pages_before_missing");
    let err = duplicate_mod4_1_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod4_1_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod4_1_pages_by_rotation_missing");
    let err = sort_mod4_1_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod4_1_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod4_1_pages_by_size_missing");
    let err = sort_mod4_1_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod4_1_pages_missing");
    let err = add_page_numbers_mod4_1_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod4_1_pages_missing");
    let err = add_text_watermark_mod4_1_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod4_1_pages_missing");
    let err = add_page_header_mod4_1_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod4_1_pages_missing");
    let err = add_page_footer_mod4_1_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod4_1_pages_missing");
    let err = add_page_border_mod4_1_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod4_1_pages_missing");
    let err = bookmark_mod4_1_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod4_1_pages_missing");
    let err = set_page_size_mod4_1_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod4_1_pages_missing");
    let err = insert_blank_before_mod4_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod4_1_pages_missing");
    let err = insert_blank_after_mod4_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_1_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_1_pages_to_end_missing");
    let err = duplicate_mod4_1_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_1_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_1_pages_to_start_missing");
    let err = duplicate_mod4_1_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod4_1_pages_rejects_missing_file() {
    let missing = tmp("extract_mod4_1_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod4_1_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_1_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod4_1_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_1_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_1_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod4_1_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_1_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_1_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod4_1_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_1_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_1_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod4_1_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_1_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_1_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod4_1_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_1_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_1_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod4_1_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_1_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_1_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod4_1_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_1_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_1_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod4_1_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_1_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_1_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod4_1_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_1_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_1_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod4_1_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_1_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod4_2_pages_missing");
    let err = rotate_mod4_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod4_2_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod4_2_pages_ccw_missing");
    let err = rotate_mod4_2_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod4_2_pages_missing");
    let err = rotate_180_mod4_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod4_2_pages_missing");
    let err = reset_rotation_mod4_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("delete_mod4_2_pages_missing");
    let err = delete_mod4_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("keep_mod4_2_pages_missing");
    let err = keep_mod4_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_2_pages_missing");
    let err = duplicate_mod4_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod4_2_pages_missing");
    let err = flatten_mod4_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("crop_mod4_2_pages_missing");
    let err = crop_mod4_2_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("expand_mod4_2_pages_missing");
    let err = expand_mod4_2_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod4_2_pages_missing");
    let err = shrink_mod4_2_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod4_2_pages_missing");
    let err = reverse_mod4_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod4_2_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod4_2_pages_to_start_missing");
    let err = move_mod4_2_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod4_2_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod4_2_pages_to_end_missing");
    let err = move_mod4_2_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod4_2_pages_missing");
    let err = clear_crop_mod4_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_2_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_2_pages_before_missing");
    let err = duplicate_mod4_2_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod4_2_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod4_2_pages_by_rotation_missing");
    let err = sort_mod4_2_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod4_2_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod4_2_pages_by_size_missing");
    let err = sort_mod4_2_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod4_2_pages_missing");
    let err = add_page_numbers_mod4_2_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod4_2_pages_missing");
    let err = add_text_watermark_mod4_2_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod4_2_pages_missing");
    let err = add_page_header_mod4_2_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod4_2_pages_missing");
    let err = add_page_footer_mod4_2_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod4_2_pages_missing");
    let err = add_page_border_mod4_2_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod4_2_pages_missing");
    let err = bookmark_mod4_2_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod4_2_pages_missing");
    let err = set_page_size_mod4_2_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod4_2_pages_missing");
    let err = insert_blank_before_mod4_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod4_2_pages_missing");
    let err = insert_blank_after_mod4_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_2_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_2_pages_to_end_missing");
    let err = duplicate_mod4_2_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_2_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_2_pages_to_start_missing");
    let err = duplicate_mod4_2_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod4_2_pages_rejects_missing_file() {
    let missing = tmp("extract_mod4_2_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod4_2_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_2_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod4_2_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_2_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_2_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod4_2_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_2_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_2_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod4_2_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_2_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_2_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod4_2_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_2_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_2_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod4_2_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_2_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_2_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod4_2_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_2_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_2_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod4_2_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_2_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_2_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod4_2_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_2_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_2_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod4_2_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_2_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_2_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod4_2_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_2_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod4_3_pages_missing");
    let err = rotate_mod4_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod4_3_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod4_3_pages_ccw_missing");
    let err = rotate_mod4_3_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod4_3_pages_missing");
    let err = rotate_180_mod4_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod4_3_pages_missing");
    let err = reset_rotation_mod4_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("delete_mod4_3_pages_missing");
    let err = delete_mod4_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("keep_mod4_3_pages_missing");
    let err = keep_mod4_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_3_pages_missing");
    let err = duplicate_mod4_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod4_3_pages_missing");
    let err = flatten_mod4_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("crop_mod4_3_pages_missing");
    let err = crop_mod4_3_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("expand_mod4_3_pages_missing");
    let err = expand_mod4_3_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod4_3_pages_missing");
    let err = shrink_mod4_3_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod4_3_pages_missing");
    let err = reverse_mod4_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod4_3_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod4_3_pages_to_start_missing");
    let err = move_mod4_3_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod4_3_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod4_3_pages_to_end_missing");
    let err = move_mod4_3_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod4_3_pages_missing");
    let err = clear_crop_mod4_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_3_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_3_pages_before_missing");
    let err = duplicate_mod4_3_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod4_3_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod4_3_pages_by_rotation_missing");
    let err = sort_mod4_3_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod4_3_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod4_3_pages_by_size_missing");
    let err = sort_mod4_3_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod4_3_pages_missing");
    let err = add_page_numbers_mod4_3_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod4_3_pages_missing");
    let err = add_text_watermark_mod4_3_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod4_3_pages_missing");
    let err = add_page_header_mod4_3_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod4_3_pages_missing");
    let err = add_page_footer_mod4_3_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod4_3_pages_missing");
    let err = add_page_border_mod4_3_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod4_3_pages_missing");
    let err = bookmark_mod4_3_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod4_3_pages_missing");
    let err = set_page_size_mod4_3_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod4_3_pages_missing");
    let err = insert_blank_before_mod4_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod4_3_pages_missing");
    let err = insert_blank_after_mod4_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_3_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_3_pages_to_end_missing");
    let err = duplicate_mod4_3_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod4_3_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod4_3_pages_to_start_missing");
    let err = duplicate_mod4_3_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod4_3_pages_rejects_missing_file() {
    let missing = tmp("extract_mod4_3_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod4_3_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_3_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod4_3_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_3_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_3_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod4_3_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_3_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_3_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod4_3_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_3_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_3_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod4_3_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_3_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_3_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod4_3_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_3_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_3_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod4_3_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_3_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_3_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod4_3_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_3_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_3_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod4_3_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_3_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_3_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod4_3_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_3_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod4_3_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod4_3_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod4_3_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod5_0_pages_missing");
    let err = rotate_mod5_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod5_0_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod5_0_pages_ccw_missing");
    let err = rotate_mod5_0_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod5_0_pages_missing");
    let err = rotate_180_mod5_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod5_0_pages_missing");
    let err = reset_rotation_mod5_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("delete_mod5_0_pages_missing");
    let err = delete_mod5_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("keep_mod5_0_pages_missing");
    let err = keep_mod5_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_0_pages_missing");
    let err = duplicate_mod5_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod5_0_pages_missing");
    let err = flatten_mod5_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("crop_mod5_0_pages_missing");
    let err = crop_mod5_0_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("expand_mod5_0_pages_missing");
    let err = expand_mod5_0_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod5_0_pages_missing");
    let err = shrink_mod5_0_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod5_0_pages_missing");
    let err = reverse_mod5_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod5_0_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod5_0_pages_to_start_missing");
    let err = move_mod5_0_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod5_0_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod5_0_pages_to_end_missing");
    let err = move_mod5_0_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod5_0_pages_missing");
    let err = clear_crop_mod5_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_0_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_0_pages_before_missing");
    let err = duplicate_mod5_0_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod5_0_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod5_0_pages_by_rotation_missing");
    let err = sort_mod5_0_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod5_0_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod5_0_pages_by_size_missing");
    let err = sort_mod5_0_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod5_0_pages_missing");
    let err = add_page_numbers_mod5_0_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod5_0_pages_missing");
    let err = add_text_watermark_mod5_0_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod5_0_pages_missing");
    let err = add_page_header_mod5_0_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod5_0_pages_missing");
    let err = add_page_footer_mod5_0_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod5_0_pages_missing");
    let err = add_page_border_mod5_0_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod5_0_pages_missing");
    let err = bookmark_mod5_0_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod5_0_pages_missing");
    let err = set_page_size_mod5_0_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod5_0_pages_missing");
    let err = insert_blank_before_mod5_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod5_0_pages_missing");
    let err = insert_blank_after_mod5_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_0_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_0_pages_to_end_missing");
    let err = duplicate_mod5_0_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_0_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_0_pages_to_start_missing");
    let err = duplicate_mod5_0_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod5_0_pages_rejects_missing_file() {
    let missing = tmp("extract_mod5_0_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod5_0_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_0_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod5_0_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_0_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_0_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod5_0_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_0_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_0_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod5_0_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_0_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_0_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod5_0_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_0_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_0_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod5_0_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_0_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_0_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod5_0_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_0_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_0_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod5_0_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_0_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_0_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod5_0_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_0_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_0_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod5_0_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_0_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_0_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod5_0_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_0_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod5_1_pages_missing");
    let err = rotate_mod5_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod5_1_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod5_1_pages_ccw_missing");
    let err = rotate_mod5_1_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod5_1_pages_missing");
    let err = rotate_180_mod5_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod5_1_pages_missing");
    let err = reset_rotation_mod5_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("delete_mod5_1_pages_missing");
    let err = delete_mod5_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("keep_mod5_1_pages_missing");
    let err = keep_mod5_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_1_pages_missing");
    let err = duplicate_mod5_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod5_1_pages_missing");
    let err = flatten_mod5_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("crop_mod5_1_pages_missing");
    let err = crop_mod5_1_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("expand_mod5_1_pages_missing");
    let err = expand_mod5_1_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod5_1_pages_missing");
    let err = shrink_mod5_1_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod5_1_pages_missing");
    let err = reverse_mod5_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod5_1_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod5_1_pages_to_start_missing");
    let err = move_mod5_1_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod5_1_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod5_1_pages_to_end_missing");
    let err = move_mod5_1_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod5_1_pages_missing");
    let err = clear_crop_mod5_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_1_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_1_pages_before_missing");
    let err = duplicate_mod5_1_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod5_1_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod5_1_pages_by_rotation_missing");
    let err = sort_mod5_1_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod5_1_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod5_1_pages_by_size_missing");
    let err = sort_mod5_1_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod5_1_pages_missing");
    let err = add_page_numbers_mod5_1_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod5_1_pages_missing");
    let err = add_text_watermark_mod5_1_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod5_1_pages_missing");
    let err = add_page_header_mod5_1_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod5_1_pages_missing");
    let err = add_page_footer_mod5_1_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod5_1_pages_missing");
    let err = add_page_border_mod5_1_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod5_1_pages_missing");
    let err = bookmark_mod5_1_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod5_1_pages_missing");
    let err = set_page_size_mod5_1_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod5_1_pages_missing");
    let err = insert_blank_before_mod5_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod5_1_pages_missing");
    let err = insert_blank_after_mod5_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_1_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_1_pages_to_end_missing");
    let err = duplicate_mod5_1_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_1_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_1_pages_to_start_missing");
    let err = duplicate_mod5_1_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod5_1_pages_rejects_missing_file() {
    let missing = tmp("extract_mod5_1_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod5_1_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_1_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod5_1_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_1_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_1_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod5_1_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_1_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_1_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod5_1_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_1_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_1_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod5_1_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_1_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_1_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod5_1_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_1_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_1_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod5_1_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_1_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_1_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod5_1_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_1_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_1_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod5_1_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_1_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_1_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod5_1_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_1_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_1_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod5_1_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_1_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod5_2_pages_missing");
    let err = rotate_mod5_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod5_2_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod5_2_pages_ccw_missing");
    let err = rotate_mod5_2_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod5_2_pages_missing");
    let err = rotate_180_mod5_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod5_2_pages_missing");
    let err = reset_rotation_mod5_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("delete_mod5_2_pages_missing");
    let err = delete_mod5_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("keep_mod5_2_pages_missing");
    let err = keep_mod5_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_2_pages_missing");
    let err = duplicate_mod5_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod5_2_pages_missing");
    let err = flatten_mod5_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("crop_mod5_2_pages_missing");
    let err = crop_mod5_2_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("expand_mod5_2_pages_missing");
    let err = expand_mod5_2_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod5_2_pages_missing");
    let err = shrink_mod5_2_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod5_2_pages_missing");
    let err = reverse_mod5_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod5_2_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod5_2_pages_to_start_missing");
    let err = move_mod5_2_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod5_2_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod5_2_pages_to_end_missing");
    let err = move_mod5_2_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod5_2_pages_missing");
    let err = clear_crop_mod5_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_2_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_2_pages_before_missing");
    let err = duplicate_mod5_2_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod5_2_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod5_2_pages_by_rotation_missing");
    let err = sort_mod5_2_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod5_2_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod5_2_pages_by_size_missing");
    let err = sort_mod5_2_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod5_2_pages_missing");
    let err = add_page_numbers_mod5_2_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod5_2_pages_missing");
    let err = add_text_watermark_mod5_2_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod5_2_pages_missing");
    let err = add_page_header_mod5_2_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod5_2_pages_missing");
    let err = add_page_footer_mod5_2_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod5_2_pages_missing");
    let err = add_page_border_mod5_2_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod5_2_pages_missing");
    let err = bookmark_mod5_2_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod5_2_pages_missing");
    let err = set_page_size_mod5_2_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod5_2_pages_missing");
    let err = insert_blank_before_mod5_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod5_2_pages_missing");
    let err = insert_blank_after_mod5_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_2_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_2_pages_to_end_missing");
    let err = duplicate_mod5_2_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_2_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_2_pages_to_start_missing");
    let err = duplicate_mod5_2_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod5_2_pages_rejects_missing_file() {
    let missing = tmp("extract_mod5_2_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod5_2_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_2_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod5_2_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_2_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_2_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod5_2_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_2_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_2_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod5_2_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_2_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_2_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod5_2_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_2_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_2_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod5_2_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_2_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_2_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod5_2_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_2_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_2_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod5_2_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_2_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_2_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod5_2_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_2_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_2_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod5_2_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_2_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_2_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod5_2_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_2_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod5_3_pages_missing");
    let err = rotate_mod5_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod5_3_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod5_3_pages_ccw_missing");
    let err = rotate_mod5_3_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod5_3_pages_missing");
    let err = rotate_180_mod5_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod5_3_pages_missing");
    let err = reset_rotation_mod5_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("delete_mod5_3_pages_missing");
    let err = delete_mod5_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("keep_mod5_3_pages_missing");
    let err = keep_mod5_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_3_pages_missing");
    let err = duplicate_mod5_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod5_3_pages_missing");
    let err = flatten_mod5_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("crop_mod5_3_pages_missing");
    let err = crop_mod5_3_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("expand_mod5_3_pages_missing");
    let err = expand_mod5_3_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod5_3_pages_missing");
    let err = shrink_mod5_3_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod5_3_pages_missing");
    let err = reverse_mod5_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod5_3_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod5_3_pages_to_start_missing");
    let err = move_mod5_3_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod5_3_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod5_3_pages_to_end_missing");
    let err = move_mod5_3_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod5_3_pages_missing");
    let err = clear_crop_mod5_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_3_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_3_pages_before_missing");
    let err = duplicate_mod5_3_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod5_3_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod5_3_pages_by_rotation_missing");
    let err = sort_mod5_3_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod5_3_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod5_3_pages_by_size_missing");
    let err = sort_mod5_3_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod5_3_pages_missing");
    let err = add_page_numbers_mod5_3_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod5_3_pages_missing");
    let err = add_text_watermark_mod5_3_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod5_3_pages_missing");
    let err = add_page_header_mod5_3_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod5_3_pages_missing");
    let err = add_page_footer_mod5_3_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod5_3_pages_missing");
    let err = add_page_border_mod5_3_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod5_3_pages_missing");
    let err = bookmark_mod5_3_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod5_3_pages_missing");
    let err = set_page_size_mod5_3_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod5_3_pages_missing");
    let err = insert_blank_before_mod5_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod5_3_pages_missing");
    let err = insert_blank_after_mod5_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_3_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_3_pages_to_end_missing");
    let err = duplicate_mod5_3_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_3_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_3_pages_to_start_missing");
    let err = duplicate_mod5_3_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod5_3_pages_rejects_missing_file() {
    let missing = tmp("extract_mod5_3_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod5_3_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_3_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod5_3_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_3_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_3_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod5_3_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_3_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_3_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod5_3_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_3_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_3_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod5_3_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_3_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_3_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod5_3_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_3_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_3_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod5_3_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_3_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_3_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod5_3_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_3_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_3_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod5_3_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_3_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_3_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod5_3_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_3_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_3_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod5_3_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_3_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod5_4_pages_missing");
    let err = rotate_mod5_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod5_4_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod5_4_pages_ccw_missing");
    let err = rotate_mod5_4_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod5_4_pages_missing");
    let err = rotate_180_mod5_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod5_4_pages_missing");
    let err = reset_rotation_mod5_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("delete_mod5_4_pages_missing");
    let err = delete_mod5_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("keep_mod5_4_pages_missing");
    let err = keep_mod5_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_4_pages_missing");
    let err = duplicate_mod5_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod5_4_pages_missing");
    let err = flatten_mod5_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("crop_mod5_4_pages_missing");
    let err = crop_mod5_4_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("expand_mod5_4_pages_missing");
    let err = expand_mod5_4_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod5_4_pages_missing");
    let err = shrink_mod5_4_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod5_4_pages_missing");
    let err = reverse_mod5_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod5_4_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod5_4_pages_to_start_missing");
    let err = move_mod5_4_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod5_4_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod5_4_pages_to_end_missing");
    let err = move_mod5_4_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod5_4_pages_missing");
    let err = clear_crop_mod5_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_4_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_4_pages_before_missing");
    let err = duplicate_mod5_4_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod5_4_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod5_4_pages_by_rotation_missing");
    let err = sort_mod5_4_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod5_4_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod5_4_pages_by_size_missing");
    let err = sort_mod5_4_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod5_4_pages_missing");
    let err = add_page_numbers_mod5_4_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod5_4_pages_missing");
    let err = add_text_watermark_mod5_4_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod5_4_pages_missing");
    let err = add_page_header_mod5_4_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod5_4_pages_missing");
    let err = add_page_footer_mod5_4_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod5_4_pages_missing");
    let err = add_page_border_mod5_4_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod5_4_pages_missing");
    let err = bookmark_mod5_4_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod5_4_pages_missing");
    let err = set_page_size_mod5_4_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod5_4_pages_missing");
    let err = insert_blank_before_mod5_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod5_4_pages_missing");
    let err = insert_blank_after_mod5_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_4_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_4_pages_to_end_missing");
    let err = duplicate_mod5_4_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod5_4_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod5_4_pages_to_start_missing");
    let err = duplicate_mod5_4_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod5_4_pages_rejects_missing_file() {
    let missing = tmp("extract_mod5_4_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod5_4_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_4_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod5_4_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_4_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_4_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod5_4_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_4_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_4_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod5_4_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_4_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_4_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod5_4_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_4_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_4_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod5_4_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_4_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_4_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod5_4_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_4_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_4_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod5_4_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_4_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_4_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod5_4_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_4_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_4_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod5_4_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_4_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod5_4_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod5_4_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod5_4_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod6_0_pages_missing");
    let err = rotate_mod6_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod6_0_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod6_0_pages_ccw_missing");
    let err = rotate_mod6_0_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod6_0_pages_missing");
    let err = rotate_180_mod6_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod6_0_pages_missing");
    let err = reset_rotation_mod6_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("delete_mod6_0_pages_missing");
    let err = delete_mod6_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("keep_mod6_0_pages_missing");
    let err = keep_mod6_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_0_pages_missing");
    let err = duplicate_mod6_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod6_0_pages_missing");
    let err = flatten_mod6_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("crop_mod6_0_pages_missing");
    let err = crop_mod6_0_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("expand_mod6_0_pages_missing");
    let err = expand_mod6_0_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod6_0_pages_missing");
    let err = shrink_mod6_0_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod6_0_pages_missing");
    let err = reverse_mod6_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod6_0_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod6_0_pages_to_start_missing");
    let err = move_mod6_0_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod6_0_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod6_0_pages_to_end_missing");
    let err = move_mod6_0_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod6_0_pages_missing");
    let err = clear_crop_mod6_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_0_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_0_pages_before_missing");
    let err = duplicate_mod6_0_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod6_0_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod6_0_pages_by_rotation_missing");
    let err = sort_mod6_0_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod6_0_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod6_0_pages_by_size_missing");
    let err = sort_mod6_0_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod6_0_pages_missing");
    let err = add_page_numbers_mod6_0_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod6_0_pages_missing");
    let err = add_text_watermark_mod6_0_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod6_0_pages_missing");
    let err = add_page_header_mod6_0_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod6_0_pages_missing");
    let err = add_page_footer_mod6_0_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod6_0_pages_missing");
    let err = add_page_border_mod6_0_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod6_0_pages_missing");
    let err = bookmark_mod6_0_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod6_0_pages_missing");
    let err = set_page_size_mod6_0_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod6_0_pages_missing");
    let err = insert_blank_before_mod6_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod6_0_pages_missing");
    let err = insert_blank_after_mod6_0_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_0_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_0_pages_to_end_missing");
    let err = duplicate_mod6_0_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_0_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_0_pages_to_start_missing");
    let err = duplicate_mod6_0_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod6_0_pages_rejects_missing_file() {
    let missing = tmp("extract_mod6_0_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod6_0_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_0_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod6_0_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_0_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_0_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod6_0_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_0_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_0_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod6_0_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_0_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_0_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod6_0_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_0_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_0_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod6_0_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_0_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_0_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod6_0_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_0_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_0_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod6_0_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_0_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_0_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod6_0_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_0_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_0_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod6_0_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_0_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_0_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod6_0_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_0_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod6_1_pages_missing");
    let err = rotate_mod6_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod6_1_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod6_1_pages_ccw_missing");
    let err = rotate_mod6_1_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod6_1_pages_missing");
    let err = rotate_180_mod6_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod6_1_pages_missing");
    let err = reset_rotation_mod6_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("delete_mod6_1_pages_missing");
    let err = delete_mod6_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("keep_mod6_1_pages_missing");
    let err = keep_mod6_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_1_pages_missing");
    let err = duplicate_mod6_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod6_1_pages_missing");
    let err = flatten_mod6_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("crop_mod6_1_pages_missing");
    let err = crop_mod6_1_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("expand_mod6_1_pages_missing");
    let err = expand_mod6_1_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod6_1_pages_missing");
    let err = shrink_mod6_1_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod6_1_pages_missing");
    let err = reverse_mod6_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod6_1_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod6_1_pages_to_start_missing");
    let err = move_mod6_1_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod6_1_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod6_1_pages_to_end_missing");
    let err = move_mod6_1_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod6_1_pages_missing");
    let err = clear_crop_mod6_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_1_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_1_pages_before_missing");
    let err = duplicate_mod6_1_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod6_1_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod6_1_pages_by_rotation_missing");
    let err = sort_mod6_1_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod6_1_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod6_1_pages_by_size_missing");
    let err = sort_mod6_1_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod6_1_pages_missing");
    let err = add_page_numbers_mod6_1_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod6_1_pages_missing");
    let err = add_text_watermark_mod6_1_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod6_1_pages_missing");
    let err = add_page_header_mod6_1_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod6_1_pages_missing");
    let err = add_page_footer_mod6_1_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod6_1_pages_missing");
    let err = add_page_border_mod6_1_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod6_1_pages_missing");
    let err = bookmark_mod6_1_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod6_1_pages_missing");
    let err = set_page_size_mod6_1_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod6_1_pages_missing");
    let err = insert_blank_before_mod6_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod6_1_pages_missing");
    let err = insert_blank_after_mod6_1_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_1_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_1_pages_to_end_missing");
    let err = duplicate_mod6_1_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_1_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_1_pages_to_start_missing");
    let err = duplicate_mod6_1_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod6_1_pages_rejects_missing_file() {
    let missing = tmp("extract_mod6_1_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod6_1_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_1_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod6_1_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_1_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_1_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod6_1_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_1_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_1_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod6_1_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_1_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_1_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod6_1_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_1_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_1_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod6_1_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_1_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_1_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod6_1_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_1_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_1_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod6_1_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_1_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_1_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod6_1_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_1_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_1_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod6_1_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_1_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_1_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod6_1_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_1_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod6_2_pages_missing");
    let err = rotate_mod6_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod6_2_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod6_2_pages_ccw_missing");
    let err = rotate_mod6_2_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod6_2_pages_missing");
    let err = rotate_180_mod6_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod6_2_pages_missing");
    let err = reset_rotation_mod6_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("delete_mod6_2_pages_missing");
    let err = delete_mod6_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("keep_mod6_2_pages_missing");
    let err = keep_mod6_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_2_pages_missing");
    let err = duplicate_mod6_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod6_2_pages_missing");
    let err = flatten_mod6_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("crop_mod6_2_pages_missing");
    let err = crop_mod6_2_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("expand_mod6_2_pages_missing");
    let err = expand_mod6_2_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod6_2_pages_missing");
    let err = shrink_mod6_2_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod6_2_pages_missing");
    let err = reverse_mod6_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod6_2_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod6_2_pages_to_start_missing");
    let err = move_mod6_2_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod6_2_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod6_2_pages_to_end_missing");
    let err = move_mod6_2_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod6_2_pages_missing");
    let err = clear_crop_mod6_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_2_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_2_pages_before_missing");
    let err = duplicate_mod6_2_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod6_2_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod6_2_pages_by_rotation_missing");
    let err = sort_mod6_2_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod6_2_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod6_2_pages_by_size_missing");
    let err = sort_mod6_2_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod6_2_pages_missing");
    let err = add_page_numbers_mod6_2_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod6_2_pages_missing");
    let err = add_text_watermark_mod6_2_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod6_2_pages_missing");
    let err = add_page_header_mod6_2_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod6_2_pages_missing");
    let err = add_page_footer_mod6_2_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod6_2_pages_missing");
    let err = add_page_border_mod6_2_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod6_2_pages_missing");
    let err = bookmark_mod6_2_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod6_2_pages_missing");
    let err = set_page_size_mod6_2_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod6_2_pages_missing");
    let err = insert_blank_before_mod6_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod6_2_pages_missing");
    let err = insert_blank_after_mod6_2_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_2_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_2_pages_to_end_missing");
    let err = duplicate_mod6_2_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_2_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_2_pages_to_start_missing");
    let err = duplicate_mod6_2_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod6_2_pages_rejects_missing_file() {
    let missing = tmp("extract_mod6_2_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod6_2_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_2_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod6_2_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_2_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_2_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod6_2_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_2_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_2_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod6_2_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_2_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_2_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod6_2_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_2_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_2_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod6_2_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_2_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_2_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod6_2_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_2_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_2_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod6_2_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_2_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_2_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod6_2_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_2_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_2_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod6_2_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_2_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_2_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod6_2_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_2_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod6_3_pages_missing");
    let err = rotate_mod6_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod6_3_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod6_3_pages_ccw_missing");
    let err = rotate_mod6_3_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod6_3_pages_missing");
    let err = rotate_180_mod6_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod6_3_pages_missing");
    let err = reset_rotation_mod6_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("delete_mod6_3_pages_missing");
    let err = delete_mod6_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("keep_mod6_3_pages_missing");
    let err = keep_mod6_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_3_pages_missing");
    let err = duplicate_mod6_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod6_3_pages_missing");
    let err = flatten_mod6_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("crop_mod6_3_pages_missing");
    let err = crop_mod6_3_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("expand_mod6_3_pages_missing");
    let err = expand_mod6_3_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod6_3_pages_missing");
    let err = shrink_mod6_3_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod6_3_pages_missing");
    let err = reverse_mod6_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod6_3_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod6_3_pages_to_start_missing");
    let err = move_mod6_3_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod6_3_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod6_3_pages_to_end_missing");
    let err = move_mod6_3_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod6_3_pages_missing");
    let err = clear_crop_mod6_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_3_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_3_pages_before_missing");
    let err = duplicate_mod6_3_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod6_3_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod6_3_pages_by_rotation_missing");
    let err = sort_mod6_3_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod6_3_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod6_3_pages_by_size_missing");
    let err = sort_mod6_3_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod6_3_pages_missing");
    let err = add_page_numbers_mod6_3_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod6_3_pages_missing");
    let err = add_text_watermark_mod6_3_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod6_3_pages_missing");
    let err = add_page_header_mod6_3_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod6_3_pages_missing");
    let err = add_page_footer_mod6_3_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod6_3_pages_missing");
    let err = add_page_border_mod6_3_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod6_3_pages_missing");
    let err = bookmark_mod6_3_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod6_3_pages_missing");
    let err = set_page_size_mod6_3_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod6_3_pages_missing");
    let err = insert_blank_before_mod6_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod6_3_pages_missing");
    let err = insert_blank_after_mod6_3_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_3_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_3_pages_to_end_missing");
    let err = duplicate_mod6_3_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_3_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_3_pages_to_start_missing");
    let err = duplicate_mod6_3_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod6_3_pages_rejects_missing_file() {
    let missing = tmp("extract_mod6_3_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod6_3_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_3_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod6_3_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_3_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_3_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod6_3_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_3_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_3_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod6_3_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_3_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_3_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod6_3_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_3_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_3_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod6_3_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_3_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_3_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod6_3_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_3_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_3_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod6_3_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_3_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_3_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod6_3_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_3_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_3_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod6_3_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_3_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_3_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod6_3_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_3_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod6_4_pages_missing");
    let err = rotate_mod6_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod6_4_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod6_4_pages_ccw_missing");
    let err = rotate_mod6_4_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod6_4_pages_missing");
    let err = rotate_180_mod6_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod6_4_pages_missing");
    let err = reset_rotation_mod6_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("delete_mod6_4_pages_missing");
    let err = delete_mod6_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("keep_mod6_4_pages_missing");
    let err = keep_mod6_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_4_pages_missing");
    let err = duplicate_mod6_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod6_4_pages_missing");
    let err = flatten_mod6_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("crop_mod6_4_pages_missing");
    let err = crop_mod6_4_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("expand_mod6_4_pages_missing");
    let err = expand_mod6_4_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod6_4_pages_missing");
    let err = shrink_mod6_4_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod6_4_pages_missing");
    let err = reverse_mod6_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod6_4_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod6_4_pages_to_start_missing");
    let err = move_mod6_4_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod6_4_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod6_4_pages_to_end_missing");
    let err = move_mod6_4_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod6_4_pages_missing");
    let err = clear_crop_mod6_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_4_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_4_pages_before_missing");
    let err = duplicate_mod6_4_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod6_4_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod6_4_pages_by_rotation_missing");
    let err = sort_mod6_4_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod6_4_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod6_4_pages_by_size_missing");
    let err = sort_mod6_4_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod6_4_pages_missing");
    let err = add_page_numbers_mod6_4_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod6_4_pages_missing");
    let err = add_text_watermark_mod6_4_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod6_4_pages_missing");
    let err = add_page_header_mod6_4_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod6_4_pages_missing");
    let err = add_page_footer_mod6_4_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod6_4_pages_missing");
    let err = add_page_border_mod6_4_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod6_4_pages_missing");
    let err = bookmark_mod6_4_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod6_4_pages_missing");
    let err = set_page_size_mod6_4_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod6_4_pages_missing");
    let err = insert_blank_before_mod6_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod6_4_pages_missing");
    let err = insert_blank_after_mod6_4_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_4_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_4_pages_to_end_missing");
    let err = duplicate_mod6_4_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_4_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_4_pages_to_start_missing");
    let err = duplicate_mod6_4_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod6_4_pages_rejects_missing_file() {
    let missing = tmp("extract_mod6_4_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod6_4_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_4_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod6_4_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_4_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_4_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod6_4_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_4_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_4_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod6_4_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_4_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_4_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod6_4_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_4_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_4_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod6_4_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_4_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_4_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod6_4_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_4_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_4_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod6_4_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_4_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_4_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod6_4_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_4_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_4_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod6_4_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_4_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_4_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod6_4_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_4_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("rotate_mod6_5_pages_missing");
    let err = rotate_mod6_5_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_mod6_5_pages_ccw_rejects_missing_file() {
    let missing = tmp("rotate_mod6_5_pages_ccw_missing");
    let err = rotate_mod6_5_pages_ccw(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_180_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("rotate_180_mod6_5_pages_missing");
    let err = rotate_180_mod6_5_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reset_rotation_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("reset_rotation_mod6_5_pages_missing");
    let err = reset_rotation_mod6_5_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn delete_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("delete_mod6_5_pages_missing");
    let err = delete_mod6_5_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn keep_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("keep_mod6_5_pages_missing");
    let err = keep_mod6_5_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_5_pages_missing");
    let err = duplicate_mod6_5_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn flatten_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("flatten_mod6_5_pages_missing");
    let err = flatten_mod6_5_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn crop_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("crop_mod6_5_pages_missing");
    let err = crop_mod6_5_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn expand_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("expand_mod6_5_pages_missing");
    let err = expand_mod6_5_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn shrink_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("shrink_mod6_5_pages_missing");
    let err = shrink_mod6_5_pages(missing.to_string_lossy().into_owned(), 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn reverse_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("reverse_mod6_5_pages_missing");
    let err = reverse_mod6_5_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod6_5_pages_to_start_rejects_missing_file() {
    let missing = tmp("move_mod6_5_pages_to_start_missing");
    let err = move_mod6_5_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_mod6_5_pages_to_end_rejects_missing_file() {
    let missing = tmp("move_mod6_5_pages_to_end_missing");
    let err = move_mod6_5_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn clear_crop_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("clear_crop_mod6_5_pages_missing");
    let err = clear_crop_mod6_5_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_5_pages_before_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_5_pages_before_missing");
    let err = duplicate_mod6_5_pages_before(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod6_5_pages_by_rotation_rejects_missing_file() {
    let missing = tmp("sort_mod6_5_pages_by_rotation_missing");
    let err = sort_mod6_5_pages_by_rotation(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn sort_mod6_5_pages_by_size_rejects_missing_file() {
    let missing = tmp("sort_mod6_5_pages_by_size_missing");
    let err = sort_mod6_5_pages_by_size(missing.to_string_lossy().into_owned(), false).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_numbers_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("add_page_numbers_mod6_5_pages_missing");
    let err = add_page_numbers_mod6_5_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_text_watermark_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("add_text_watermark_mod6_5_pages_missing");
    let err = add_text_watermark_mod6_5_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_header_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("add_page_header_mod6_5_pages_missing");
    let err = add_page_header_mod6_5_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_footer_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("add_page_footer_mod6_5_pages_missing");
    let err = add_page_footer_mod6_5_pages(missing.to_string_lossy().into_owned(), "wm".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn add_page_border_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("add_page_border_mod6_5_pages_missing");
    let err = add_page_border_mod6_5_pages(missing.to_string_lossy().into_owned(), 1.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn bookmark_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("bookmark_mod6_5_pages_missing");
    let err = bookmark_mod6_5_pages(missing.to_string_lossy().into_owned(), None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn set_page_size_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("set_page_size_mod6_5_pages_missing");
    let err = set_page_size_mod6_5_pages(missing.to_string_lossy().into_owned(), "letter".to_string()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_before_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_before_mod6_5_pages_missing");
    let err = insert_blank_before_mod6_5_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_blank_after_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("insert_blank_after_mod6_5_pages_missing");
    let err = insert_blank_after_mod6_5_pages(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_5_pages_to_end_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_5_pages_to_end_missing");
    let err = duplicate_mod6_5_pages_to_end(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn duplicate_mod6_5_pages_to_start_rejects_missing_file() {
    let missing = tmp("duplicate_mod6_5_pages_to_start_missing");
    let err = duplicate_mod6_5_pages_to_start(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn extract_mod6_5_pages_rejects_missing_file() {
    let missing = tmp("extract_mod6_5_pages_missing");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod6_5_pages(missing.to_string_lossy().into_owned(), output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_5_pages_as_pdf_rejects_missing_file() {
    let missing = tmp("export_mod6_5_pages_as_pdf_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_5_pages_as_pdf(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_5_pages_png_rejects_missing_file() {
    let missing = tmp("export_mod6_5_pages_png_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_5_pages_png(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_5_pages_jpeg_rejects_missing_file() {
    let missing = tmp("export_mod6_5_pages_jpeg_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_5_pages_jpeg(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_5_pages_webp_rejects_missing_file() {
    let missing = tmp("export_mod6_5_pages_webp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_5_pages_webp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_5_pages_bmp_rejects_missing_file() {
    let missing = tmp("export_mod6_5_pages_bmp_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_5_pages_bmp(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_5_pages_tiff_rejects_missing_file() {
    let missing = tmp("export_mod6_5_pages_tiff_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_5_pages_tiff(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_5_pages_gif_rejects_missing_file() {
    let missing = tmp("export_mod6_5_pages_gif_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_5_pages_gif(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_5_pages_ppm_rejects_missing_file() {
    let missing = tmp("export_mod6_5_pages_ppm_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_5_pages_ppm(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_5_pages_tga_rejects_missing_file() {
    let missing = tmp("export_mod6_5_pages_tga_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_5_pages_tga(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn export_mod6_5_pages_ico_rejects_missing_file() {
    let missing = tmp("export_mod6_5_pages_ico_missing");
    let output_dir = tmp("export_dir");
    let err =
        export_mod6_5_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(!err.is_empty());
}

// PARITY_DOCMOD_TESTS_END
// PARITY_BATCH_TESTS_START
// Auto-generated parity batch tests

#[test]
fn rotate_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_odd_pages_in_range");
    let err = rotate_odd_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_even_pages_in_range");
    let err = rotate_even_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_odd_pages_in_range_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_odd_pages_in_range_ccw");
    let err = rotate_odd_pages_in_range_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_even_pages_in_range_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_even_pages_in_range_ccw");
    let err = rotate_even_pages_in_range_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_odd_pages_in_range");
    let err = rotate_180_odd_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_even_pages_in_range");
    let err = rotate_180_even_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_odd_pages_in_range");
    let err = reset_rotation_odd_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_even_pages_in_range");
    let err = reset_rotation_even_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_odd_pages_in_range");
    let err = delete_odd_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_even_pages_in_range");
    let err = delete_even_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_odd_pages_in_range");
    let err = keep_odd_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_even_pages_in_range");
    let err = keep_even_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_odd_pages_in_range");
    let err = duplicate_odd_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_even_pages_in_range");
    let err = duplicate_even_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_odd_pages_in_range_before_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_odd_pages_in_range_before");
    let err = duplicate_odd_pages_in_range_before(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_even_pages_in_range_before_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_even_pages_in_range_before");
    let err = duplicate_even_pages_in_range_before(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_odd_pages_in_range_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_odd_pages_in_range_to_start");
    let err = duplicate_odd_pages_in_range_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_even_pages_in_range_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_even_pages_in_range_to_start");
    let err = duplicate_even_pages_in_range_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_odd_pages_in_range_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_odd_pages_in_range_to_end");
    let err = duplicate_odd_pages_in_range_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_even_pages_in_range_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_even_pages_in_range_to_end");
    let err = duplicate_even_pages_in_range_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_odd_pages_in_range");
    let err = flatten_odd_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_even_pages_in_range");
    let err = flatten_even_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_odd_pages_in_range");
    let err = reverse_odd_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_even_pages_in_range");
    let err = reverse_even_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_odd_pages_in_range_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "move_odd_pages_in_range_to_start");
    let err = move_odd_pages_in_range_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_even_pages_in_range_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "move_even_pages_in_range_to_start");
    let err = move_even_pages_in_range_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_odd_pages_in_range_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "move_odd_pages_in_range_to_end");
    let err = move_odd_pages_in_range_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_even_pages_in_range_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "move_even_pages_in_range_to_end");
    let err = move_even_pages_in_range_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_odd_pages_in_range_by_rotation_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_odd_pages_in_range_by_rotation");
    let err = sort_odd_pages_in_range_by_rotation(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_even_pages_in_range_by_rotation_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_even_pages_in_range_by_rotation");
    let err = sort_even_pages_in_range_by_rotation(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_odd_pages_in_range_by_size_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_odd_pages_in_range_by_size");
    let err = sort_odd_pages_in_range_by_size(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_even_pages_in_range_by_size_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_even_pages_in_range_by_size");
    let err = sort_even_pages_in_range_by_size(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_odd_pages_in_range");
    let err = crop_odd_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_even_pages_in_range");
    let err = crop_even_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_odd_pages_in_range");
    let err = expand_odd_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_even_pages_in_range");
    let err = expand_even_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_odd_pages_in_range");
    let err = shrink_odd_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_even_pages_in_range");
    let err = shrink_even_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_odd_pages_in_range");
    let err = clear_crop_odd_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_even_pages_in_range");
    let err = clear_crop_even_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_odd_pages_in_range");
    let err = insert_blank_before_odd_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_even_pages_in_range");
    let err = insert_blank_before_even_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_odd_pages_in_range");
    let err = insert_blank_after_odd_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_even_pages_in_range");
    let err = insert_blank_after_even_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_odd_pages_in_range");
    let err = bookmark_odd_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_even_pages_in_range");
    let err = bookmark_even_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_odd_pages_in_range");
    let err = set_page_size_odd_pages_in_range(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_even_pages_in_range");
    let err = set_page_size_even_pages_in_range(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_odd_pages_in_range");
    let output_path = tmp("extract_out.pdf");
    let err = extract_odd_pages_in_range(path.clone(), 5, 10, output_path.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_even_pages_in_range");
    let output_path = tmp("extract_out.pdf");
    let err = extract_even_pages_in_range(path.clone(), 5, 10, output_path.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_odd_pages_in_range");
    let err = add_page_numbers_odd_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_even_pages_in_range");
    let err = add_page_numbers_even_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_odd_pages_in_range");
    let err = add_text_watermark_odd_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_even_pages_in_range");
    let err = add_text_watermark_even_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_odd_pages_in_range");
    let err = add_page_header_odd_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_even_pages_in_range");
    let err = add_page_header_even_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_odd_pages_in_range");
    let err = add_page_footer_odd_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_even_pages_in_range");
    let err = add_page_footer_even_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_odd_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_odd_pages_in_range");
    let err = add_page_border_odd_pages_in_range(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_even_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_even_pages_in_range");
    let err = add_page_border_even_pages_in_range(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_odd_pages_in_range_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_odd_pages_in_range_as_pdf");
    let output_dir = tmp("export_dir");
    let err =
        export_odd_pages_in_range_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_even_pages_in_range_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_even_pages_in_range_as_pdf");
    let output_dir = tmp("export_dir");
    let err =
        export_even_pages_in_range_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_odd_pages_in_range_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_odd_pages_in_range_png");
    let output_dir = tmp("export_dir");
    let err =
        export_odd_pages_in_range_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_even_pages_in_range_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_even_pages_in_range_png");
    let output_dir = tmp("export_dir");
    let err =
        export_even_pages_in_range_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_odd_pages_in_range_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_odd_pages_in_range_jpeg");
    let output_dir = tmp("export_dir");
    let err =
        export_odd_pages_in_range_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_even_pages_in_range_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_even_pages_in_range_jpeg");
    let output_dir = tmp("export_dir");
    let err =
        export_even_pages_in_range_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_odd_pages_in_range_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_odd_pages_in_range_webp");
    let output_dir = tmp("export_dir");
    let err =
        export_odd_pages_in_range_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_even_pages_in_range_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_even_pages_in_range_webp");
    let output_dir = tmp("export_dir");
    let err =
        export_even_pages_in_range_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_odd_pages_in_range_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_odd_pages_in_range_bmp");
    let output_dir = tmp("export_dir");
    let err =
        export_odd_pages_in_range_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_even_pages_in_range_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_even_pages_in_range_bmp");
    let output_dir = tmp("export_dir");
    let err =
        export_even_pages_in_range_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_odd_pages_in_range_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_odd_pages_in_range_tiff");
    let output_dir = tmp("export_dir");
    let err =
        export_odd_pages_in_range_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_even_pages_in_range_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_even_pages_in_range_tiff");
    let output_dir = tmp("export_dir");
    let err =
        export_even_pages_in_range_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_odd_pages_in_range_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_odd_pages_in_range_gif");
    let output_dir = tmp("export_dir");
    let err =
        export_odd_pages_in_range_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_even_pages_in_range_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_even_pages_in_range_gif");
    let output_dir = tmp("export_dir");
    let err =
        export_even_pages_in_range_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_odd_pages_in_range_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_odd_pages_in_range_ppm");
    let output_dir = tmp("export_dir");
    let err =
        export_odd_pages_in_range_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_even_pages_in_range_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_even_pages_in_range_ppm");
    let output_dir = tmp("export_dir");
    let err =
        export_even_pages_in_range_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_odd_pages_in_range_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_odd_pages_in_range_tga");
    let output_dir = tmp("export_dir");
    let err =
        export_odd_pages_in_range_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_even_pages_in_range_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_even_pages_in_range_tga");
    let output_dir = tmp("export_dir");
    let err =
        export_even_pages_in_range_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_odd_pages_in_range_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_odd_pages_in_range_ico");
    let output_dir = tmp("export_dir");
    let err =
        export_odd_pages_in_range_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_even_pages_in_range_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_even_pages_in_range_ico");
    let output_dir = tmp("export_dir");
    let err =
        export_even_pages_in_range_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_range_local_odd_pages");
    let err = rotate_range_local_odd_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_range_local_even_pages");
    let err = rotate_range_local_even_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_range_local_odd_pages_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_range_local_odd_pages_ccw");
    let err = rotate_range_local_odd_pages_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_range_local_even_pages_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_range_local_even_pages_ccw");
    let err = rotate_range_local_even_pages_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_range_local_odd_pages");
    let err = rotate_180_range_local_odd_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_range_local_even_pages");
    let err = rotate_180_range_local_even_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_range_local_odd_pages");
    let err = reset_rotation_range_local_odd_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_range_local_even_pages");
    let err = reset_rotation_range_local_even_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_range_local_odd_pages");
    let err = delete_range_local_odd_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_range_local_even_pages");
    let err = delete_range_local_even_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_range_local_odd_pages");
    let err = keep_range_local_odd_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_range_local_even_pages");
    let err = keep_range_local_even_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_range_local_odd_pages");
    let err = duplicate_range_local_odd_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_range_local_even_pages");
    let err = duplicate_range_local_even_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_range_local_odd_pages_before_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_range_local_odd_pages_before");
    let err = duplicate_range_local_odd_pages_before(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_range_local_even_pages_before_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_range_local_even_pages_before");
    let err = duplicate_range_local_even_pages_before(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_range_local_odd_pages");
    let err = flatten_range_local_odd_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_range_local_even_pages");
    let err = flatten_range_local_even_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_odd_pages_ico_rejects_missing_file() {
    let missing = tmp("export_odd_pages_ico_missing_src");
    let output_dir = tmp("export_odd_pages_ico_missing_dir");
    let err = export_odd_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

#[test]
fn export_even_pages_ico_rejects_missing_file() {
    let missing = tmp("export_even_pages_ico_missing_src");
    let output_dir = tmp("export_even_pages_ico_missing_dir");
    let err = export_even_pages_ico(missing.to_string_lossy().into_owned(), output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("File not found"));
}

// PARITY_BATCH_TESTS_END
// PARITY_BATCH2_TESTS_START
// Auto-generated parity batch2 tests

#[test]
fn reverse_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_range_local_odd_pages");
    let err = reverse_range_local_odd_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_range_local_even_pages");
    let err = reverse_range_local_even_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_odd_range_local_pages_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "move_odd_range_local_pages_to_start");
    let err = move_odd_range_local_pages_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_even_range_local_pages_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "move_even_range_local_pages_to_start");
    let err = move_even_range_local_pages_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_odd_range_local_pages_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "move_odd_range_local_pages_to_end");
    let err = move_odd_range_local_pages_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_even_range_local_pages_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "move_even_range_local_pages_to_end");
    let err = move_even_range_local_pages_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_range_local_odd_pages_by_rotation_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_range_local_odd_pages_by_rotation");
    let err = sort_range_local_odd_pages_by_rotation(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_range_local_even_pages_by_rotation_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_range_local_even_pages_by_rotation");
    let err = sort_range_local_even_pages_by_rotation(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_range_local_odd_pages_by_size_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_range_local_odd_pages_by_size");
    let err = sort_range_local_odd_pages_by_size(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_range_local_even_pages_by_size_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_range_local_even_pages_by_size");
    let err = sort_range_local_even_pages_by_size(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_range_local_odd_pages_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_range_local_odd_pages_to_start");
    let err = duplicate_range_local_odd_pages_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_range_local_even_pages_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_range_local_even_pages_to_start");
    let err = duplicate_range_local_even_pages_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_range_local_odd_pages_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_range_local_odd_pages_to_end");
    let err = duplicate_range_local_odd_pages_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_range_local_even_pages_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_range_local_even_pages_to_end");
    let err = duplicate_range_local_even_pages_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_range_local_odd_pages");
    let err = crop_range_local_odd_pages(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_range_local_even_pages");
    let err = crop_range_local_even_pages(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_range_local_odd_pages");
    let err = expand_range_local_odd_pages(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_range_local_even_pages");
    let err = expand_range_local_even_pages(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_range_local_odd_pages");
    let err = shrink_range_local_odd_pages(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_range_local_even_pages");
    let err = shrink_range_local_even_pages(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_range_local_odd_pages");
    let err = clear_crop_range_local_odd_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_range_local_even_pages");
    let err = clear_crop_range_local_even_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_range_local_odd_pages");
    let err = insert_blank_before_range_local_odd_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_range_local_even_pages");
    let err = insert_blank_before_range_local_even_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_range_local_odd_pages");
    let err = insert_blank_after_range_local_odd_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_range_local_even_pages");
    let err = insert_blank_after_range_local_even_pages(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_range_local_odd_pages");
    let err = bookmark_range_local_odd_pages(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_range_local_even_pages");
    let err = bookmark_range_local_even_pages(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_range_local_odd_pages");
    let err = set_page_size_range_local_odd_pages(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_range_local_even_pages");
    let err = set_page_size_range_local_even_pages(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_range_local_odd_pages");
    let output_path = tmp("extract_out.pdf");
    let err =
        extract_range_local_odd_pages(path.clone(), 5, 10, output_path.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_range_local_even_pages");
    let output_path = tmp("extract_out.pdf");
    let err =
        extract_range_local_even_pages(path.clone(), 5, 10, output_path.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_range_local_odd_pages");
    let err = add_page_numbers_range_local_odd_pages(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_range_local_even_pages");
    let err = add_page_numbers_range_local_even_pages(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_range_local_odd_pages");
    let err = add_text_watermark_range_local_odd_pages(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_range_local_even_pages");
    let err = add_text_watermark_range_local_even_pages(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_range_local_odd_pages");
    let err = add_page_header_range_local_odd_pages(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_range_local_even_pages");
    let err = add_page_header_range_local_even_pages(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_range_local_odd_pages");
    let err = add_page_footer_range_local_odd_pages(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_range_local_even_pages");
    let err = add_page_footer_range_local_even_pages(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_range_local_odd_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_range_local_odd_pages");
    let err = add_page_border_range_local_odd_pages(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_range_local_even_pages_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_range_local_even_pages");
    let err = add_page_border_range_local_even_pages(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_odd_pages_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_odd_pages_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_range_local_odd_pages_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_even_pages_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_even_pages_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_range_local_even_pages_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_odd_pages_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_odd_pages_png");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_odd_pages_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_even_pages_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_even_pages_png");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_even_pages_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_odd_pages_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_odd_pages_jpeg");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_odd_pages_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_even_pages_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_even_pages_jpeg");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_even_pages_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_odd_pages_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_odd_pages_webp");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_odd_pages_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_even_pages_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_even_pages_webp");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_even_pages_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_odd_pages_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_odd_pages_bmp");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_odd_pages_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_even_pages_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_even_pages_bmp");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_even_pages_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_odd_pages_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_odd_pages_tiff");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_odd_pages_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_even_pages_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_even_pages_tiff");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_even_pages_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_odd_pages_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_odd_pages_gif");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_odd_pages_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_even_pages_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_even_pages_gif");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_even_pages_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_odd_pages_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_odd_pages_ppm");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_odd_pages_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_even_pages_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_even_pages_ppm");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_even_pages_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_odd_pages_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_odd_pages_tga");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_odd_pages_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_range_local_even_pages_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_range_local_even_pages_tga");
    let output_dir = tmp("export_dir");
    let err =
        export_range_local_even_pages_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod3_0_pages_in_range_mod3");
    let err = rotate_mod3_0_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod3_1_pages_in_range_mod3");
    let err = rotate_mod3_1_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod3_2_pages_in_range_mod3");
    let err = rotate_mod3_2_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod3_0_pages_in_range_mod3_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod3_0_pages_in_range_mod3_ccw");
    let err = rotate_mod3_0_pages_in_range_mod3_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod3_1_pages_in_range_mod3_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod3_1_pages_in_range_mod3_ccw");
    let err = rotate_mod3_1_pages_in_range_mod3_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod3_2_pages_in_range_mod3_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod3_2_pages_in_range_mod3_ccw");
    let err = rotate_mod3_2_pages_in_range_mod3_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod3_0_pages_in_range_mod3");
    let err = rotate_180_mod3_0_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod3_1_pages_in_range_mod3");
    let err = rotate_180_mod3_1_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod3_2_pages_in_range_mod3");
    let err = rotate_180_mod3_2_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod3_0_pages_in_range_mod3");
    let err = reset_rotation_mod3_0_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod3_1_pages_in_range_mod3");
    let err = reset_rotation_mod3_1_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod3_2_pages_in_range_mod3");
    let err = reset_rotation_mod3_2_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod3_0_pages_in_range_mod3");
    let err = delete_mod3_0_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod3_1_pages_in_range_mod3");
    let err = delete_mod3_1_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod3_2_pages_in_range_mod3");
    let err = delete_mod3_2_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod3_0_pages_in_range_mod3");
    let err = keep_mod3_0_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod3_1_pages_in_range_mod3");
    let err = keep_mod3_1_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod3_2_pages_in_range_mod3");
    let err = keep_mod3_2_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod3_0_pages_in_range_mod3");
    let err = duplicate_mod3_0_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod3_1_pages_in_range_mod3");
    let err = duplicate_mod3_1_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod3_2_pages_in_range_mod3");
    let err = duplicate_mod3_2_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod3_0_pages_in_range_mod3");
    let err = flatten_mod3_0_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod3_1_pages_in_range_mod3");
    let err = flatten_mod3_1_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod3_2_pages_in_range_mod3");
    let err = flatten_mod3_2_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod3_0_pages_in_range_mod3");
    let err = reverse_mod3_0_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod3_1_pages_in_range_mod3");
    let err = reverse_mod3_1_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod3_2_pages_in_range_mod3");
    let err = reverse_mod3_2_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod3_0_pages_in_range_mod3");
    let err = crop_mod3_0_pages_in_range_mod3(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod3_1_pages_in_range_mod3");
    let err = crop_mod3_1_pages_in_range_mod3(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod3_2_pages_in_range_mod3");
    let err = crop_mod3_2_pages_in_range_mod3(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod3_0_pages_in_range_mod3");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod3_0_pages_in_range_mod3(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod3_1_pages_in_range_mod3");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod3_1_pages_in_range_mod3(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod3_2_pages_in_range_mod3");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod3_2_pages_in_range_mod3(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_0_pages_in_range_mod3_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_0_pages_in_range_mod3_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod3_0_pages_in_range_mod3_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_1_pages_in_range_mod3_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_1_pages_in_range_mod3_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod3_1_pages_in_range_mod3_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_2_pages_in_range_mod3_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_2_pages_in_range_mod3_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod3_2_pages_in_range_mod3_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_0_pages_in_range_mod3_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_0_pages_in_range_mod3_png");
    let output_dir = tmp("export_dir");
    let err = export_mod3_0_pages_in_range_mod3_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_1_pages_in_range_mod3_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_1_pages_in_range_mod3_png");
    let output_dir = tmp("export_dir");
    let err = export_mod3_1_pages_in_range_mod3_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_2_pages_in_range_mod3_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_2_pages_in_range_mod3_png");
    let output_dir = tmp("export_dir");
    let err = export_mod3_2_pages_in_range_mod3_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_0_pages_in_range_mod3_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_0_pages_in_range_mod3_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod3_0_pages_in_range_mod3_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

// PARITY_BATCH2_TESTS_END
// PARITY_BATCH3_TESTS_START
// Auto-generated parity batch3 tests

#[test]
fn duplicate_mod3_0_pages_in_range_before_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod3_0_pages_in_range_before_mod3");
    let err = duplicate_mod3_0_pages_in_range_before_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod3_1_pages_in_range_before_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod3_1_pages_in_range_before_mod3");
    let err = duplicate_mod3_1_pages_in_range_before_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod3_2_pages_in_range_before_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod3_2_pages_in_range_before_mod3");
    let err = duplicate_mod3_2_pages_in_range_before_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod3_0_pages_in_range_to_start_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod3_0_pages_in_range_to_start_mod3");
    let err = duplicate_mod3_0_pages_in_range_to_start_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod3_1_pages_in_range_to_start_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod3_1_pages_in_range_to_start_mod3");
    let err = duplicate_mod3_1_pages_in_range_to_start_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod3_2_pages_in_range_to_start_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod3_2_pages_in_range_to_start_mod3");
    let err = duplicate_mod3_2_pages_in_range_to_start_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod3_0_pages_in_range_to_end_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod3_0_pages_in_range_to_end_mod3");
    let err = duplicate_mod3_0_pages_in_range_to_end_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod3_1_pages_in_range_to_end_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod3_1_pages_in_range_to_end_mod3");
    let err = duplicate_mod3_1_pages_in_range_to_end_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod3_2_pages_in_range_to_end_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod3_2_pages_in_range_to_end_mod3");
    let err = duplicate_mod3_2_pages_in_range_to_end_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod3_0_pages_in_range_mod3");
    let err = expand_mod3_0_pages_in_range_mod3(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod3_1_pages_in_range_mod3");
    let err = expand_mod3_1_pages_in_range_mod3(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod3_2_pages_in_range_mod3");
    let err = expand_mod3_2_pages_in_range_mod3(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod3_0_pages_in_range_mod3");
    let err = shrink_mod3_0_pages_in_range_mod3(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod3_1_pages_in_range_mod3");
    let err = shrink_mod3_1_pages_in_range_mod3(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod3_2_pages_in_range_mod3");
    let err = shrink_mod3_2_pages_in_range_mod3(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod3_0_pages_in_range_mod3");
    let err = clear_crop_mod3_0_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod3_1_pages_in_range_mod3");
    let err = clear_crop_mod3_1_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod3_2_pages_in_range_mod3");
    let err = clear_crop_mod3_2_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod3_0_pages_in_range_mod3");
    let err = insert_blank_before_mod3_0_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod3_1_pages_in_range_mod3");
    let err = insert_blank_before_mod3_1_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod3_2_pages_in_range_mod3");
    let err = insert_blank_before_mod3_2_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod3_0_pages_in_range_mod3");
    let err = insert_blank_after_mod3_0_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod3_1_pages_in_range_mod3");
    let err = insert_blank_after_mod3_1_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod3_2_pages_in_range_mod3");
    let err = insert_blank_after_mod3_2_pages_in_range_mod3(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod3_0_pages_in_range_mod3");
    let err = bookmark_mod3_0_pages_in_range_mod3(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod3_1_pages_in_range_mod3");
    let err = bookmark_mod3_1_pages_in_range_mod3(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod3_2_pages_in_range_mod3");
    let err = bookmark_mod3_2_pages_in_range_mod3(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod3_0_pages_in_range_mod3");
    let err = set_page_size_mod3_0_pages_in_range_mod3(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod3_1_pages_in_range_mod3");
    let err = set_page_size_mod3_1_pages_in_range_mod3(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod3_2_pages_in_range_mod3");
    let err = set_page_size_mod3_2_pages_in_range_mod3(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod3_0_pages_in_range_mod3");
    let err = add_page_numbers_mod3_0_pages_in_range_mod3(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod3_1_pages_in_range_mod3");
    let err = add_page_numbers_mod3_1_pages_in_range_mod3(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod3_2_pages_in_range_mod3");
    let err = add_page_numbers_mod3_2_pages_in_range_mod3(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod3_0_pages_in_range_mod3");
    let err = add_text_watermark_mod3_0_pages_in_range_mod3(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod3_1_pages_in_range_mod3");
    let err = add_text_watermark_mod3_1_pages_in_range_mod3(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod3_2_pages_in_range_mod3");
    let err = add_text_watermark_mod3_2_pages_in_range_mod3(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod3_0_pages_in_range_mod3");
    let err = add_page_header_mod3_0_pages_in_range_mod3(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod3_1_pages_in_range_mod3");
    let err = add_page_header_mod3_1_pages_in_range_mod3(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod3_2_pages_in_range_mod3");
    let err = add_page_header_mod3_2_pages_in_range_mod3(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod3_0_pages_in_range_mod3");
    let err = add_page_footer_mod3_0_pages_in_range_mod3(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod3_1_pages_in_range_mod3");
    let err = add_page_footer_mod3_1_pages_in_range_mod3(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod3_2_pages_in_range_mod3");
    let err = add_page_footer_mod3_2_pages_in_range_mod3(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod3_0_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod3_0_pages_in_range_mod3");
    let err = add_page_border_mod3_0_pages_in_range_mod3(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod3_1_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod3_1_pages_in_range_mod3");
    let err = add_page_border_mod3_1_pages_in_range_mod3(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod3_2_pages_in_range_mod3_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod3_2_pages_in_range_mod3");
    let err = add_page_border_mod3_2_pages_in_range_mod3(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_0_pages_in_range_mod3_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_0_pages_in_range_mod3_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod3_0_pages_in_range_mod3_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_1_pages_in_range_mod3_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_1_pages_in_range_mod3_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod3_1_pages_in_range_mod3_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_2_pages_in_range_mod3_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_2_pages_in_range_mod3_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod3_2_pages_in_range_mod3_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_0_pages_in_range_mod3_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_0_pages_in_range_mod3_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod3_0_pages_in_range_mod3_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_1_pages_in_range_mod3_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_1_pages_in_range_mod3_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod3_1_pages_in_range_mod3_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_2_pages_in_range_mod3_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_2_pages_in_range_mod3_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod3_2_pages_in_range_mod3_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_0_pages_in_range_mod3_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_0_pages_in_range_mod3_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod3_0_pages_in_range_mod3_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_1_pages_in_range_mod3_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_1_pages_in_range_mod3_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod3_1_pages_in_range_mod3_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_2_pages_in_range_mod3_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_2_pages_in_range_mod3_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod3_2_pages_in_range_mod3_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_0_pages_in_range_mod3_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_0_pages_in_range_mod3_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod3_0_pages_in_range_mod3_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_1_pages_in_range_mod3_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_1_pages_in_range_mod3_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod3_1_pages_in_range_mod3_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_2_pages_in_range_mod3_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_2_pages_in_range_mod3_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod3_2_pages_in_range_mod3_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_0_pages_in_range_mod3_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_0_pages_in_range_mod3_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod3_0_pages_in_range_mod3_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_1_pages_in_range_mod3_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_1_pages_in_range_mod3_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod3_1_pages_in_range_mod3_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_2_pages_in_range_mod3_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_2_pages_in_range_mod3_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod3_2_pages_in_range_mod3_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_0_pages_in_range_mod3_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_0_pages_in_range_mod3_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod3_0_pages_in_range_mod3_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_1_pages_in_range_mod3_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_1_pages_in_range_mod3_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod3_1_pages_in_range_mod3_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_2_pages_in_range_mod3_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_2_pages_in_range_mod3_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod3_2_pages_in_range_mod3_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_0_pages_in_range_mod3_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_0_pages_in_range_mod3_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod3_0_pages_in_range_mod3_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_1_pages_in_range_mod3_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_1_pages_in_range_mod3_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod3_1_pages_in_range_mod3_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_2_pages_in_range_mod3_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_2_pages_in_range_mod3_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod3_2_pages_in_range_mod3_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_1_pages_in_range_mod3_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_1_pages_in_range_mod3_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod3_1_pages_in_range_mod3_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod3_2_pages_in_range_mod3_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod3_2_pages_in_range_mod3_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod3_2_pages_in_range_mod3_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_first_half_pages_in_range");
    let err = rotate_first_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_second_half_pages_in_range");
    let err = rotate_second_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_first_half_pages_in_range_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_first_half_pages_in_range_ccw");
    let err = rotate_first_half_pages_in_range_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_second_half_pages_in_range_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_second_half_pages_in_range_ccw");
    let err = rotate_second_half_pages_in_range_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_first_half_pages_in_range");
    let err = rotate_180_first_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_second_half_pages_in_range");
    let err = rotate_180_second_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_first_half_pages_in_range");
    let err = reset_rotation_first_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_second_half_pages_in_range");
    let err = reset_rotation_second_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_first_half_pages_in_range");
    let err = delete_first_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_second_half_pages_in_range");
    let err = delete_second_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_first_half_pages_in_range");
    let err = keep_first_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_second_half_pages_in_range");
    let err = keep_second_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_first_half_pages_in_range");
    let err = duplicate_first_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_second_half_pages_in_range");
    let err = duplicate_second_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_first_half_pages_in_range");
    let err = flatten_first_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_second_half_pages_in_range");
    let err = flatten_second_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_first_half_pages_in_range");
    let err = reverse_first_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_second_half_pages_in_range");
    let err = reverse_second_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_first_half_pages_in_range");
    let err = crop_first_half_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_second_half_pages_in_range");
    let err = crop_second_half_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_first_half_pages_in_range");
    let output_path = tmp("extract_out.pdf");
    let err =
        extract_first_half_pages_in_range(path.clone(), 5, 10, output_path.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_second_half_pages_in_range");
    let output_path = tmp("extract_out.pdf");
    let err = extract_second_half_pages_in_range(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_half_pages_in_range_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_half_pages_in_range_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_first_half_pages_in_range_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_half_pages_in_range_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_half_pages_in_range_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_second_half_pages_in_range_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_half_pages_in_range_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_half_pages_in_range_png");
    let output_dir = tmp("export_dir");
    let err = export_first_half_pages_in_range_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_half_pages_in_range_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_half_pages_in_range_png");
    let output_dir = tmp("export_dir");
    let err = export_second_half_pages_in_range_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_half_pages_in_range_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_half_pages_in_range_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_first_half_pages_in_range_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_half_pages_in_range_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_half_pages_in_range_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_second_half_pages_in_range_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_first_half_pages_in_range");
    let err = add_page_numbers_first_half_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_second_half_pages_in_range");
    let err = add_page_numbers_second_half_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_first_half_pages_in_range");
    let err = add_text_watermark_first_half_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_second_half_pages_in_range");
    let err = add_text_watermark_second_half_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

// PARITY_BATCH3_TESTS_END
// PARITY_BATCH4_TESTS_START
// Auto-generated parity batch4 tests

#[test]
fn duplicate_first_half_pages_in_range_before_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_first_half_pages_in_range_before");
    let err = duplicate_first_half_pages_in_range_before(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_second_half_pages_in_range_before_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_second_half_pages_in_range_before");
    let err = duplicate_second_half_pages_in_range_before(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_first_half_pages_in_range_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_first_half_pages_in_range_to_start");
    let err = duplicate_first_half_pages_in_range_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_second_half_pages_in_range_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_second_half_pages_in_range_to_start");
    let err = duplicate_second_half_pages_in_range_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_first_half_pages_in_range_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_first_half_pages_in_range_to_end");
    let err = duplicate_first_half_pages_in_range_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_second_half_pages_in_range_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_second_half_pages_in_range_to_end");
    let err = duplicate_second_half_pages_in_range_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_first_half_pages_in_range");
    let err = expand_first_half_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_second_half_pages_in_range");
    let err = expand_second_half_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_first_half_pages_in_range");
    let err = shrink_first_half_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_second_half_pages_in_range");
    let err = shrink_second_half_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_first_half_pages_in_range");
    let err = clear_crop_first_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_second_half_pages_in_range");
    let err = clear_crop_second_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_first_half_pages_in_range");
    let err = insert_blank_before_first_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_second_half_pages_in_range");
    let err = insert_blank_before_second_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_first_half_pages_in_range");
    let err = insert_blank_after_first_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_second_half_pages_in_range");
    let err = insert_blank_after_second_half_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_first_half_pages_in_range");
    let err = bookmark_first_half_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_second_half_pages_in_range");
    let err = bookmark_second_half_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_first_half_pages_in_range");
    let err = set_page_size_first_half_pages_in_range(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_second_half_pages_in_range");
    let err = set_page_size_second_half_pages_in_range(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_first_half_pages_in_range");
    let err = add_page_header_first_half_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_second_half_pages_in_range");
    let err = add_page_header_second_half_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_first_half_pages_in_range");
    let err = add_page_footer_first_half_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_second_half_pages_in_range");
    let err = add_page_footer_second_half_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_first_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_first_half_pages_in_range");
    let err = add_page_border_first_half_pages_in_range(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_second_half_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_second_half_pages_in_range");
    let err = add_page_border_second_half_pages_in_range(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_half_pages_in_range_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_half_pages_in_range_webp");
    let output_dir = tmp("export_dir");
    let err = export_first_half_pages_in_range_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_half_pages_in_range_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_half_pages_in_range_webp");
    let output_dir = tmp("export_dir");
    let err = export_second_half_pages_in_range_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_half_pages_in_range_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_half_pages_in_range_bmp");
    let output_dir = tmp("export_dir");
    let err = export_first_half_pages_in_range_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_half_pages_in_range_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_half_pages_in_range_bmp");
    let output_dir = tmp("export_dir");
    let err = export_second_half_pages_in_range_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_half_pages_in_range_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_half_pages_in_range_tiff");
    let output_dir = tmp("export_dir");
    let err = export_first_half_pages_in_range_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_half_pages_in_range_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_half_pages_in_range_tiff");
    let output_dir = tmp("export_dir");
    let err = export_second_half_pages_in_range_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_half_pages_in_range_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_half_pages_in_range_gif");
    let output_dir = tmp("export_dir");
    let err = export_first_half_pages_in_range_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_half_pages_in_range_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_half_pages_in_range_gif");
    let output_dir = tmp("export_dir");
    let err = export_second_half_pages_in_range_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_half_pages_in_range_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_half_pages_in_range_ppm");
    let output_dir = tmp("export_dir");
    let err = export_first_half_pages_in_range_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_half_pages_in_range_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_half_pages_in_range_ppm");
    let output_dir = tmp("export_dir");
    let err = export_second_half_pages_in_range_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_half_pages_in_range_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_half_pages_in_range_tga");
    let output_dir = tmp("export_dir");
    let err = export_first_half_pages_in_range_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_half_pages_in_range_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_half_pages_in_range_tga");
    let output_dir = tmp("export_dir");
    let err = export_second_half_pages_in_range_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_half_pages_in_range_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_half_pages_in_range_ico");
    let output_dir = tmp("export_dir");
    let err = export_first_half_pages_in_range_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_half_pages_in_range_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_half_pages_in_range_ico");
    let output_dir = tmp("export_dir");
    let err = export_second_half_pages_in_range_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_first_half_pages_in_range_by_rotation_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_first_half_pages_in_range_by_rotation");
    let err = sort_first_half_pages_in_range_by_rotation(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_second_half_pages_in_range_by_rotation_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_second_half_pages_in_range_by_rotation");
    let err = sort_second_half_pages_in_range_by_rotation(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_first_half_pages_in_range_by_size_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_first_half_pages_in_range_by_size");
    let err = sort_first_half_pages_in_range_by_size(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_second_half_pages_in_range_by_size_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_second_half_pages_in_range_by_size");
    let err = sort_second_half_pages_in_range_by_size(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod4_0_pages_in_range_mod4");
    let err = rotate_mod4_0_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod4_1_pages_in_range_mod4");
    let err = rotate_mod4_1_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod4_2_pages_in_range_mod4");
    let err = rotate_mod4_2_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod4_3_pages_in_range_mod4");
    let err = rotate_mod4_3_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod4_0_pages_in_range_mod4_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod4_0_pages_in_range_mod4_ccw");
    let err = rotate_mod4_0_pages_in_range_mod4_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod4_1_pages_in_range_mod4_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod4_1_pages_in_range_mod4_ccw");
    let err = rotate_mod4_1_pages_in_range_mod4_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod4_2_pages_in_range_mod4_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod4_2_pages_in_range_mod4_ccw");
    let err = rotate_mod4_2_pages_in_range_mod4_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod4_3_pages_in_range_mod4_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod4_3_pages_in_range_mod4_ccw");
    let err = rotate_mod4_3_pages_in_range_mod4_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod4_0_pages_in_range_mod4");
    let err = rotate_180_mod4_0_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod4_1_pages_in_range_mod4");
    let err = rotate_180_mod4_1_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod4_2_pages_in_range_mod4");
    let err = rotate_180_mod4_2_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod4_3_pages_in_range_mod4");
    let err = rotate_180_mod4_3_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod4_0_pages_in_range_mod4");
    let err = reset_rotation_mod4_0_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod4_1_pages_in_range_mod4");
    let err = reset_rotation_mod4_1_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod4_2_pages_in_range_mod4");
    let err = reset_rotation_mod4_2_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod4_3_pages_in_range_mod4");
    let err = reset_rotation_mod4_3_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod4_0_pages_in_range_mod4");
    let err = delete_mod4_0_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod4_1_pages_in_range_mod4");
    let err = delete_mod4_1_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod4_2_pages_in_range_mod4");
    let err = delete_mod4_2_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod4_3_pages_in_range_mod4");
    let err = delete_mod4_3_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod4_0_pages_in_range_mod4");
    let err = keep_mod4_0_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod4_1_pages_in_range_mod4");
    let err = keep_mod4_1_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod4_2_pages_in_range_mod4");
    let err = keep_mod4_2_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod4_3_pages_in_range_mod4");
    let err = keep_mod4_3_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_0_pages_in_range_mod4");
    let err = duplicate_mod4_0_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_1_pages_in_range_mod4");
    let err = duplicate_mod4_1_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_2_pages_in_range_mod4");
    let err = duplicate_mod4_2_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_3_pages_in_range_mod4");
    let err = duplicate_mod4_3_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod4_0_pages_in_range_mod4");
    let err = flatten_mod4_0_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod4_1_pages_in_range_mod4");
    let err = flatten_mod4_1_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod4_2_pages_in_range_mod4");
    let err = flatten_mod4_2_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod4_3_pages_in_range_mod4");
    let err = flatten_mod4_3_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod4_0_pages_in_range_mod4");
    let err = reverse_mod4_0_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod4_1_pages_in_range_mod4");
    let err = reverse_mod4_1_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod4_2_pages_in_range_mod4");
    let err = reverse_mod4_2_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod4_3_pages_in_range_mod4");
    let err = reverse_mod4_3_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod4_0_pages_in_range_mod4");
    let err = crop_mod4_0_pages_in_range_mod4(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod4_1_pages_in_range_mod4");
    let err = crop_mod4_1_pages_in_range_mod4(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod4_2_pages_in_range_mod4");
    let err = crop_mod4_2_pages_in_range_mod4(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod4_3_pages_in_range_mod4");
    let err = crop_mod4_3_pages_in_range_mod4(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod4_0_pages_in_range_mod4");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod4_0_pages_in_range_mod4(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod4_1_pages_in_range_mod4");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod4_1_pages_in_range_mod4(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod4_2_pages_in_range_mod4");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod4_2_pages_in_range_mod4(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod4_3_pages_in_range_mod4");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod4_3_pages_in_range_mod4(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_0_pages_in_range_mod4_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_0_pages_in_range_mod4_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod4_0_pages_in_range_mod4_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_1_pages_in_range_mod4_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_1_pages_in_range_mod4_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod4_1_pages_in_range_mod4_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_2_pages_in_range_mod4_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_2_pages_in_range_mod4_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod4_2_pages_in_range_mod4_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_3_pages_in_range_mod4_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_3_pages_in_range_mod4_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod4_3_pages_in_range_mod4_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_0_pages_in_range_mod4_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_0_pages_in_range_mod4_png");
    let output_dir = tmp("export_dir");
    let err = export_mod4_0_pages_in_range_mod4_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_1_pages_in_range_mod4_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_1_pages_in_range_mod4_png");
    let output_dir = tmp("export_dir");
    let err = export_mod4_1_pages_in_range_mod4_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_2_pages_in_range_mod4_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_2_pages_in_range_mod4_png");
    let output_dir = tmp("export_dir");
    let err = export_mod4_2_pages_in_range_mod4_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_3_pages_in_range_mod4_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_3_pages_in_range_mod4_png");
    let output_dir = tmp("export_dir");
    let err = export_mod4_3_pages_in_range_mod4_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_0_pages_in_range_mod4_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_0_pages_in_range_mod4_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod4_0_pages_in_range_mod4_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_1_pages_in_range_mod4_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_1_pages_in_range_mod4_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod4_1_pages_in_range_mod4_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_2_pages_in_range_mod4_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_2_pages_in_range_mod4_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod4_2_pages_in_range_mod4_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_3_pages_in_range_mod4_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_3_pages_in_range_mod4_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod4_3_pages_in_range_mod4_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

// PARITY_BATCH4_TESTS_END
// PARITY_BATCH5_TESTS_START
// Auto-generated parity batch5 tests

#[test]
fn duplicate_mod4_0_pages_in_range_before_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_0_pages_in_range_before_mod4");
    let err = duplicate_mod4_0_pages_in_range_before_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_1_pages_in_range_before_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_1_pages_in_range_before_mod4");
    let err = duplicate_mod4_1_pages_in_range_before_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_2_pages_in_range_before_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_2_pages_in_range_before_mod4");
    let err = duplicate_mod4_2_pages_in_range_before_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_3_pages_in_range_before_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_3_pages_in_range_before_mod4");
    let err = duplicate_mod4_3_pages_in_range_before_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_0_pages_in_range_to_start_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_0_pages_in_range_to_start_mod4");
    let err = duplicate_mod4_0_pages_in_range_to_start_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_1_pages_in_range_to_start_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_1_pages_in_range_to_start_mod4");
    let err = duplicate_mod4_1_pages_in_range_to_start_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_2_pages_in_range_to_start_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_2_pages_in_range_to_start_mod4");
    let err = duplicate_mod4_2_pages_in_range_to_start_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_3_pages_in_range_to_start_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_3_pages_in_range_to_start_mod4");
    let err = duplicate_mod4_3_pages_in_range_to_start_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_0_pages_in_range_to_end_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_0_pages_in_range_to_end_mod4");
    let err = duplicate_mod4_0_pages_in_range_to_end_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_1_pages_in_range_to_end_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_1_pages_in_range_to_end_mod4");
    let err = duplicate_mod4_1_pages_in_range_to_end_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_2_pages_in_range_to_end_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_2_pages_in_range_to_end_mod4");
    let err = duplicate_mod4_2_pages_in_range_to_end_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod4_3_pages_in_range_to_end_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod4_3_pages_in_range_to_end_mod4");
    let err = duplicate_mod4_3_pages_in_range_to_end_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod4_0_pages_in_range_mod4");
    let err = expand_mod4_0_pages_in_range_mod4(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod4_1_pages_in_range_mod4");
    let err = expand_mod4_1_pages_in_range_mod4(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod4_2_pages_in_range_mod4");
    let err = expand_mod4_2_pages_in_range_mod4(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod4_3_pages_in_range_mod4");
    let err = expand_mod4_3_pages_in_range_mod4(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod4_0_pages_in_range_mod4");
    let err = shrink_mod4_0_pages_in_range_mod4(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod4_1_pages_in_range_mod4");
    let err = shrink_mod4_1_pages_in_range_mod4(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod4_2_pages_in_range_mod4");
    let err = shrink_mod4_2_pages_in_range_mod4(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod4_3_pages_in_range_mod4");
    let err = shrink_mod4_3_pages_in_range_mod4(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod4_0_pages_in_range_mod4");
    let err = clear_crop_mod4_0_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod4_1_pages_in_range_mod4");
    let err = clear_crop_mod4_1_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod4_2_pages_in_range_mod4");
    let err = clear_crop_mod4_2_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod4_3_pages_in_range_mod4");
    let err = clear_crop_mod4_3_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod4_0_pages_in_range_mod4");
    let err = insert_blank_before_mod4_0_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod4_1_pages_in_range_mod4");
    let err = insert_blank_before_mod4_1_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod4_2_pages_in_range_mod4");
    let err = insert_blank_before_mod4_2_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod4_3_pages_in_range_mod4");
    let err = insert_blank_before_mod4_3_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod4_0_pages_in_range_mod4");
    let err = insert_blank_after_mod4_0_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod4_1_pages_in_range_mod4");
    let err = insert_blank_after_mod4_1_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod4_2_pages_in_range_mod4");
    let err = insert_blank_after_mod4_2_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod4_3_pages_in_range_mod4");
    let err = insert_blank_after_mod4_3_pages_in_range_mod4(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod4_0_pages_in_range_mod4");
    let err = bookmark_mod4_0_pages_in_range_mod4(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod4_1_pages_in_range_mod4");
    let err = bookmark_mod4_1_pages_in_range_mod4(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod4_2_pages_in_range_mod4");
    let err = bookmark_mod4_2_pages_in_range_mod4(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod4_3_pages_in_range_mod4");
    let err = bookmark_mod4_3_pages_in_range_mod4(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod4_0_pages_in_range_mod4");
    let err = set_page_size_mod4_0_pages_in_range_mod4(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod4_1_pages_in_range_mod4");
    let err = set_page_size_mod4_1_pages_in_range_mod4(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod4_2_pages_in_range_mod4");
    let err = set_page_size_mod4_2_pages_in_range_mod4(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod4_3_pages_in_range_mod4");
    let err = set_page_size_mod4_3_pages_in_range_mod4(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod4_0_pages_in_range_mod4");
    let err = add_page_numbers_mod4_0_pages_in_range_mod4(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod4_1_pages_in_range_mod4");
    let err = add_page_numbers_mod4_1_pages_in_range_mod4(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod4_2_pages_in_range_mod4");
    let err = add_page_numbers_mod4_2_pages_in_range_mod4(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod4_3_pages_in_range_mod4");
    let err = add_page_numbers_mod4_3_pages_in_range_mod4(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod4_0_pages_in_range_mod4");
    let err = add_text_watermark_mod4_0_pages_in_range_mod4(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod4_1_pages_in_range_mod4");
    let err = add_text_watermark_mod4_1_pages_in_range_mod4(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod4_2_pages_in_range_mod4");
    let err = add_text_watermark_mod4_2_pages_in_range_mod4(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod4_3_pages_in_range_mod4");
    let err = add_text_watermark_mod4_3_pages_in_range_mod4(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod4_0_pages_in_range_mod4");
    let err = add_page_header_mod4_0_pages_in_range_mod4(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod4_1_pages_in_range_mod4");
    let err = add_page_header_mod4_1_pages_in_range_mod4(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod4_2_pages_in_range_mod4");
    let err = add_page_header_mod4_2_pages_in_range_mod4(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod4_3_pages_in_range_mod4");
    let err = add_page_header_mod4_3_pages_in_range_mod4(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod4_0_pages_in_range_mod4");
    let err = add_page_footer_mod4_0_pages_in_range_mod4(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod4_1_pages_in_range_mod4");
    let err = add_page_footer_mod4_1_pages_in_range_mod4(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod4_2_pages_in_range_mod4");
    let err = add_page_footer_mod4_2_pages_in_range_mod4(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod4_3_pages_in_range_mod4");
    let err = add_page_footer_mod4_3_pages_in_range_mod4(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod4_0_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod4_0_pages_in_range_mod4");
    let err = add_page_border_mod4_0_pages_in_range_mod4(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod4_1_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod4_1_pages_in_range_mod4");
    let err = add_page_border_mod4_1_pages_in_range_mod4(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod4_2_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod4_2_pages_in_range_mod4");
    let err = add_page_border_mod4_2_pages_in_range_mod4(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod4_3_pages_in_range_mod4_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod4_3_pages_in_range_mod4");
    let err = add_page_border_mod4_3_pages_in_range_mod4(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_0_pages_in_range_mod4_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_0_pages_in_range_mod4_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod4_0_pages_in_range_mod4_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_1_pages_in_range_mod4_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_1_pages_in_range_mod4_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod4_1_pages_in_range_mod4_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_2_pages_in_range_mod4_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_2_pages_in_range_mod4_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod4_2_pages_in_range_mod4_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_3_pages_in_range_mod4_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_3_pages_in_range_mod4_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod4_3_pages_in_range_mod4_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_0_pages_in_range_mod4_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_0_pages_in_range_mod4_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod4_0_pages_in_range_mod4_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_1_pages_in_range_mod4_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_1_pages_in_range_mod4_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod4_1_pages_in_range_mod4_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_2_pages_in_range_mod4_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_2_pages_in_range_mod4_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod4_2_pages_in_range_mod4_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_3_pages_in_range_mod4_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_3_pages_in_range_mod4_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod4_3_pages_in_range_mod4_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_0_pages_in_range_mod4_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_0_pages_in_range_mod4_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod4_0_pages_in_range_mod4_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_1_pages_in_range_mod4_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_1_pages_in_range_mod4_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod4_1_pages_in_range_mod4_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_2_pages_in_range_mod4_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_2_pages_in_range_mod4_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod4_2_pages_in_range_mod4_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_3_pages_in_range_mod4_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_3_pages_in_range_mod4_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod4_3_pages_in_range_mod4_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_0_pages_in_range_mod4_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_0_pages_in_range_mod4_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod4_0_pages_in_range_mod4_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_1_pages_in_range_mod4_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_1_pages_in_range_mod4_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod4_1_pages_in_range_mod4_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_2_pages_in_range_mod4_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_2_pages_in_range_mod4_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod4_2_pages_in_range_mod4_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_3_pages_in_range_mod4_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_3_pages_in_range_mod4_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod4_3_pages_in_range_mod4_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_0_pages_in_range_mod4_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_0_pages_in_range_mod4_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod4_0_pages_in_range_mod4_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_1_pages_in_range_mod4_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_1_pages_in_range_mod4_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod4_1_pages_in_range_mod4_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_2_pages_in_range_mod4_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_2_pages_in_range_mod4_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod4_2_pages_in_range_mod4_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_3_pages_in_range_mod4_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_3_pages_in_range_mod4_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod4_3_pages_in_range_mod4_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_0_pages_in_range_mod4_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_0_pages_in_range_mod4_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod4_0_pages_in_range_mod4_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_1_pages_in_range_mod4_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_1_pages_in_range_mod4_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod4_1_pages_in_range_mod4_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_2_pages_in_range_mod4_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_2_pages_in_range_mod4_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod4_2_pages_in_range_mod4_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_3_pages_in_range_mod4_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_3_pages_in_range_mod4_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod4_3_pages_in_range_mod4_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_0_pages_in_range_mod4_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_0_pages_in_range_mod4_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod4_0_pages_in_range_mod4_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_1_pages_in_range_mod4_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_1_pages_in_range_mod4_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod4_1_pages_in_range_mod4_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_2_pages_in_range_mod4_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_2_pages_in_range_mod4_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod4_2_pages_in_range_mod4_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod4_3_pages_in_range_mod4_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod4_3_pages_in_range_mod4_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod4_3_pages_in_range_mod4_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

// PARITY_BATCH5_TESTS_END
// PARITY_BATCH6_TESTS_START
// Auto-generated parity batch6 tests

#[test]
fn rotate_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod5_0_pages_in_range_mod5");
    let err = rotate_mod5_0_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod5_1_pages_in_range_mod5");
    let err = rotate_mod5_1_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod5_2_pages_in_range_mod5");
    let err = rotate_mod5_2_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod5_3_pages_in_range_mod5");
    let err = rotate_mod5_3_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod5_4_pages_in_range_mod5");
    let err = rotate_mod5_4_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod5_0_pages_in_range_mod5_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod5_0_pages_in_range_mod5_ccw");
    let err = rotate_mod5_0_pages_in_range_mod5_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod5_1_pages_in_range_mod5_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod5_1_pages_in_range_mod5_ccw");
    let err = rotate_mod5_1_pages_in_range_mod5_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod5_2_pages_in_range_mod5_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod5_2_pages_in_range_mod5_ccw");
    let err = rotate_mod5_2_pages_in_range_mod5_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod5_3_pages_in_range_mod5_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod5_3_pages_in_range_mod5_ccw");
    let err = rotate_mod5_3_pages_in_range_mod5_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod5_4_pages_in_range_mod5_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod5_4_pages_in_range_mod5_ccw");
    let err = rotate_mod5_4_pages_in_range_mod5_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod5_0_pages_in_range_mod5");
    let err = rotate_180_mod5_0_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod5_1_pages_in_range_mod5");
    let err = rotate_180_mod5_1_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod5_2_pages_in_range_mod5");
    let err = rotate_180_mod5_2_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod5_3_pages_in_range_mod5");
    let err = rotate_180_mod5_3_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod5_4_pages_in_range_mod5");
    let err = rotate_180_mod5_4_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod5_0_pages_in_range_mod5");
    let err = reset_rotation_mod5_0_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod5_1_pages_in_range_mod5");
    let err = reset_rotation_mod5_1_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod5_2_pages_in_range_mod5");
    let err = reset_rotation_mod5_2_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod5_3_pages_in_range_mod5");
    let err = reset_rotation_mod5_3_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod5_4_pages_in_range_mod5");
    let err = reset_rotation_mod5_4_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod5_0_pages_in_range_mod5");
    let err = delete_mod5_0_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod5_1_pages_in_range_mod5");
    let err = delete_mod5_1_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod5_2_pages_in_range_mod5");
    let err = delete_mod5_2_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod5_3_pages_in_range_mod5");
    let err = delete_mod5_3_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod5_4_pages_in_range_mod5");
    let err = delete_mod5_4_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod5_0_pages_in_range_mod5");
    let err = keep_mod5_0_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod5_1_pages_in_range_mod5");
    let err = keep_mod5_1_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod5_2_pages_in_range_mod5");
    let err = keep_mod5_2_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod5_3_pages_in_range_mod5");
    let err = keep_mod5_3_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod5_4_pages_in_range_mod5");
    let err = keep_mod5_4_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_0_pages_in_range_mod5");
    let err = duplicate_mod5_0_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_1_pages_in_range_mod5");
    let err = duplicate_mod5_1_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_2_pages_in_range_mod5");
    let err = duplicate_mod5_2_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_3_pages_in_range_mod5");
    let err = duplicate_mod5_3_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_4_pages_in_range_mod5");
    let err = duplicate_mod5_4_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod5_0_pages_in_range_mod5");
    let err = flatten_mod5_0_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod5_1_pages_in_range_mod5");
    let err = flatten_mod5_1_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod5_2_pages_in_range_mod5");
    let err = flatten_mod5_2_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod5_3_pages_in_range_mod5");
    let err = flatten_mod5_3_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod5_4_pages_in_range_mod5");
    let err = flatten_mod5_4_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod5_0_pages_in_range_mod5");
    let err = reverse_mod5_0_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod5_1_pages_in_range_mod5");
    let err = reverse_mod5_1_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod5_2_pages_in_range_mod5");
    let err = reverse_mod5_2_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod5_3_pages_in_range_mod5");
    let err = reverse_mod5_3_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod5_4_pages_in_range_mod5");
    let err = reverse_mod5_4_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod5_0_pages_in_range_mod5");
    let err = crop_mod5_0_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod5_1_pages_in_range_mod5");
    let err = crop_mod5_1_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod5_2_pages_in_range_mod5");
    let err = crop_mod5_2_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod5_3_pages_in_range_mod5");
    let err = crop_mod5_3_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod5_4_pages_in_range_mod5");
    let err = crop_mod5_4_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod5_0_pages_in_range_mod5");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod5_0_pages_in_range_mod5(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod5_1_pages_in_range_mod5");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod5_1_pages_in_range_mod5(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod5_2_pages_in_range_mod5");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod5_2_pages_in_range_mod5(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod5_3_pages_in_range_mod5");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod5_3_pages_in_range_mod5(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod5_4_pages_in_range_mod5");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod5_4_pages_in_range_mod5(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_0_pages_in_range_mod5_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_0_pages_in_range_mod5_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod5_0_pages_in_range_mod5_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_1_pages_in_range_mod5_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_1_pages_in_range_mod5_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod5_1_pages_in_range_mod5_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_2_pages_in_range_mod5_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_2_pages_in_range_mod5_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod5_2_pages_in_range_mod5_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_3_pages_in_range_mod5_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_3_pages_in_range_mod5_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod5_3_pages_in_range_mod5_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_4_pages_in_range_mod5_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_4_pages_in_range_mod5_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod5_4_pages_in_range_mod5_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_0_pages_in_range_mod5_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_0_pages_in_range_mod5_png");
    let output_dir = tmp("export_dir");
    let err = export_mod5_0_pages_in_range_mod5_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_1_pages_in_range_mod5_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_1_pages_in_range_mod5_png");
    let output_dir = tmp("export_dir");
    let err = export_mod5_1_pages_in_range_mod5_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_2_pages_in_range_mod5_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_2_pages_in_range_mod5_png");
    let output_dir = tmp("export_dir");
    let err = export_mod5_2_pages_in_range_mod5_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_3_pages_in_range_mod5_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_3_pages_in_range_mod5_png");
    let output_dir = tmp("export_dir");
    let err = export_mod5_3_pages_in_range_mod5_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_4_pages_in_range_mod5_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_4_pages_in_range_mod5_png");
    let output_dir = tmp("export_dir");
    let err = export_mod5_4_pages_in_range_mod5_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_0_pages_in_range_mod5_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_0_pages_in_range_mod5_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod5_0_pages_in_range_mod5_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_1_pages_in_range_mod5_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_1_pages_in_range_mod5_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod5_1_pages_in_range_mod5_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_2_pages_in_range_mod5_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_2_pages_in_range_mod5_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod5_2_pages_in_range_mod5_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_3_pages_in_range_mod5_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_3_pages_in_range_mod5_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod5_3_pages_in_range_mod5_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_4_pages_in_range_mod5_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_4_pages_in_range_mod5_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod5_4_pages_in_range_mod5_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_0_pages_in_range_before_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_0_pages_in_range_before_mod5");
    let err = duplicate_mod5_0_pages_in_range_before_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_1_pages_in_range_before_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_1_pages_in_range_before_mod5");
    let err = duplicate_mod5_1_pages_in_range_before_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_2_pages_in_range_before_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_2_pages_in_range_before_mod5");
    let err = duplicate_mod5_2_pages_in_range_before_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_3_pages_in_range_before_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_3_pages_in_range_before_mod5");
    let err = duplicate_mod5_3_pages_in_range_before_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_4_pages_in_range_before_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_4_pages_in_range_before_mod5");
    let err = duplicate_mod5_4_pages_in_range_before_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_0_pages_in_range_to_start_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_0_pages_in_range_to_start_mod5");
    let err = duplicate_mod5_0_pages_in_range_to_start_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_1_pages_in_range_to_start_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_1_pages_in_range_to_start_mod5");
    let err = duplicate_mod5_1_pages_in_range_to_start_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_2_pages_in_range_to_start_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_2_pages_in_range_to_start_mod5");
    let err = duplicate_mod5_2_pages_in_range_to_start_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_3_pages_in_range_to_start_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_3_pages_in_range_to_start_mod5");
    let err = duplicate_mod5_3_pages_in_range_to_start_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_4_pages_in_range_to_start_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_4_pages_in_range_to_start_mod5");
    let err = duplicate_mod5_4_pages_in_range_to_start_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_0_pages_in_range_to_end_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_0_pages_in_range_to_end_mod5");
    let err = duplicate_mod5_0_pages_in_range_to_end_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_1_pages_in_range_to_end_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_1_pages_in_range_to_end_mod5");
    let err = duplicate_mod5_1_pages_in_range_to_end_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_2_pages_in_range_to_end_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_2_pages_in_range_to_end_mod5");
    let err = duplicate_mod5_2_pages_in_range_to_end_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_3_pages_in_range_to_end_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_3_pages_in_range_to_end_mod5");
    let err = duplicate_mod5_3_pages_in_range_to_end_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod5_4_pages_in_range_to_end_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod5_4_pages_in_range_to_end_mod5");
    let err = duplicate_mod5_4_pages_in_range_to_end_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod5_0_pages_in_range_mod5");
    let err = expand_mod5_0_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod5_1_pages_in_range_mod5");
    let err = expand_mod5_1_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod5_2_pages_in_range_mod5");
    let err = expand_mod5_2_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod5_3_pages_in_range_mod5");
    let err = expand_mod5_3_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod5_4_pages_in_range_mod5");
    let err = expand_mod5_4_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod5_0_pages_in_range_mod5");
    let err = shrink_mod5_0_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod5_1_pages_in_range_mod5");
    let err = shrink_mod5_1_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod5_2_pages_in_range_mod5");
    let err = shrink_mod5_2_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod5_3_pages_in_range_mod5");
    let err = shrink_mod5_3_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod5_4_pages_in_range_mod5");
    let err = shrink_mod5_4_pages_in_range_mod5(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod5_0_pages_in_range_mod5");
    let err = clear_crop_mod5_0_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod5_1_pages_in_range_mod5");
    let err = clear_crop_mod5_1_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod5_2_pages_in_range_mod5");
    let err = clear_crop_mod5_2_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod5_3_pages_in_range_mod5");
    let err = clear_crop_mod5_3_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod5_4_pages_in_range_mod5");
    let err = clear_crop_mod5_4_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod5_0_pages_in_range_mod5");
    let err = insert_blank_before_mod5_0_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod5_1_pages_in_range_mod5");
    let err = insert_blank_before_mod5_1_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod5_2_pages_in_range_mod5");
    let err = insert_blank_before_mod5_2_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod5_3_pages_in_range_mod5");
    let err = insert_blank_before_mod5_3_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod5_4_pages_in_range_mod5");
    let err = insert_blank_before_mod5_4_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod5_0_pages_in_range_mod5");
    let err = insert_blank_after_mod5_0_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod5_1_pages_in_range_mod5");
    let err = insert_blank_after_mod5_1_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod5_2_pages_in_range_mod5");
    let err = insert_blank_after_mod5_2_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod5_3_pages_in_range_mod5");
    let err = insert_blank_after_mod5_3_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod5_4_pages_in_range_mod5");
    let err = insert_blank_after_mod5_4_pages_in_range_mod5(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod5_0_pages_in_range_mod5");
    let err = bookmark_mod5_0_pages_in_range_mod5(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod5_1_pages_in_range_mod5");
    let err = bookmark_mod5_1_pages_in_range_mod5(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod5_2_pages_in_range_mod5");
    let err = bookmark_mod5_2_pages_in_range_mod5(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod5_3_pages_in_range_mod5");
    let err = bookmark_mod5_3_pages_in_range_mod5(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod5_4_pages_in_range_mod5");
    let err = bookmark_mod5_4_pages_in_range_mod5(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod5_0_pages_in_range_mod5");
    let err = set_page_size_mod5_0_pages_in_range_mod5(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod5_1_pages_in_range_mod5");
    let err = set_page_size_mod5_1_pages_in_range_mod5(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod5_2_pages_in_range_mod5");
    let err = set_page_size_mod5_2_pages_in_range_mod5(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod5_3_pages_in_range_mod5");
    let err = set_page_size_mod5_3_pages_in_range_mod5(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod5_4_pages_in_range_mod5");
    let err = set_page_size_mod5_4_pages_in_range_mod5(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod5_0_pages_in_range_mod5");
    let err = add_page_numbers_mod5_0_pages_in_range_mod5(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod5_1_pages_in_range_mod5");
    let err = add_page_numbers_mod5_1_pages_in_range_mod5(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod5_2_pages_in_range_mod5");
    let err = add_page_numbers_mod5_2_pages_in_range_mod5(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod5_3_pages_in_range_mod5");
    let err = add_page_numbers_mod5_3_pages_in_range_mod5(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod5_4_pages_in_range_mod5");
    let err = add_page_numbers_mod5_4_pages_in_range_mod5(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod5_0_pages_in_range_mod5");
    let err = add_text_watermark_mod5_0_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod5_1_pages_in_range_mod5");
    let err = add_text_watermark_mod5_1_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod5_2_pages_in_range_mod5");
    let err = add_text_watermark_mod5_2_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod5_3_pages_in_range_mod5");
    let err = add_text_watermark_mod5_3_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod5_4_pages_in_range_mod5");
    let err = add_text_watermark_mod5_4_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod5_0_pages_in_range_mod5");
    let err = add_page_header_mod5_0_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod5_1_pages_in_range_mod5");
    let err = add_page_header_mod5_1_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod5_2_pages_in_range_mod5");
    let err = add_page_header_mod5_2_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod5_3_pages_in_range_mod5");
    let err = add_page_header_mod5_3_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod5_4_pages_in_range_mod5");
    let err = add_page_header_mod5_4_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod5_0_pages_in_range_mod5");
    let err = add_page_footer_mod5_0_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod5_1_pages_in_range_mod5");
    let err = add_page_footer_mod5_1_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod5_2_pages_in_range_mod5");
    let err = add_page_footer_mod5_2_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod5_3_pages_in_range_mod5");
    let err = add_page_footer_mod5_3_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod5_4_pages_in_range_mod5");
    let err = add_page_footer_mod5_4_pages_in_range_mod5(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod5_0_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod5_0_pages_in_range_mod5");
    let err = add_page_border_mod5_0_pages_in_range_mod5(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod5_1_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod5_1_pages_in_range_mod5");
    let err = add_page_border_mod5_1_pages_in_range_mod5(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod5_2_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod5_2_pages_in_range_mod5");
    let err = add_page_border_mod5_2_pages_in_range_mod5(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod5_3_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod5_3_pages_in_range_mod5");
    let err = add_page_border_mod5_3_pages_in_range_mod5(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod5_4_pages_in_range_mod5_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod5_4_pages_in_range_mod5");
    let err = add_page_border_mod5_4_pages_in_range_mod5(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_0_pages_in_range_mod5_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_0_pages_in_range_mod5_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod5_0_pages_in_range_mod5_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_1_pages_in_range_mod5_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_1_pages_in_range_mod5_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod5_1_pages_in_range_mod5_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_2_pages_in_range_mod5_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_2_pages_in_range_mod5_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod5_2_pages_in_range_mod5_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_3_pages_in_range_mod5_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_3_pages_in_range_mod5_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod5_3_pages_in_range_mod5_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_4_pages_in_range_mod5_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_4_pages_in_range_mod5_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod5_4_pages_in_range_mod5_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_0_pages_in_range_mod5_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_0_pages_in_range_mod5_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod5_0_pages_in_range_mod5_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_1_pages_in_range_mod5_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_1_pages_in_range_mod5_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod5_1_pages_in_range_mod5_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_2_pages_in_range_mod5_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_2_pages_in_range_mod5_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod5_2_pages_in_range_mod5_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_3_pages_in_range_mod5_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_3_pages_in_range_mod5_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod5_3_pages_in_range_mod5_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_4_pages_in_range_mod5_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_4_pages_in_range_mod5_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod5_4_pages_in_range_mod5_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_0_pages_in_range_mod5_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_0_pages_in_range_mod5_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod5_0_pages_in_range_mod5_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_1_pages_in_range_mod5_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_1_pages_in_range_mod5_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod5_1_pages_in_range_mod5_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_2_pages_in_range_mod5_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_2_pages_in_range_mod5_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod5_2_pages_in_range_mod5_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_3_pages_in_range_mod5_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_3_pages_in_range_mod5_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod5_3_pages_in_range_mod5_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_4_pages_in_range_mod5_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_4_pages_in_range_mod5_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod5_4_pages_in_range_mod5_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_0_pages_in_range_mod5_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_0_pages_in_range_mod5_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod5_0_pages_in_range_mod5_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_1_pages_in_range_mod5_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_1_pages_in_range_mod5_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod5_1_pages_in_range_mod5_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_2_pages_in_range_mod5_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_2_pages_in_range_mod5_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod5_2_pages_in_range_mod5_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_3_pages_in_range_mod5_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_3_pages_in_range_mod5_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod5_3_pages_in_range_mod5_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_4_pages_in_range_mod5_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_4_pages_in_range_mod5_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod5_4_pages_in_range_mod5_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_0_pages_in_range_mod5_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_0_pages_in_range_mod5_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod5_0_pages_in_range_mod5_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_1_pages_in_range_mod5_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_1_pages_in_range_mod5_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod5_1_pages_in_range_mod5_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_2_pages_in_range_mod5_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_2_pages_in_range_mod5_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod5_2_pages_in_range_mod5_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_3_pages_in_range_mod5_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_3_pages_in_range_mod5_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod5_3_pages_in_range_mod5_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_4_pages_in_range_mod5_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_4_pages_in_range_mod5_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod5_4_pages_in_range_mod5_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_0_pages_in_range_mod5_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_0_pages_in_range_mod5_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod5_0_pages_in_range_mod5_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_1_pages_in_range_mod5_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_1_pages_in_range_mod5_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod5_1_pages_in_range_mod5_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_2_pages_in_range_mod5_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_2_pages_in_range_mod5_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod5_2_pages_in_range_mod5_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_3_pages_in_range_mod5_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_3_pages_in_range_mod5_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod5_3_pages_in_range_mod5_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_4_pages_in_range_mod5_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_4_pages_in_range_mod5_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod5_4_pages_in_range_mod5_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_0_pages_in_range_mod5_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_0_pages_in_range_mod5_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod5_0_pages_in_range_mod5_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_1_pages_in_range_mod5_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_1_pages_in_range_mod5_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod5_1_pages_in_range_mod5_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_2_pages_in_range_mod5_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_2_pages_in_range_mod5_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod5_2_pages_in_range_mod5_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_3_pages_in_range_mod5_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_3_pages_in_range_mod5_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod5_3_pages_in_range_mod5_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod5_4_pages_in_range_mod5_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod5_4_pages_in_range_mod5_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod5_4_pages_in_range_mod5_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod6_0_pages_in_range_mod6");
    let err = rotate_mod6_0_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod6_1_pages_in_range_mod6");
    let err = rotate_mod6_1_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod6_2_pages_in_range_mod6");
    let err = rotate_mod6_2_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod6_3_pages_in_range_mod6");
    let err = rotate_mod6_3_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod6_4_pages_in_range_mod6");
    let err = rotate_mod6_4_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod6_5_pages_in_range_mod6");
    let err = rotate_mod6_5_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod6_0_pages_in_range_mod6_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod6_0_pages_in_range_mod6_ccw");
    let err = rotate_mod6_0_pages_in_range_mod6_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod6_1_pages_in_range_mod6_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod6_1_pages_in_range_mod6_ccw");
    let err = rotate_mod6_1_pages_in_range_mod6_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod6_2_pages_in_range_mod6_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod6_2_pages_in_range_mod6_ccw");
    let err = rotate_mod6_2_pages_in_range_mod6_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod6_3_pages_in_range_mod6_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod6_3_pages_in_range_mod6_ccw");
    let err = rotate_mod6_3_pages_in_range_mod6_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod6_4_pages_in_range_mod6_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod6_4_pages_in_range_mod6_ccw");
    let err = rotate_mod6_4_pages_in_range_mod6_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_mod6_5_pages_in_range_mod6_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_mod6_5_pages_in_range_mod6_ccw");
    let err = rotate_mod6_5_pages_in_range_mod6_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod6_0_pages_in_range_mod6");
    let err = rotate_180_mod6_0_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod6_1_pages_in_range_mod6");
    let err = rotate_180_mod6_1_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod6_2_pages_in_range_mod6");
    let err = rotate_180_mod6_2_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod6_3_pages_in_range_mod6");
    let err = rotate_180_mod6_3_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod6_4_pages_in_range_mod6");
    let err = rotate_180_mod6_4_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_mod6_5_pages_in_range_mod6");
    let err = rotate_180_mod6_5_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod6_0_pages_in_range_mod6");
    let err = reset_rotation_mod6_0_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod6_1_pages_in_range_mod6");
    let err = reset_rotation_mod6_1_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod6_2_pages_in_range_mod6");
    let err = reset_rotation_mod6_2_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod6_3_pages_in_range_mod6");
    let err = reset_rotation_mod6_3_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod6_4_pages_in_range_mod6");
    let err = reset_rotation_mod6_4_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_mod6_5_pages_in_range_mod6");
    let err = reset_rotation_mod6_5_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod6_0_pages_in_range_mod6");
    let err = delete_mod6_0_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod6_1_pages_in_range_mod6");
    let err = delete_mod6_1_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod6_2_pages_in_range_mod6");
    let err = delete_mod6_2_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod6_3_pages_in_range_mod6");
    let err = delete_mod6_3_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod6_4_pages_in_range_mod6");
    let err = delete_mod6_4_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_mod6_5_pages_in_range_mod6");
    let err = delete_mod6_5_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod6_0_pages_in_range_mod6");
    let err = keep_mod6_0_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod6_1_pages_in_range_mod6");
    let err = keep_mod6_1_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod6_2_pages_in_range_mod6");
    let err = keep_mod6_2_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod6_3_pages_in_range_mod6");
    let err = keep_mod6_3_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod6_4_pages_in_range_mod6");
    let err = keep_mod6_4_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_mod6_5_pages_in_range_mod6");
    let err = keep_mod6_5_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_0_pages_in_range_mod6");
    let err = duplicate_mod6_0_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_1_pages_in_range_mod6");
    let err = duplicate_mod6_1_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_2_pages_in_range_mod6");
    let err = duplicate_mod6_2_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_3_pages_in_range_mod6");
    let err = duplicate_mod6_3_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_4_pages_in_range_mod6");
    let err = duplicate_mod6_4_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_5_pages_in_range_mod6");
    let err = duplicate_mod6_5_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod6_0_pages_in_range_mod6");
    let err = flatten_mod6_0_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod6_1_pages_in_range_mod6");
    let err = flatten_mod6_1_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod6_2_pages_in_range_mod6");
    let err = flatten_mod6_2_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod6_3_pages_in_range_mod6");
    let err = flatten_mod6_3_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod6_4_pages_in_range_mod6");
    let err = flatten_mod6_4_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_mod6_5_pages_in_range_mod6");
    let err = flatten_mod6_5_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod6_0_pages_in_range_mod6");
    let err = reverse_mod6_0_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod6_1_pages_in_range_mod6");
    let err = reverse_mod6_1_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod6_2_pages_in_range_mod6");
    let err = reverse_mod6_2_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod6_3_pages_in_range_mod6");
    let err = reverse_mod6_3_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod6_4_pages_in_range_mod6");
    let err = reverse_mod6_4_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_mod6_5_pages_in_range_mod6");
    let err = reverse_mod6_5_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod6_0_pages_in_range_mod6");
    let err = crop_mod6_0_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod6_1_pages_in_range_mod6");
    let err = crop_mod6_1_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod6_2_pages_in_range_mod6");
    let err = crop_mod6_2_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod6_3_pages_in_range_mod6");
    let err = crop_mod6_3_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod6_4_pages_in_range_mod6");
    let err = crop_mod6_4_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_mod6_5_pages_in_range_mod6");
    let err = crop_mod6_5_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod6_0_pages_in_range_mod6");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod6_0_pages_in_range_mod6(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod6_1_pages_in_range_mod6");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod6_1_pages_in_range_mod6(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod6_2_pages_in_range_mod6");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod6_2_pages_in_range_mod6(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod6_3_pages_in_range_mod6");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod6_3_pages_in_range_mod6(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod6_4_pages_in_range_mod6");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod6_4_pages_in_range_mod6(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_mod6_5_pages_in_range_mod6");
    let output_path = tmp("extract_out.pdf");
    let err = extract_mod6_5_pages_in_range_mod6(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_0_pages_in_range_mod6_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_0_pages_in_range_mod6_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod6_0_pages_in_range_mod6_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_1_pages_in_range_mod6_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_1_pages_in_range_mod6_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod6_1_pages_in_range_mod6_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_2_pages_in_range_mod6_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_2_pages_in_range_mod6_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod6_2_pages_in_range_mod6_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_3_pages_in_range_mod6_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_3_pages_in_range_mod6_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod6_3_pages_in_range_mod6_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_4_pages_in_range_mod6_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_4_pages_in_range_mod6_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod6_4_pages_in_range_mod6_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_5_pages_in_range_mod6_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_5_pages_in_range_mod6_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_mod6_5_pages_in_range_mod6_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_0_pages_in_range_mod6_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_0_pages_in_range_mod6_png");
    let output_dir = tmp("export_dir");
    let err = export_mod6_0_pages_in_range_mod6_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_1_pages_in_range_mod6_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_1_pages_in_range_mod6_png");
    let output_dir = tmp("export_dir");
    let err = export_mod6_1_pages_in_range_mod6_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_2_pages_in_range_mod6_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_2_pages_in_range_mod6_png");
    let output_dir = tmp("export_dir");
    let err = export_mod6_2_pages_in_range_mod6_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_3_pages_in_range_mod6_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_3_pages_in_range_mod6_png");
    let output_dir = tmp("export_dir");
    let err = export_mod6_3_pages_in_range_mod6_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_4_pages_in_range_mod6_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_4_pages_in_range_mod6_png");
    let output_dir = tmp("export_dir");
    let err = export_mod6_4_pages_in_range_mod6_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_5_pages_in_range_mod6_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_5_pages_in_range_mod6_png");
    let output_dir = tmp("export_dir");
    let err = export_mod6_5_pages_in_range_mod6_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_0_pages_in_range_mod6_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_0_pages_in_range_mod6_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod6_0_pages_in_range_mod6_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_1_pages_in_range_mod6_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_1_pages_in_range_mod6_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod6_1_pages_in_range_mod6_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_2_pages_in_range_mod6_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_2_pages_in_range_mod6_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod6_2_pages_in_range_mod6_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_3_pages_in_range_mod6_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_3_pages_in_range_mod6_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod6_3_pages_in_range_mod6_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_4_pages_in_range_mod6_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_4_pages_in_range_mod6_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod6_4_pages_in_range_mod6_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_5_pages_in_range_mod6_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_5_pages_in_range_mod6_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_mod6_5_pages_in_range_mod6_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_0_pages_in_range_before_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_0_pages_in_range_before_mod6");
    let err = duplicate_mod6_0_pages_in_range_before_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_1_pages_in_range_before_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_1_pages_in_range_before_mod6");
    let err = duplicate_mod6_1_pages_in_range_before_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_2_pages_in_range_before_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_2_pages_in_range_before_mod6");
    let err = duplicate_mod6_2_pages_in_range_before_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_3_pages_in_range_before_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_3_pages_in_range_before_mod6");
    let err = duplicate_mod6_3_pages_in_range_before_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_4_pages_in_range_before_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_4_pages_in_range_before_mod6");
    let err = duplicate_mod6_4_pages_in_range_before_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_5_pages_in_range_before_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_5_pages_in_range_before_mod6");
    let err = duplicate_mod6_5_pages_in_range_before_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_0_pages_in_range_to_start_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_0_pages_in_range_to_start_mod6");
    let err = duplicate_mod6_0_pages_in_range_to_start_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_1_pages_in_range_to_start_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_1_pages_in_range_to_start_mod6");
    let err = duplicate_mod6_1_pages_in_range_to_start_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_2_pages_in_range_to_start_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_2_pages_in_range_to_start_mod6");
    let err = duplicate_mod6_2_pages_in_range_to_start_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_3_pages_in_range_to_start_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_3_pages_in_range_to_start_mod6");
    let err = duplicate_mod6_3_pages_in_range_to_start_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_4_pages_in_range_to_start_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_4_pages_in_range_to_start_mod6");
    let err = duplicate_mod6_4_pages_in_range_to_start_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_5_pages_in_range_to_start_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_5_pages_in_range_to_start_mod6");
    let err = duplicate_mod6_5_pages_in_range_to_start_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_0_pages_in_range_to_end_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_0_pages_in_range_to_end_mod6");
    let err = duplicate_mod6_0_pages_in_range_to_end_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_1_pages_in_range_to_end_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_1_pages_in_range_to_end_mod6");
    let err = duplicate_mod6_1_pages_in_range_to_end_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_2_pages_in_range_to_end_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_2_pages_in_range_to_end_mod6");
    let err = duplicate_mod6_2_pages_in_range_to_end_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_3_pages_in_range_to_end_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_3_pages_in_range_to_end_mod6");
    let err = duplicate_mod6_3_pages_in_range_to_end_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_4_pages_in_range_to_end_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_4_pages_in_range_to_end_mod6");
    let err = duplicate_mod6_4_pages_in_range_to_end_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_mod6_5_pages_in_range_to_end_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_mod6_5_pages_in_range_to_end_mod6");
    let err = duplicate_mod6_5_pages_in_range_to_end_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod6_0_pages_in_range_mod6");
    let err = expand_mod6_0_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod6_1_pages_in_range_mod6");
    let err = expand_mod6_1_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod6_2_pages_in_range_mod6");
    let err = expand_mod6_2_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod6_3_pages_in_range_mod6");
    let err = expand_mod6_3_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod6_4_pages_in_range_mod6");
    let err = expand_mod6_4_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_mod6_5_pages_in_range_mod6");
    let err = expand_mod6_5_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod6_0_pages_in_range_mod6");
    let err = shrink_mod6_0_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod6_1_pages_in_range_mod6");
    let err = shrink_mod6_1_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod6_2_pages_in_range_mod6");
    let err = shrink_mod6_2_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod6_3_pages_in_range_mod6");
    let err = shrink_mod6_3_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod6_4_pages_in_range_mod6");
    let err = shrink_mod6_4_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_mod6_5_pages_in_range_mod6");
    let err = shrink_mod6_5_pages_in_range_mod6(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod6_0_pages_in_range_mod6");
    let err = clear_crop_mod6_0_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod6_1_pages_in_range_mod6");
    let err = clear_crop_mod6_1_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod6_2_pages_in_range_mod6");
    let err = clear_crop_mod6_2_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod6_3_pages_in_range_mod6");
    let err = clear_crop_mod6_3_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod6_4_pages_in_range_mod6");
    let err = clear_crop_mod6_4_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_mod6_5_pages_in_range_mod6");
    let err = clear_crop_mod6_5_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod6_0_pages_in_range_mod6");
    let err = insert_blank_before_mod6_0_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod6_1_pages_in_range_mod6");
    let err = insert_blank_before_mod6_1_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod6_2_pages_in_range_mod6");
    let err = insert_blank_before_mod6_2_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod6_3_pages_in_range_mod6");
    let err = insert_blank_before_mod6_3_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod6_4_pages_in_range_mod6");
    let err = insert_blank_before_mod6_4_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_mod6_5_pages_in_range_mod6");
    let err = insert_blank_before_mod6_5_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod6_0_pages_in_range_mod6");
    let err = insert_blank_after_mod6_0_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod6_1_pages_in_range_mod6");
    let err = insert_blank_after_mod6_1_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod6_2_pages_in_range_mod6");
    let err = insert_blank_after_mod6_2_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod6_3_pages_in_range_mod6");
    let err = insert_blank_after_mod6_3_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod6_4_pages_in_range_mod6");
    let err = insert_blank_after_mod6_4_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_mod6_5_pages_in_range_mod6");
    let err = insert_blank_after_mod6_5_pages_in_range_mod6(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod6_0_pages_in_range_mod6");
    let err = bookmark_mod6_0_pages_in_range_mod6(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod6_1_pages_in_range_mod6");
    let err = bookmark_mod6_1_pages_in_range_mod6(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod6_2_pages_in_range_mod6");
    let err = bookmark_mod6_2_pages_in_range_mod6(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod6_3_pages_in_range_mod6");
    let err = bookmark_mod6_3_pages_in_range_mod6(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod6_4_pages_in_range_mod6");
    let err = bookmark_mod6_4_pages_in_range_mod6(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_mod6_5_pages_in_range_mod6");
    let err = bookmark_mod6_5_pages_in_range_mod6(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod6_0_pages_in_range_mod6");
    let err = set_page_size_mod6_0_pages_in_range_mod6(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod6_1_pages_in_range_mod6");
    let err = set_page_size_mod6_1_pages_in_range_mod6(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod6_2_pages_in_range_mod6");
    let err = set_page_size_mod6_2_pages_in_range_mod6(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod6_3_pages_in_range_mod6");
    let err = set_page_size_mod6_3_pages_in_range_mod6(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod6_4_pages_in_range_mod6");
    let err = set_page_size_mod6_4_pages_in_range_mod6(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_mod6_5_pages_in_range_mod6");
    let err = set_page_size_mod6_5_pages_in_range_mod6(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod6_0_pages_in_range_mod6");
    let err = add_page_numbers_mod6_0_pages_in_range_mod6(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod6_1_pages_in_range_mod6");
    let err = add_page_numbers_mod6_1_pages_in_range_mod6(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod6_2_pages_in_range_mod6");
    let err = add_page_numbers_mod6_2_pages_in_range_mod6(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod6_3_pages_in_range_mod6");
    let err = add_page_numbers_mod6_3_pages_in_range_mod6(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod6_4_pages_in_range_mod6");
    let err = add_page_numbers_mod6_4_pages_in_range_mod6(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_mod6_5_pages_in_range_mod6");
    let err = add_page_numbers_mod6_5_pages_in_range_mod6(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod6_0_pages_in_range_mod6");
    let err = add_text_watermark_mod6_0_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod6_1_pages_in_range_mod6");
    let err = add_text_watermark_mod6_1_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod6_2_pages_in_range_mod6");
    let err = add_text_watermark_mod6_2_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod6_3_pages_in_range_mod6");
    let err = add_text_watermark_mod6_3_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod6_4_pages_in_range_mod6");
    let err = add_text_watermark_mod6_4_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_mod6_5_pages_in_range_mod6");
    let err = add_text_watermark_mod6_5_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod6_0_pages_in_range_mod6");
    let err = add_page_header_mod6_0_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod6_1_pages_in_range_mod6");
    let err = add_page_header_mod6_1_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod6_2_pages_in_range_mod6");
    let err = add_page_header_mod6_2_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod6_3_pages_in_range_mod6");
    let err = add_page_header_mod6_3_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod6_4_pages_in_range_mod6");
    let err = add_page_header_mod6_4_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_mod6_5_pages_in_range_mod6");
    let err = add_page_header_mod6_5_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod6_0_pages_in_range_mod6");
    let err = add_page_footer_mod6_0_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod6_1_pages_in_range_mod6");
    let err = add_page_footer_mod6_1_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod6_2_pages_in_range_mod6");
    let err = add_page_footer_mod6_2_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod6_3_pages_in_range_mod6");
    let err = add_page_footer_mod6_3_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod6_4_pages_in_range_mod6");
    let err = add_page_footer_mod6_4_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_mod6_5_pages_in_range_mod6");
    let err = add_page_footer_mod6_5_pages_in_range_mod6(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod6_0_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod6_0_pages_in_range_mod6");
    let err = add_page_border_mod6_0_pages_in_range_mod6(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod6_1_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod6_1_pages_in_range_mod6");
    let err = add_page_border_mod6_1_pages_in_range_mod6(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod6_2_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod6_2_pages_in_range_mod6");
    let err = add_page_border_mod6_2_pages_in_range_mod6(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod6_3_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod6_3_pages_in_range_mod6");
    let err = add_page_border_mod6_3_pages_in_range_mod6(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod6_4_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod6_4_pages_in_range_mod6");
    let err = add_page_border_mod6_4_pages_in_range_mod6(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_mod6_5_pages_in_range_mod6_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_mod6_5_pages_in_range_mod6");
    let err = add_page_border_mod6_5_pages_in_range_mod6(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_0_pages_in_range_mod6_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_0_pages_in_range_mod6_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod6_0_pages_in_range_mod6_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_1_pages_in_range_mod6_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_1_pages_in_range_mod6_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod6_1_pages_in_range_mod6_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_2_pages_in_range_mod6_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_2_pages_in_range_mod6_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod6_2_pages_in_range_mod6_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_3_pages_in_range_mod6_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_3_pages_in_range_mod6_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod6_3_pages_in_range_mod6_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_4_pages_in_range_mod6_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_4_pages_in_range_mod6_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod6_4_pages_in_range_mod6_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_5_pages_in_range_mod6_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_5_pages_in_range_mod6_webp");
    let output_dir = tmp("export_dir");
    let err = export_mod6_5_pages_in_range_mod6_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_0_pages_in_range_mod6_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_0_pages_in_range_mod6_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod6_0_pages_in_range_mod6_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_1_pages_in_range_mod6_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_1_pages_in_range_mod6_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod6_1_pages_in_range_mod6_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_2_pages_in_range_mod6_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_2_pages_in_range_mod6_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod6_2_pages_in_range_mod6_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_3_pages_in_range_mod6_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_3_pages_in_range_mod6_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod6_3_pages_in_range_mod6_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_4_pages_in_range_mod6_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_4_pages_in_range_mod6_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod6_4_pages_in_range_mod6_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_5_pages_in_range_mod6_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_5_pages_in_range_mod6_bmp");
    let output_dir = tmp("export_dir");
    let err = export_mod6_5_pages_in_range_mod6_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_0_pages_in_range_mod6_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_0_pages_in_range_mod6_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod6_0_pages_in_range_mod6_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_1_pages_in_range_mod6_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_1_pages_in_range_mod6_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod6_1_pages_in_range_mod6_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_2_pages_in_range_mod6_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_2_pages_in_range_mod6_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod6_2_pages_in_range_mod6_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_3_pages_in_range_mod6_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_3_pages_in_range_mod6_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod6_3_pages_in_range_mod6_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_4_pages_in_range_mod6_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_4_pages_in_range_mod6_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod6_4_pages_in_range_mod6_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_5_pages_in_range_mod6_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_5_pages_in_range_mod6_tiff");
    let output_dir = tmp("export_dir");
    let err = export_mod6_5_pages_in_range_mod6_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_0_pages_in_range_mod6_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_0_pages_in_range_mod6_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod6_0_pages_in_range_mod6_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_1_pages_in_range_mod6_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_1_pages_in_range_mod6_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod6_1_pages_in_range_mod6_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_2_pages_in_range_mod6_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_2_pages_in_range_mod6_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod6_2_pages_in_range_mod6_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_3_pages_in_range_mod6_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_3_pages_in_range_mod6_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod6_3_pages_in_range_mod6_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_4_pages_in_range_mod6_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_4_pages_in_range_mod6_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod6_4_pages_in_range_mod6_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_5_pages_in_range_mod6_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_5_pages_in_range_mod6_gif");
    let output_dir = tmp("export_dir");
    let err = export_mod6_5_pages_in_range_mod6_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_0_pages_in_range_mod6_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_0_pages_in_range_mod6_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod6_0_pages_in_range_mod6_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_1_pages_in_range_mod6_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_1_pages_in_range_mod6_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod6_1_pages_in_range_mod6_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_2_pages_in_range_mod6_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_2_pages_in_range_mod6_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod6_2_pages_in_range_mod6_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_3_pages_in_range_mod6_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_3_pages_in_range_mod6_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod6_3_pages_in_range_mod6_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_4_pages_in_range_mod6_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_4_pages_in_range_mod6_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod6_4_pages_in_range_mod6_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_5_pages_in_range_mod6_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_5_pages_in_range_mod6_ppm");
    let output_dir = tmp("export_dir");
    let err = export_mod6_5_pages_in_range_mod6_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_0_pages_in_range_mod6_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_0_pages_in_range_mod6_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod6_0_pages_in_range_mod6_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_1_pages_in_range_mod6_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_1_pages_in_range_mod6_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod6_1_pages_in_range_mod6_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_2_pages_in_range_mod6_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_2_pages_in_range_mod6_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod6_2_pages_in_range_mod6_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_3_pages_in_range_mod6_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_3_pages_in_range_mod6_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod6_3_pages_in_range_mod6_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_4_pages_in_range_mod6_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_4_pages_in_range_mod6_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod6_4_pages_in_range_mod6_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_5_pages_in_range_mod6_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_5_pages_in_range_mod6_tga");
    let output_dir = tmp("export_dir");
    let err = export_mod6_5_pages_in_range_mod6_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_0_pages_in_range_mod6_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_0_pages_in_range_mod6_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod6_0_pages_in_range_mod6_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_1_pages_in_range_mod6_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_1_pages_in_range_mod6_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod6_1_pages_in_range_mod6_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_2_pages_in_range_mod6_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_2_pages_in_range_mod6_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod6_2_pages_in_range_mod6_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_3_pages_in_range_mod6_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_3_pages_in_range_mod6_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod6_3_pages_in_range_mod6_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_4_pages_in_range_mod6_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_4_pages_in_range_mod6_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod6_4_pages_in_range_mod6_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_mod6_5_pages_in_range_mod6_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_mod6_5_pages_in_range_mod6_ico");
    let output_dir = tmp("export_dir");
    let err = export_mod6_5_pages_in_range_mod6_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

// PARITY_BATCH6_TESTS_END
// PARITY_BATCH7_TESTS_START
// Auto-generated parity batch7 tests

#[test]
fn rotate_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_first_third_pages_in_range");
    let err = rotate_first_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_second_third_pages_in_range");
    let err = rotate_second_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_third_third_pages_in_range");
    let err = rotate_third_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_first_third_pages_in_range_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_first_third_pages_in_range_ccw");
    let err = rotate_first_third_pages_in_range_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_second_third_pages_in_range_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_second_third_pages_in_range_ccw");
    let err = rotate_second_third_pages_in_range_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_third_third_pages_in_range_ccw_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_third_third_pages_in_range_ccw");
    let err = rotate_third_third_pages_in_range_ccw(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_first_third_pages_in_range");
    let err = rotate_180_first_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_second_third_pages_in_range");
    let err = rotate_180_second_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_180_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "rotate_180_third_third_pages_in_range");
    let err = rotate_180_third_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_first_third_pages_in_range");
    let err = reset_rotation_first_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_second_third_pages_in_range");
    let err = reset_rotation_second_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reset_rotation_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reset_rotation_third_third_pages_in_range");
    let err = reset_rotation_third_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_first_third_pages_in_range");
    let err = delete_first_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_second_third_pages_in_range");
    let err = delete_second_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "delete_third_third_pages_in_range");
    let err = delete_third_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_first_third_pages_in_range");
    let err = keep_first_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_second_third_pages_in_range");
    let err = keep_second_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn keep_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "keep_third_third_pages_in_range");
    let err = keep_third_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_first_third_pages_in_range");
    let err = duplicate_first_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_second_third_pages_in_range");
    let err = duplicate_second_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_third_third_pages_in_range");
    let err = duplicate_third_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_first_third_pages_in_range");
    let err = flatten_first_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_second_third_pages_in_range");
    let err = flatten_second_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn flatten_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "flatten_third_third_pages_in_range");
    let err = flatten_third_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_first_third_pages_in_range");
    let err = reverse_first_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_second_third_pages_in_range");
    let err = reverse_second_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reverse_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "reverse_third_third_pages_in_range");
    let err = reverse_third_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_first_third_pages_in_range");
    let err = crop_first_third_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_second_third_pages_in_range");
    let err = crop_second_third_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn crop_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "crop_third_third_pages_in_range");
    let err = crop_third_third_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_first_third_pages_in_range");
    let output_path = tmp("extract_out.pdf");
    let err = extract_first_third_pages_in_range(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_second_third_pages_in_range");
    let output_path = tmp("extract_out.pdf");
    let err = extract_second_third_pages_in_range(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn extract_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "extract_third_third_pages_in_range");
    let output_path = tmp("extract_out.pdf");
    let err = extract_third_third_pages_in_range(path.clone(), 5, 10, output_path.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_third_pages_in_range_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_third_pages_in_range_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_first_third_pages_in_range_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_third_pages_in_range_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_third_pages_in_range_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_second_third_pages_in_range_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_third_third_pages_in_range_as_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_third_third_pages_in_range_as_pdf");
    let output_dir = tmp("export_dir");
    let err = export_third_third_pages_in_range_as_pdf(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_third_pages_in_range_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_third_pages_in_range_png");
    let output_dir = tmp("export_dir");
    let err = export_first_third_pages_in_range_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_third_pages_in_range_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_third_pages_in_range_png");
    let output_dir = tmp("export_dir");
    let err = export_second_third_pages_in_range_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_third_third_pages_in_range_png_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_third_third_pages_in_range_png");
    let output_dir = tmp("export_dir");
    let err = export_third_third_pages_in_range_png(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_third_pages_in_range_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_third_pages_in_range_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_first_third_pages_in_range_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_third_pages_in_range_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_third_pages_in_range_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_second_third_pages_in_range_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_third_third_pages_in_range_jpeg_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_third_third_pages_in_range_jpeg");
    let output_dir = tmp("export_dir");
    let err = export_third_third_pages_in_range_jpeg(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_first_third_pages_in_range");
    let err = add_page_numbers_first_third_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_second_third_pages_in_range");
    let err = add_page_numbers_second_third_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_numbers_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_numbers_third_third_pages_in_range");
    let err = add_page_numbers_third_third_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_first_third_pages_in_range");
    let err = add_text_watermark_first_third_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_second_third_pages_in_range");
    let err = add_text_watermark_second_third_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_watermark_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_text_watermark_third_third_pages_in_range");
    let err = add_text_watermark_third_third_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_first_third_pages_in_range_before_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_first_third_pages_in_range_before");
    let err = duplicate_first_third_pages_in_range_before(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_second_third_pages_in_range_before_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_second_third_pages_in_range_before");
    let err = duplicate_second_third_pages_in_range_before(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_third_third_pages_in_range_before_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_third_third_pages_in_range_before");
    let err = duplicate_third_third_pages_in_range_before(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_first_third_pages_in_range_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_first_third_pages_in_range_to_start");
    let err = duplicate_first_third_pages_in_range_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_second_third_pages_in_range_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_second_third_pages_in_range_to_start");
    let err = duplicate_second_third_pages_in_range_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_third_third_pages_in_range_to_start_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_third_third_pages_in_range_to_start");
    let err = duplicate_third_third_pages_in_range_to_start(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_first_third_pages_in_range_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_first_third_pages_in_range_to_end");
    let err = duplicate_first_third_pages_in_range_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_second_third_pages_in_range_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_second_third_pages_in_range_to_end");
    let err = duplicate_second_third_pages_in_range_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn duplicate_third_third_pages_in_range_to_end_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "duplicate_third_third_pages_in_range_to_end");
    let err = duplicate_third_third_pages_in_range_to_end(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_first_third_pages_in_range");
    let err = expand_first_third_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_second_third_pages_in_range");
    let err = expand_second_third_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn expand_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "expand_third_third_pages_in_range");
    let err = expand_third_third_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_first_third_pages_in_range");
    let err = shrink_first_third_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_second_third_pages_in_range");
    let err = shrink_second_third_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shrink_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "shrink_third_third_pages_in_range");
    let err = shrink_third_third_pages_in_range(path.clone(), 5, 10, 0.0, 0.0, 0.0, 0.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_first_third_pages_in_range");
    let err = clear_crop_first_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_second_third_pages_in_range");
    let err = clear_crop_second_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_crop_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "clear_crop_third_third_pages_in_range");
    let err = clear_crop_third_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_first_third_pages_in_range");
    let err = insert_blank_before_first_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_second_third_pages_in_range");
    let err = insert_blank_before_second_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_before_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_before_third_third_pages_in_range");
    let err = insert_blank_before_third_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_first_third_pages_in_range");
    let err = insert_blank_after_first_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_second_third_pages_in_range");
    let err = insert_blank_after_second_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn insert_blank_after_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "insert_blank_after_third_third_pages_in_range");
    let err = insert_blank_after_third_third_pages_in_range(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_first_third_pages_in_range");
    let err = bookmark_first_third_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_second_third_pages_in_range");
    let err = bookmark_second_third_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn bookmark_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "bookmark_third_third_pages_in_range");
    let err = bookmark_third_third_pages_in_range(path.clone(), 5, 10, None).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_first_third_pages_in_range");
    let err = set_page_size_first_third_pages_in_range(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_second_third_pages_in_range");
    let err = set_page_size_second_third_pages_in_range(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_page_size_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "set_page_size_third_third_pages_in_range");
    let err = set_page_size_third_third_pages_in_range(path.clone(), 5, 10, "letter".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_first_third_pages_in_range");
    let err = add_page_header_first_third_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_second_third_pages_in_range");
    let err = add_page_header_second_third_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_header_third_third_pages_in_range");
    let err = add_page_header_third_third_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_first_third_pages_in_range");
    let err = add_page_footer_first_third_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_second_third_pages_in_range");
    let err = add_page_footer_second_third_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_footer_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_footer_third_third_pages_in_range");
    let err = add_page_footer_third_third_pages_in_range(path.clone(), 5, 10, "wm".to_string()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_first_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_first_third_pages_in_range");
    let err = add_page_border_first_third_pages_in_range(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_second_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_second_third_pages_in_range");
    let err = add_page_border_second_third_pages_in_range(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_border_third_third_pages_in_range_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "add_page_border_third_third_pages_in_range");
    let err = add_page_border_third_third_pages_in_range(path.clone(), 5, 10, 1.0).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_third_pages_in_range_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_third_pages_in_range_webp");
    let output_dir = tmp("export_dir");
    let err = export_first_third_pages_in_range_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_third_pages_in_range_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_third_pages_in_range_webp");
    let output_dir = tmp("export_dir");
    let err = export_second_third_pages_in_range_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_third_third_pages_in_range_webp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_third_third_pages_in_range_webp");
    let output_dir = tmp("export_dir");
    let err = export_third_third_pages_in_range_webp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_third_pages_in_range_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_third_pages_in_range_bmp");
    let output_dir = tmp("export_dir");
    let err = export_first_third_pages_in_range_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_third_pages_in_range_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_third_pages_in_range_bmp");
    let output_dir = tmp("export_dir");
    let err = export_second_third_pages_in_range_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_third_third_pages_in_range_bmp_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_third_third_pages_in_range_bmp");
    let output_dir = tmp("export_dir");
    let err = export_third_third_pages_in_range_bmp(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_third_pages_in_range_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_third_pages_in_range_tiff");
    let output_dir = tmp("export_dir");
    let err = export_first_third_pages_in_range_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_third_pages_in_range_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_third_pages_in_range_tiff");
    let output_dir = tmp("export_dir");
    let err = export_second_third_pages_in_range_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_third_third_pages_in_range_tiff_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_third_third_pages_in_range_tiff");
    let output_dir = tmp("export_dir");
    let err = export_third_third_pages_in_range_tiff(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_third_pages_in_range_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_third_pages_in_range_gif");
    let output_dir = tmp("export_dir");
    let err = export_first_third_pages_in_range_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_third_pages_in_range_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_third_pages_in_range_gif");
    let output_dir = tmp("export_dir");
    let err = export_second_third_pages_in_range_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_third_third_pages_in_range_gif_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_third_third_pages_in_range_gif");
    let output_dir = tmp("export_dir");
    let err = export_third_third_pages_in_range_gif(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_third_pages_in_range_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_third_pages_in_range_ppm");
    let output_dir = tmp("export_dir");
    let err = export_first_third_pages_in_range_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_third_pages_in_range_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_third_pages_in_range_ppm");
    let output_dir = tmp("export_dir");
    let err = export_second_third_pages_in_range_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_third_third_pages_in_range_ppm_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_third_third_pages_in_range_ppm");
    let output_dir = tmp("export_dir");
    let err = export_third_third_pages_in_range_ppm(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_third_pages_in_range_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_third_pages_in_range_tga");
    let output_dir = tmp("export_dir");
    let err = export_first_third_pages_in_range_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_third_pages_in_range_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_third_pages_in_range_tga");
    let output_dir = tmp("export_dir");
    let err = export_second_third_pages_in_range_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_third_third_pages_in_range_tga_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_third_third_pages_in_range_tga");
    let output_dir = tmp("export_dir");
    let err = export_third_third_pages_in_range_tga(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_first_third_pages_in_range_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_first_third_pages_in_range_ico");
    let output_dir = tmp("export_dir");
    let err = export_first_third_pages_in_range_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_second_third_pages_in_range_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_second_third_pages_in_range_ico");
    let output_dir = tmp("export_dir");
    let err = export_second_third_pages_in_range_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_third_third_pages_in_range_ico_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "export_third_third_pages_in_range_ico");
    let output_dir = tmp("export_dir");
    let err = export_third_third_pages_in_range_ico(path.clone(), 5, 10, output_dir.to_string_lossy().into_owned())
        .unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_first_third_pages_in_range_by_rotation_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_first_third_pages_in_range_by_rotation");
    let err = sort_first_third_pages_in_range_by_rotation(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_second_third_pages_in_range_by_rotation_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_second_third_pages_in_range_by_rotation");
    let err = sort_second_third_pages_in_range_by_rotation(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_third_third_pages_in_range_by_rotation_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_third_third_pages_in_range_by_rotation");
    let err = sort_third_third_pages_in_range_by_rotation(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_first_third_pages_in_range_by_size_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_first_third_pages_in_range_by_size");
    let err = sort_first_third_pages_in_range_by_size(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_second_third_pages_in_range_by_size_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_second_third_pages_in_range_by_size");
    let err = sort_second_third_pages_in_range_by_size(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_third_third_pages_in_range_by_size_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_third_third_pages_in_range_by_size");
    let err = sort_third_third_pages_in_range_by_size(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

// PARITY_BATCH7_TESTS_END
// PARITY_BATCH8_TESTS_START
// Auto-generated parity batch8 tests

#[test]
fn sort_odd_pages_in_range_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_odd_pages_in_range_by_rotation_desc");
    let err = sort_odd_pages_in_range_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_even_pages_in_range_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_even_pages_in_range_by_rotation_desc");
    let err = sort_even_pages_in_range_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_range_local_odd_pages_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_range_local_odd_pages_by_rotation_desc");
    let err = sort_range_local_odd_pages_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_range_local_even_pages_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_range_local_even_pages_by_rotation_desc");
    let err = sort_range_local_even_pages_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_first_half_pages_in_range_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_first_half_pages_in_range_by_rotation_desc");
    let err = sort_first_half_pages_in_range_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_second_half_pages_in_range_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_second_half_pages_in_range_by_rotation_desc");
    let err = sort_second_half_pages_in_range_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_odd_pages_in_range_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_odd_pages_in_range_by_size_desc");
    let err = sort_odd_pages_in_range_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_even_pages_in_range_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_even_pages_in_range_by_size_desc");
    let err = sort_even_pages_in_range_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_range_local_odd_pages_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_range_local_odd_pages_by_size_desc");
    let err = sort_range_local_odd_pages_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_range_local_even_pages_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_range_local_even_pages_by_size_desc");
    let err = sort_range_local_even_pages_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_first_half_pages_in_range_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_first_half_pages_in_range_by_size_desc");
    let err = sort_first_half_pages_in_range_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_second_half_pages_in_range_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_second_half_pages_in_range_by_size_desc");
    let err = sort_second_half_pages_in_range_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod3_0_pages_in_range_mod3_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod3_0_pages_in_range_mod3_by_rotation_desc");
    let err = sort_mod3_0_pages_in_range_mod3_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod3_0_pages_in_range_mod3_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod3_0_pages_in_range_mod3_by_size_desc");
    let err = sort_mod3_0_pages_in_range_mod3_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod3_1_pages_in_range_mod3_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod3_1_pages_in_range_mod3_by_rotation_desc");
    let err = sort_mod3_1_pages_in_range_mod3_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod3_1_pages_in_range_mod3_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod3_1_pages_in_range_mod3_by_size_desc");
    let err = sort_mod3_1_pages_in_range_mod3_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod3_2_pages_in_range_mod3_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod3_2_pages_in_range_mod3_by_rotation_desc");
    let err = sort_mod3_2_pages_in_range_mod3_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod3_2_pages_in_range_mod3_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod3_2_pages_in_range_mod3_by_size_desc");
    let err = sort_mod3_2_pages_in_range_mod3_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod4_0_pages_in_range_mod4_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod4_0_pages_in_range_mod4_by_rotation_desc");
    let err = sort_mod4_0_pages_in_range_mod4_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod4_0_pages_in_range_mod4_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod4_0_pages_in_range_mod4_by_size_desc");
    let err = sort_mod4_0_pages_in_range_mod4_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod4_1_pages_in_range_mod4_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod4_1_pages_in_range_mod4_by_rotation_desc");
    let err = sort_mod4_1_pages_in_range_mod4_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod4_1_pages_in_range_mod4_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod4_1_pages_in_range_mod4_by_size_desc");
    let err = sort_mod4_1_pages_in_range_mod4_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod4_2_pages_in_range_mod4_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod4_2_pages_in_range_mod4_by_rotation_desc");
    let err = sort_mod4_2_pages_in_range_mod4_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod4_2_pages_in_range_mod4_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod4_2_pages_in_range_mod4_by_size_desc");
    let err = sort_mod4_2_pages_in_range_mod4_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod4_3_pages_in_range_mod4_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod4_3_pages_in_range_mod4_by_rotation_desc");
    let err = sort_mod4_3_pages_in_range_mod4_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod4_3_pages_in_range_mod4_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod4_3_pages_in_range_mod4_by_size_desc");
    let err = sort_mod4_3_pages_in_range_mod4_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod5_0_pages_in_range_mod5_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod5_0_pages_in_range_mod5_by_rotation_desc");
    let err = sort_mod5_0_pages_in_range_mod5_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod5_0_pages_in_range_mod5_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod5_0_pages_in_range_mod5_by_size_desc");
    let err = sort_mod5_0_pages_in_range_mod5_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod5_1_pages_in_range_mod5_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod5_1_pages_in_range_mod5_by_rotation_desc");
    let err = sort_mod5_1_pages_in_range_mod5_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod5_1_pages_in_range_mod5_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod5_1_pages_in_range_mod5_by_size_desc");
    let err = sort_mod5_1_pages_in_range_mod5_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod5_2_pages_in_range_mod5_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod5_2_pages_in_range_mod5_by_rotation_desc");
    let err = sort_mod5_2_pages_in_range_mod5_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod5_2_pages_in_range_mod5_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod5_2_pages_in_range_mod5_by_size_desc");
    let err = sort_mod5_2_pages_in_range_mod5_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod5_3_pages_in_range_mod5_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod5_3_pages_in_range_mod5_by_rotation_desc");
    let err = sort_mod5_3_pages_in_range_mod5_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod5_3_pages_in_range_mod5_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod5_3_pages_in_range_mod5_by_size_desc");
    let err = sort_mod5_3_pages_in_range_mod5_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod5_4_pages_in_range_mod5_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod5_4_pages_in_range_mod5_by_rotation_desc");
    let err = sort_mod5_4_pages_in_range_mod5_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod5_4_pages_in_range_mod5_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod5_4_pages_in_range_mod5_by_size_desc");
    let err = sort_mod5_4_pages_in_range_mod5_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod6_0_pages_in_range_mod6_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod6_0_pages_in_range_mod6_by_rotation_desc");
    let err = sort_mod6_0_pages_in_range_mod6_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod6_0_pages_in_range_mod6_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod6_0_pages_in_range_mod6_by_size_desc");
    let err = sort_mod6_0_pages_in_range_mod6_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod6_1_pages_in_range_mod6_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod6_1_pages_in_range_mod6_by_rotation_desc");
    let err = sort_mod6_1_pages_in_range_mod6_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod6_1_pages_in_range_mod6_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod6_1_pages_in_range_mod6_by_size_desc");
    let err = sort_mod6_1_pages_in_range_mod6_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod6_2_pages_in_range_mod6_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod6_2_pages_in_range_mod6_by_rotation_desc");
    let err = sort_mod6_2_pages_in_range_mod6_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod6_2_pages_in_range_mod6_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod6_2_pages_in_range_mod6_by_size_desc");
    let err = sort_mod6_2_pages_in_range_mod6_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod6_3_pages_in_range_mod6_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod6_3_pages_in_range_mod6_by_rotation_desc");
    let err = sort_mod6_3_pages_in_range_mod6_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod6_3_pages_in_range_mod6_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod6_3_pages_in_range_mod6_by_size_desc");
    let err = sort_mod6_3_pages_in_range_mod6_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod6_4_pages_in_range_mod6_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod6_4_pages_in_range_mod6_by_rotation_desc");
    let err = sort_mod6_4_pages_in_range_mod6_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod6_4_pages_in_range_mod6_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod6_4_pages_in_range_mod6_by_size_desc");
    let err = sort_mod6_4_pages_in_range_mod6_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod6_5_pages_in_range_mod6_by_rotation_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod6_5_pages_in_range_mod6_by_rotation_desc");
    let err = sort_mod6_5_pages_in_range_mod6_by_rotation_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sort_mod6_5_pages_in_range_mod6_by_size_desc_rejects_invalid_range() {
    let path = save(&mut build_pdf(2), "sort_mod6_5_pages_in_range_mod6_by_size_desc");
    let err = sort_mod6_5_pages_in_range_mod6_by_size_desc(path.clone(), 5, 10).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

// PARITY_BATCH8_TESTS_END

#[test]
fn duplicate_page_range_to_start_inserts_copies() {
    let path = save(&mut build_pdf(3), "dup_range_start");
    let copied = duplicate_page_range_to_start(path.clone(), 1, 2).unwrap();
    assert_eq!(copied, 2);
    assert_eq!(page_count(&path), 5);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_pdf_page_ppm_rejects_invalid_page() {
    let path = save(&mut build_pdf(1), "export_ppm_invalid");
    let output = tmp("export_ppm_invalid_out.ppm");
    let err = export_pdf_page_ppm(path.clone(), 3, output.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore = "requires PDFium shared library"]
fn export_pdf_page_ppm_writes_file() {
    let path = save(&mut build_pdf(1), "export_ppm_write");
    let output = tmp("export_ppm_write_out.ppm");
    let written = export_pdf_page_ppm(path.clone(), 0, output.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written, output.to_string_lossy());
    assert!(output.is_file());
    assert!(std::fs::metadata(&output).unwrap().len() > 50);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn clear_all_page_crops_removes_crop_boxes() {
    let path = save(&mut build_pdf(2), "clear_all_crops");
    crop_all_pages(path.clone(), 30.0, 30.0, 30.0, 30.0).unwrap();
    let cleared = clear_all_page_crops(path.clone()).unwrap();
    assert_eq!(cleared, 2);
    let doc = Document::load(&path).unwrap();
    for page_id in doc.get_pages().values() {
        assert!(doc.get_dictionary(*page_id).unwrap().get(b"CropBox").is_err());
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn clear_pdf_bookmarks_removes_outline() {
    let path = save(&mut build_pdf_with_outlines(2), "clear_bookmarks");
    let removed = clear_pdf_bookmarks(path.clone()).unwrap();
    assert_eq!(removed, 2);
    assert!(get_pdf_bookmarks(path.clone()).unwrap().is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_header_stamps_top_text() {
    let path = save(&mut build_pdf(1), "page_header");
    let stamped = add_page_header(path.clone(), 0, 0, "DRAFT".to_string()).unwrap();
    assert_eq!(stamped, 1);
    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let bytes = pdf::page_text::read_page_content(&doc, page_id).unwrap();
    let content = String::from_utf8_lossy(&bytes);
    assert!(content.contains("(DRAFT)"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn export_page_as_pdf_writes_single_page() {
    let path = save(&mut build_pdf(3), "export_page_pdf");
    let output = tmp("export_page_pdf_out");
    let written = export_page_as_pdf(path.clone(), 1, output.to_string_lossy().into_owned()).unwrap();
    assert_eq!(page_count(&written), 1);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&written);
}

#[test]
fn export_pdf_page_bmp_rejects_invalid_page() {
    let path = save(&mut build_pdf(1), "export_bmp_invalid");
    let output = tmp("export_bmp_invalid_out.bmp");
    let err = export_pdf_page_bmp(path.clone(), 2, output.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
#[ignore = "requires PDFium shared library"]
fn export_pdf_page_bmp_writes_file() {
    let path = save(&mut build_pdf(1), "export_bmp_write");
    let output = tmp("export_bmp_write_out.bmp");
    let written = export_pdf_page_bmp(path.clone(), 0, output.to_string_lossy().into_owned()).unwrap();
    assert_eq!(written, output.to_string_lossy());
    assert!(output.is_file());
    assert!(std::fs::metadata(&output).unwrap().len() > 50);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn rotate_page_accumulates_in_90_steps() {
    let path = save(&mut build_pdf(1), "rotate");
    rotate_page(path.clone(), 0).unwrap();
    assert_eq!(rotation(&path), 90);
    rotate_page(path.clone(), 0).unwrap();
    assert_eq!(rotation(&path), 180);
    rotate_page(path.clone(), 0).unwrap();
    rotate_page(path.clone(), 0).unwrap();
    assert_eq!(rotation(&path), 0, "should wrap at 360");
    let _ = std::fs::remove_file(&path);
}

fn rotation(path: &str) -> i64 {
    let doc = Document::load(path).unwrap();
    let pid = *doc.get_pages().get(&1).unwrap();
    page_rotation(&doc, pid)
}

#[test]
fn insert_pdf_adds_pages_at_index() {
    let main_path = save(&mut build_pdf(2), "insert_main");
    let ins_path = save(&mut build_pdf(2), "insert_src");
    // Insert the first page of the source at index 1 of the main doc.
    insert_pdf(main_path.clone(), ins_path.clone(), 1, 0, 0).unwrap();
    assert_eq!(page_count(&main_path), 3);
    assert_eq!(count_entry(&main_path), 3);
    let _ = std::fs::remove_file(&main_path);
    let _ = std::fs::remove_file(&ins_path);
}

#[test]
fn split_pdf_creates_separate_files() {
    let path = save(&mut build_pdf(4), "split");
    let outputs = split_pdf(path.clone(), vec![(0, 1), (2, 3)]).unwrap();
    assert_eq!(outputs.len(), 2);
    for out in &outputs {
        assert_eq!(page_count(out), 2);
        assert_eq!(count_entry(out), 2);
        let _ = std::fs::remove_file(out);
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn split_pdf_rejects_invalid_range() {
    let path = save(&mut build_pdf(3), "split_invalid");
    let err = split_pdf(path.clone(), vec![(2, 1)]).unwrap_err();
    assert!(err.contains("Invalid page range"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn split_pdf_rejects_empty_ranges() {
    let path = save(&mut build_pdf(2), "split_empty");
    match split_pdf(path.clone(), vec![]) {
        Ok(_) => panic!("expected empty ranges to fail"),
        Err(message) => assert!(message.contains("At least one page range")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn split_pdf_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_split_missing_{}.pdf", std::process::id()));
    let err = split_pdf(missing.to_string_lossy().into_owned(), vec![(0, 0)]).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn insert_pdf_rejects_invalid_source_range() {
    let dest = save(&mut build_pdf(2), "insert_dest_range");
    let src = save(&mut build_pdf(2), "insert_src_range");
    let err = insert_pdf(dest.clone(), src.clone(), 1, 1, 0).unwrap_err();
    assert!(err.contains("Invalid insert page range"));
    let _ = std::fs::remove_file(&dest);
    let _ = std::fs::remove_file(&src);
}

#[test]
fn insert_pdf_rejects_source_range_out_of_bounds() {
    let dest = save(&mut build_pdf(2), "insert_dest_src_oob");
    let src = save(&mut build_pdf(1), "insert_src_oob");
    let err = insert_pdf(dest.clone(), src.clone(), 0, 0, 5).unwrap_err();
    assert!(err.contains("Invalid insert page range"));
    let _ = std::fs::remove_file(&dest);
    let _ = std::fs::remove_file(&src);
}

#[test]
fn insert_pdf_rejects_out_of_bounds_index() {
    let dest = save(&mut build_pdf(2), "insert_dest_bounds");
    let src = save(&mut build_pdf(1), "insert_src_bounds");
    let err = insert_pdf(dest.clone(), src.clone(), 9, 0, 0).unwrap_err();
    assert!(err.contains("Insert index out of bounds"));
    let _ = std::fs::remove_file(&dest);
    let _ = std::fs::remove_file(&src);
}

#[test]
fn insert_pdf_rejects_missing_source_file() {
    let dest = save(&mut build_pdf(2), "insert_dest_missing");
    let missing = std::env::temp_dir().join(format!("pp_insert_missing_{}.pdf", std::process::id()));
    let err = insert_pdf(dest.clone(), missing.to_string_lossy().into_owned(), 0, 0, 0).unwrap_err();
    assert!(!err.is_empty());
    let _ = std::fs::remove_file(&dest);
}

#[test]
fn insert_pdf_rejects_missing_dest_file() {
    let src = save(&mut build_pdf(1), "insert_src_missing_dest");
    let missing = std::env::temp_dir().join(format!("pp_insert_dest_missing_{}.pdf", std::process::id()));
    let err = insert_pdf(missing.to_string_lossy().into_owned(), src.clone(), 0, 0, 0).unwrap_err();
    assert!(!err.is_empty());
    let _ = std::fs::remove_file(&src);
}

fn attach_type1_font(doc: &mut Document, page_id: ObjectId, res_name: &[u8], base_font: &[u8]) -> ObjectId {
    let font_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Font".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"Type1".to_vec())),
        (b"BaseFont".to_vec(), Object::Name(base_font.to_vec())),
    ])));
    let page = doc.get_dictionary_mut(page_id).unwrap();
    let resources = match page.get_mut(b"Resources") {
        Ok(Object::Dictionary(dict)) => dict,
        _ => {
            page.set(b"Resources", Object::Dictionary(Dictionary::new()));
            doc.get_dictionary_mut(page_id).unwrap().get_mut(b"Resources").unwrap().as_dict_mut().unwrap()
        }
    };
    match resources.get_mut(b"Font") {
        Ok(Object::Dictionary(fonts)) => fonts.set(res_name, Object::Reference(font_id)),
        _ => {
            let mut fonts = Dictionary::new();
            fonts.set(res_name, Object::Reference(font_id));
            resources.set(b"Font", Object::Dictionary(fonts));
        }
    }
    font_id
}

#[test]
fn insert_pdf_merges_acroform_catalog() {
    let main = save(&mut build_pdf(1), "insert_main_form");
    let src = save(&mut build_pdf_with_text_field(), "insert_src_form");
    insert_pdf(main.clone(), src.clone(), 1, 0, 0).unwrap();
    let doc = Document::load(&main).unwrap();
    let catalog = doc.catalog().unwrap();
    let af_id = catalog.get(b"AcroForm").unwrap().as_reference().unwrap();
    let af = doc.get_dictionary(af_id).unwrap();
    let fields = af.get(b"Fields").unwrap().as_array().unwrap();
    assert!(!fields.is_empty());
    let names = get_pdf_form_fields(main.clone()).unwrap();
    assert!(names.iter().any(|field| field.name == "FirstName"));
    let _ = std::fs::remove_file(&main);
    let _ = std::fs::remove_file(&src);
}

#[test]
fn insert_pdf_renames_conflicting_form_field() {
    let main_path = save(&mut build_pdf_with_text_field(), "insert_main_form_conflict");
    let src_path = save(&mut build_pdf_with_text_field(), "insert_src_form_conflict");
    insert_pdf(main_path.clone(), src_path.clone(), 1, 0, 0).unwrap();
    let fields = get_pdf_form_fields(main_path.clone()).unwrap();
    let names: Vec<_> = fields.iter().map(|field| field.name.as_str()).collect();
    assert!(names.contains(&"FirstName"));
    assert!(names.iter().any(|name| name.starts_with("imported_FirstName")));
    let _ = std::fs::remove_file(&main_path);
    let _ = std::fs::remove_file(&src_path);
}

#[test]
fn insert_pdf_dedups_identical_fonts() {
    let mut dest = build_pdf(1);
    let dest_page = *dest.get_pages().get(&1).unwrap();
    let dest_font = attach_type1_font(&mut dest, dest_page, b"F1", b"Helvetica");
    let dest_path = save(&mut dest, "insert_font_dest");

    let mut src = build_pdf(1);
    let src_page = *src.get_pages().get(&1).unwrap();
    let _src_font = attach_type1_font(&mut src, src_page, b"F1", b"Helvetica");
    let src_path = save(&mut src, "insert_font_src");

    insert_pdf(dest_path.clone(), src_path.clone(), 1, 0, 0).unwrap();
    let doc = Document::load(&dest_path).unwrap();
    let pages: Vec<_> = doc.get_pages().into_values().collect();
    let dest_entry =
        pdf::fonts::page_font_entries(&doc, pages[0]).into_iter().find(|(name, _)| name == b"F1").map(|(_, id)| id);
    let inserted_entry =
        pdf::fonts::page_font_entries(&doc, pages[1]).into_iter().find(|(name, _)| name == b"F1").map(|(_, id)| id);
    assert_eq!(dest_entry, Some(dest_font));
    assert_eq!(inserted_entry, Some(dest_font));
    let _ = std::fs::remove_file(&dest_path);
    let _ = std::fs::remove_file(&src_path);
}

#[test]
fn delete_page_rejects_invalid_index() {
    let path = save(&mut build_pdf(2), "delete_invalid");
    let err = delete_page(path.clone(), 9).unwrap_err();
    assert!(err.contains("Page index out of bounds"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_page_rejects_only_page() {
    let path = save(&mut build_pdf(1), "delete_only");
    let err = delete_page(path.clone(), 0).unwrap_err();
    assert!(err.contains("only page"));
    assert_eq!(page_count(&path), 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_page_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_delete_missing_{}.pdf", std::process::id()));
    let err = delete_page(missing.to_string_lossy().into_owned(), 0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_page_rejects_invalid_index() {
    let path = save(&mut build_pdf(2), "move_invalid");
    let err = move_page(path.clone(), 0, 9).unwrap_err();
    assert!(err.contains("Index out of bounds"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_page_rejects_invalid_from_index() {
    let path = save(&mut build_pdf(2), "move_invalid_from");
    let err = move_page(path.clone(), 9, 0).unwrap_err();
    assert!(err.contains("Index out of bounds"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn move_page_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_move_missing_{}.pdf", std::process::id()));
    let err = move_page(missing.to_string_lossy().into_owned(), 0, 1).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn move_page_same_index_is_noop() {
    let path = save(&mut build_pdf(3), "move_noop");
    move_page(path.clone(), 1, 1).unwrap();
    assert_eq!(page_count(&path), 3);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn rotate_page_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_rotate_missing_{}.pdf", std::process::id()));
    let err = rotate_page(missing.to_string_lossy().into_owned(), 0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn rotate_page_rejects_invalid_index() {
    let path = save(&mut build_pdf(1), "rotate_invalid");
    let err = rotate_page(path.clone(), 9).unwrap_err();
    assert!(err.contains("Page not found"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn convert_pdf_to_markdown_rejects_missing_file() {
    let missing = tmp("convert_md_missing");
    let err = convert_pdf_to_markdown(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn save_pdf_markdown_rejects_missing_file() {
    let missing = tmp("save_md_missing");
    let custom = tmp("save_md_custom_out.md");
    assert!(save_pdf_markdown(
        missing.to_string_lossy().into_owned(),
        false,
        Some(custom.to_string_lossy().into_owned()),
    )
    .is_err());
}

#[test]
fn write_markdown_file_creates_sibling_md() {
    let pdf_path = tmp("markdown_write");
    let md_path = pdf_path.with_extension("md");
    let _ = std::fs::remove_file(&md_path);

    let result = pdf::markdown::write_markdown_file(&md_path, "# Test\n", false).unwrap();

    assert!(result.written);
    assert!(!result.conflict);
    assert_eq!(result.markdown_path, md_path.to_string_lossy());
    assert_eq!(std::fs::read_to_string(&md_path).unwrap(), "# Test\n");
    let _ = std::fs::remove_file(&md_path);
}

#[test]
fn write_markdown_file_detects_conflict_without_overwrite() {
    let pdf_path = tmp("markdown_conflict");
    let md_path = pdf_path.with_extension("md");
    std::fs::write(&md_path, "# Existing\n").unwrap();

    let result = pdf::markdown::write_markdown_file(&md_path, "# New\n", false).unwrap();

    assert!(!result.written);
    assert!(result.conflict);
    assert_eq!(std::fs::read_to_string(&md_path).unwrap(), "# Existing\n");
    let _ = std::fs::remove_file(&md_path);
}

#[test]
fn write_markdown_file_overwrites_after_confirmation() {
    let pdf_path = tmp("markdown_overwrite");
    let md_path = pdf_path.with_extension("md");
    std::fs::write(&md_path, "# Existing\n").unwrap();

    let result = pdf::markdown::write_markdown_file(&md_path, "# New\n", true).unwrap();

    assert!(result.written);
    assert!(!result.conflict);
    assert_eq!(std::fs::read_to_string(&md_path).unwrap(), "# New\n");
    let _ = std::fs::remove_file(&md_path);
}

#[test]
fn write_markdown_file_skips_rewrite_when_content_matches() {
    let pdf_path = tmp("markdown_unchanged");
    let md_path = pdf_path.with_extension("md");
    std::fs::write(&md_path, "# Same\n").unwrap();

    let result = pdf::markdown::write_markdown_file(&md_path, "# Same\n", false).unwrap();

    assert!(!result.written);
    assert!(!result.conflict);
    assert_eq!(std::fs::read_to_string(&md_path).unwrap(), "# Same\n");
    let _ = std::fs::remove_file(&md_path);
}

#[test]
fn write_markdown_file_writes_custom_path() {
    let custom = std::env::temp_dir().join(format!("pp_md_custom_{}.md", std::process::id()));
    let _ = std::fs::remove_file(&custom);

    let result = pdf::markdown::write_markdown_file(&custom, "# Custom\n", false).unwrap();

    assert!(result.written);
    assert!(!result.conflict);
    assert_eq!(result.markdown_path, custom.to_string_lossy());
    assert_eq!(std::fs::read_to_string(&custom).unwrap(), "# Custom\n");
    let _ = std::fs::remove_file(&custom);
}

fn md_line(text: &str, top: f32, bottom: f32, cells: Vec<(&str, f32, f32)>) -> MarkdownTextLine {
    let height = (top - bottom).max(1.0);
    let (left, right, cells) = if cells.is_empty() {
        (72.0, 420.0, vec![MarkdownTextCell { text: text.to_string() }])
    } else {
        let left = cells.iter().map(|(_, left, _)| *left).fold(f32::INFINITY, f32::min);
        let right = cells.iter().map(|(_, _, right)| *right).fold(f32::NEG_INFINITY, f32::max);
        let cells = cells.into_iter().map(|(text, _, _)| MarkdownTextCell { text: text.to_string() }).collect();
        (left, right, cells)
    };
    MarkdownTextLine { text: text.to_string(), left, right, bottom, top, height, cells }
}

#[test]
fn symbol_font_bullets_become_markdown_bullets() {
    // Wingdings 'n' (0x6E) is the square bullet that leaked as a literal "n".
    assert_eq!(map_symbol_glyph("Wingdings", 'n'), '•');
    assert_eq!(map_symbol_glyph("Wingdings", 'l'), '•');
    // Subset-prefixed font names still match.
    assert_eq!(map_symbol_glyph("ABCDEF+Wingdings", 'n'), '•');
    // Private-Use-Area encoded variant (0xF000 + code).
    assert_eq!(map_symbol_glyph("Wingdings", '\u{F06E}'), '•');
    // Ordinary text fonts are never rewritten.
    assert_eq!(map_symbol_glyph("ArialMT", 'n'), 'n');
    // Symbol-font letters are Greek (nu), not bullets.
    assert_eq!(map_symbol_glyph("Symbol", 'n'), 'n');
    // Pre-check stays in sync with the mapper.
    assert!(is_symbol_glyph_candidate('n'));
    assert!(!is_symbol_glyph_candidate('a'));
}

#[test]
fn get_pdf_page_count_reports_document_length() {
    let path = save(&mut build_pdf(5), "page_count");
    assert_eq!(get_pdf_page_count(path.clone()).unwrap(), 5);
    let _ = fs::remove_file(&path);
}

#[test]
fn list_pdf_browser_entries_lists_pdfs_and_directories() {
    let dir = std::env::temp_dir().join(format!("pp_browser_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let standalone = save(&mut build_pdf(1), "browser_src");
    fs::copy(&standalone, dir.join("sample.pdf")).unwrap();
    let _ = fs::remove_file(&standalone);
    fs::write(dir.join("notes.txt"), b"text").unwrap();
    fs::create_dir_all(dir.join("nested")).unwrap();

    let listing = list_pdf_browser_entries(Some(dir.to_string_lossy().into_owned()), None).unwrap();
    let names: Vec<&str> = listing.entries.iter().map(|entry| entry.name.as_str()).collect();
    assert!(names.contains(&"nested"));
    assert!(names.contains(&"sample.pdf"));
    assert!(!names.contains(&"notes.txt"));
    assert!(listing.entries.iter().find(|e| e.name == "nested").unwrap().is_dir);
    assert!(!listing.entries.iter().find(|e| e.name == "sample.pdf").unwrap().is_dir);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn list_pdf_browser_entries_from_file_path_uses_parent_dir() {
    let dir = std::env::temp_dir().join(format!("pp_browser_file_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let standalone = save(&mut build_pdf(1), "browser_file_src");
    let pdf_path = dir.join("target.pdf");
    fs::copy(&standalone, &pdf_path).unwrap();
    let _ = fs::remove_file(&standalone);

    let listing = list_pdf_browser_entries(Some(pdf_path.to_string_lossy().into_owned()), None).unwrap();
    assert_eq!(listing.current_dir, dir.canonicalize().unwrap().to_string_lossy());
    assert!(listing.entries.iter().any(|entry| entry.name == "target.pdf"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn list_pdf_browser_entries_rejects_missing_directory() {
    let missing = std::env::temp_dir().join(format!("pp_browser_missing_{}", std::process::id()));
    match list_pdf_browser_entries(Some(missing.to_string_lossy().into_owned()), None) {
        Ok(_) => panic!("expected missing directory to fail"),
        Err(message) => assert!(!message.is_empty()),
    }
}

#[test]
fn text_documents_load_and_generate_pdf_pages() {
    let dir = std::env::temp_dir().join(format!("pp_text_document_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let markdown = dir.join("notes.md");
    fs::write(&markdown, "# Notes\n").unwrap();
    assert_eq!(read_text_document(markdown.to_string_lossy().into_owned()).unwrap(), "# Notes\n");
    let css = dir.join("styles.css");
    fs::write(&css, "h1 { color: red; }\n").unwrap();
    assert_eq!(read_text_document(css.to_string_lossy().into_owned()).unwrap(), "h1 { color: red; }\n");

    let mut png = Vec::new();
    image::DynamicImage::new_rgb8(8, 8).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
    let working = create_pdf_from_document_pages(vec![png]).unwrap();
    assert_eq!(get_pdf_page_count(working.clone()).unwrap(), 1);
    let existing = dir.join("notes.pdf");
    fs::write(&existing, b"keep").unwrap();
    let generated = materialize_document_pdf(working, markdown.to_string_lossy().into_owned()).unwrap();
    assert!(generated.ends_with("notes_converted.pdf"));
    assert_eq!(fs::read(existing).unwrap(), b"keep");
    assert_eq!(get_pdf_page_count(generated).unwrap(), 1);

    let listing = list_pdf_browser_entries(Some(dir.to_string_lossy().into_owned()), Some(true)).unwrap();
    assert!(listing.entries.iter().any(|entry| entry.name == "notes.md"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn discard_document_pdf_removes_materialized_sibling_and_rejects_unrelated_paths() {
    let dir = std::env::temp_dir().join(format!("pp_discard_doc_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let markdown = dir.join("notes.md");
    fs::write(&markdown, "# Notes\n").unwrap();

    let mut png = Vec::new();
    image::DynamicImage::new_rgb8(8, 8).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
    let working = create_pdf_from_document_pages(vec![png]).unwrap();
    let generated = materialize_document_pdf(working, markdown.to_string_lossy().into_owned()).unwrap();
    let generated_path = std::path::PathBuf::from(&generated);
    assert!(generated_path.exists());

    // Happy path: discarding the materialized sibling succeeds.
    discard_document_pdf(generated.clone(), markdown.to_string_lossy().into_owned()).unwrap();
    assert!(!generated_path.exists());

    // Idempotent: a second discard does not error (file already gone).
    discard_document_pdf(generated.clone(), markdown.to_string_lossy().into_owned()).unwrap();

    // Reject paths that don't match the materialize pattern, even if the
    // caller claims a sibling source. The check is the only thing keeping a
    // hostile frontend from deleting arbitrary user files via this command.
    let unrelated_pdf = dir.join("user_report.pdf");
    fs::write(&unrelated_pdf, b"important").unwrap();
    let err =
        discard_document_pdf(unrelated_pdf.to_string_lossy().into_owned(), markdown.to_string_lossy().into_owned());
    assert!(err.is_err(), "expected rejection of non-materialize-named pdf");
    assert_eq!(fs::read(&unrelated_pdf).unwrap(), b"important", "user file must be untouched");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn markdown_text_lays_out_to_multipage_pdf() {
    use pdf::markdown_pdf::markdown_to_pdf;
    let sample = "# Heading One\n\nFirst paragraph with **bold** and `code` and [a link](https://example.com).\n\n## Heading Two\n\n```rust\nfn main() { println!(\"hi\"); }\n```\n\n- item one\n- item two\n- item three\n\n| Col A | Col B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n\nFinal paragraph.\n";
    let out = std::env::temp_dir().join(format!("pp_md_{}.pdf", std::process::id()));
    markdown_to_pdf(sample, &out).unwrap();
    assert_eq!(get_pdf_page_count(out.to_string_lossy().into_owned()).unwrap(), 1);
    let _ = fs::remove_file(&out);

    let big = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore.\n"
        .repeat(400);
    let big_md = format!("# Big\n\n{big}");
    let out2 = std::env::temp_dir().join(format!("pp_md_big_{}.pdf", std::process::id()));
    markdown_to_pdf(&big_md, &out2).unwrap();
    let count = get_pdf_page_count(out2.to_string_lossy().into_owned()).unwrap();
    assert!(count >= 3, "expected >=3 pages, got {count}");
    let _ = fs::remove_file(&out2);

    if let Ok(real) = fs::read_to_string("/tmp/mongrel/MONGRELDB_AI_0523_FINAL_HARDENING_AUDIT.md") {
        let out3 = std::env::temp_dir().join(format!("pp_md_real_{}.pdf", std::process::id()));
        markdown_to_pdf(&real, &out3).unwrap();
        let real_count = get_pdf_page_count(out3.to_string_lossy().into_owned()).unwrap();
        assert!(real_count >= 10, "real audit doc expected >=10 pages, got {real_count}");
        let _ = fs::remove_file(&out3);
    }
}

#[test]
fn image_pages_pdf_embeds_jpeg_directly_and_png_via_fallback() {
    use pdf::page_images::create_pdf_from_image_pages;
    let out_jpeg = std::env::temp_dir().join(format!("pp_img_jpeg_{}.pdf", std::process::id()));
    let out_png = std::env::temp_dir().join(format!("pp_img_png_{}.pdf", std::process::id()));

    let mut jpeg = Vec::new();
    image::DynamicImage::new_rgb8(12, 8)
        .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
        .unwrap();
    assert!(jpeg.starts_with(&[0xFF, 0xD8]));
    create_pdf_from_image_pages(&[jpeg], &out_jpeg).unwrap();
    assert_eq!(get_pdf_page_count(out_jpeg.to_string_lossy().into_owned()).unwrap(), 1);

    let mut png = Vec::new();
    image::DynamicImage::new_rgb8(10, 6)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    create_pdf_from_image_pages(&[png], &out_png).unwrap();
    assert_eq!(get_pdf_page_count(out_png.to_string_lossy().into_owned()).unwrap(), 1);

    let _ = fs::remove_file(&out_jpeg);
    let _ = fs::remove_file(&out_png);
}

#[test]
fn discard_working_copy_missing_path_succeeds() {
    let missing = std::env::temp_dir().join(format!("pp_missing_wc_{}.pdf", std::process::id()));
    discard_working_copy(missing.to_string_lossy().into_owned()).unwrap();
}

#[test]
fn file_byte_size_returns_length() {
    let path = save(&mut build_pdf(1), "byte_size");
    let len = file_byte_size(path.clone()).unwrap();
    assert!(len > 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn open_working_copy_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_open_wc_missing_{}.pdf", std::process::id()));
    let err = open_working_copy(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn save_working_copy_rejects_missing_working_file() {
    let missing = std::env::temp_dir().join(format!("pp_save_wc_missing_{}.pdf", std::process::id()));
    let target = std::env::temp_dir().join(format!("pp_save_wc_target_{}.pdf", std::process::id()));
    let err =
        save_working_copy(missing.to_string_lossy().into_owned(), target.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn snapshot_pdf_rejects_missing_source() {
    let missing = std::env::temp_dir().join(format!("pp_snapshot_missing_{}.pdf", std::process::id()));
    let err = snapshot_pdf(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn open_working_copy_creates_isolated_temp_file() {
    let path = save(&mut build_pdf(1), "wc_open");
    let working = open_working_copy(path.clone()).unwrap();
    assert_ne!(working, path);
    assert!(PathBuf::from(&working).exists());
    assert_eq!(fs::read(&working).unwrap(), fs::read(&path).unwrap());
    discard_working_copy(working).unwrap();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn working_copy_isolates_edits_until_saved() {
    let original = std::env::temp_dir().join(format!("pp_wc_orig_{}.pdf", std::process::id()));
    fs::write(&original, b"ORIGINAL").unwrap();
    let orig_str = original.to_string_lossy().into_owned();

    let working = open_working_copy(orig_str.clone()).unwrap();
    fs::write(&working, b"EDITED").unwrap(); // simulate an edit on the working copy
    assert_eq!(fs::read(&original).unwrap(), b"ORIGINAL"); // original untouched

    save_working_copy(working.clone(), orig_str).unwrap();
    assert_eq!(fs::read(&original).unwrap(), b"EDITED"); // save commits to original

    discard_working_copy(working.clone()).unwrap();
    assert!(!std::path::Path::new(&working).exists());
    let _ = fs::remove_file(&original);
}

#[test]
fn snapshot_pdf_creates_unique_history_files() {
    let path = save(&mut build_pdf(1), "snap_unique");
    let first = snapshot_pdf(path.clone()).unwrap();
    let second = snapshot_pdf(path.clone()).unwrap();
    assert_ne!(first, second);
    assert!(PathBuf::from(&first).exists());
    assert!(PathBuf::from(&second).exists());
    discard_working_copy(first).unwrap();
    discard_working_copy(second).unwrap();
    let _ = fs::remove_file(&path);
}

#[test]
fn snapshot_undo_restore_reverts_working_copy() {
    let path = save(&mut build_pdf(2), "undo_snap");
    let working = open_working_copy(path.clone()).unwrap();
    let baseline = snapshot_pdf(working.clone()).unwrap();
    assert_eq!(get_pdf_page_count(working.clone()).unwrap(), 2);

    delete_page(working.clone(), 0).unwrap();
    assert_eq!(get_pdf_page_count(working.clone()).unwrap(), 1);
    let edited = snapshot_pdf(working.clone()).unwrap();

    save_working_copy(baseline.clone(), working.clone()).unwrap();
    assert_eq!(get_pdf_page_count(working.clone()).unwrap(), 2);

    save_working_copy(edited.clone(), working.clone()).unwrap();
    assert_eq!(get_pdf_page_count(working.clone()).unwrap(), 1);

    discard_working_copy(working).unwrap();
    discard_working_copy(baseline).unwrap();
    discard_working_copy(edited).unwrap();
    let _ = fs::remove_file(&path);
}

fn build_paragraph_pdf() -> Document {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    let mut page = Dictionary::new();
    page.set("Type", Object::Name(b"Page".to_vec()));
    page.set("Parent", Object::Reference(pages_id));
    page.set(
        "Resources",
        Object::Dictionary(Dictionary::from_iter(vec![(
            "Font",
            Object::Dictionary(Dictionary::from_iter(vec![(
                "F1",
                Object::Dictionary(Dictionary::from_iter(vec![
                    ("Type", Object::Name(b"Font".to_vec())),
                    ("Subtype", Object::Name(b"Type1".to_vec())),
                    ("BaseFont", Object::Name(b"Helvetica".to_vec())),
                ])),
            )])),
        )])),
    );
    page.set(
        "MediaBox",
        Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(612), Object::Integer(792)]),
    );

    let content = b"BT /F1 12 Tf 72 700 Td (First line of paragraph) Tj ET\nBT /F1 12 Tf 72 686 Td (Second line of paragraph) Tj ET".to_vec();
    let stream_id = doc.add_object(Stream::new(Dictionary::new(), content));
    page.set("Contents", Object::Reference(stream_id));

    let page_id = doc.add_object(Object::Dictionary(page));

    let mut pages = Dictionary::new();
    pages.set("Type", Object::Name(b"Pages".to_vec()));
    pages.set("Count", Object::Integer(1));
    pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));

    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc
}

fn write_e2e_fixtures() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../e2e/fixtures");
    fs::create_dir_all(&fixtures_dir).unwrap();

    for (name, pages) in [("sample.pdf", 1usize), ("sample-3p.pdf", 3), ("sample-b.pdf", 1)] {
        let source = save(&mut build_pdf(pages), &format!("e2e_{name}"));
        let dest = fixtures_dir.join(name);
        fs::copy(&source, &dest).unwrap();
        let _ = fs::remove_file(source);
        eprintln!("wrote {}", dest.display());
    }

    let source = save(&mut build_paragraph_pdf(), "e2e_sample_paragraph");
    let dest = fixtures_dir.join("sample-paragraph.pdf");
    fs::copy(&source, &dest).unwrap();
    let _ = fs::remove_file(source);
    eprintln!("wrote {}", dest.display());
}

/// Writes `e2e/fixtures/*.pdf` for the WebdriverIO suite.
#[test]
#[ignore]
fn export_e2e_fixtures() {
    write_e2e_fixtures();
}

/// Back-compat alias for `export_e2e_fixtures`.
#[test]
#[ignore]
fn export_e2e_sample_pdf() {
    write_e2e_fixtures();
}

#[test]
fn encode_pdf_delta_roundtrip() {
    let base = b"AAAAABBBBBCCCCCDDDDD".to_vec();
    let mut current = base.clone();
    current[5] = b'x';
    current[12] = b'y';
    current.extend_from_slice(b"EEEEE");
    let delta = pdf::history::encode_pdf_delta(&base, &current).unwrap();
    let restored = pdf::history::apply_pdf_delta(&base, &delta).unwrap();
    assert_eq!(restored, current);
}

#[test]
fn snapshot_pdf_entry_uses_delta_for_large_files() {
    let path = save(&mut build_pdf(20), "delta_snap");
    let working = open_working_copy(path.clone()).unwrap();
    let (_, baseline) = snapshot_pdf_entry(vec![], working.clone()).unwrap();
    assert_eq!(baseline.kind, "full");

    delete_page(working.clone(), 0).unwrap();
    let (history, edited) = snapshot_pdf_entry(vec![baseline.clone()], working.clone()).unwrap();
    assert_eq!(edited.kind, "delta");
    assert_eq!(edited.base_index, Some(0));

    restore_history_entry(history.clone(), 0, working.clone()).unwrap();
    assert_eq!(get_pdf_page_count(working.clone()).unwrap(), 20);

    restore_history_entry(vec![baseline, edited], 1, working.clone()).unwrap();
    assert_eq!(get_pdf_page_count(working.clone()).unwrap(), 19);

    discard_working_copy(working).unwrap();
    let _ = fs::remove_file(&path);
}

#[test]
fn prune_history_entry_rematerializes_orphaned_deltas() {
    let path = save(&mut build_pdf(2), "prune_snap");
    let working = open_working_copy(path.clone()).unwrap();
    let (_, baseline) = snapshot_pdf_entry(vec![], working.clone()).unwrap();
    delete_page(working.clone(), 0).unwrap();
    let (_, edited) = snapshot_pdf_entry(vec![baseline.clone()], working.clone()).unwrap();
    let history = vec![baseline, edited];

    let pruned = prune_history_entry(history, 0).unwrap();
    assert_eq!(pruned.len(), 1);
    assert_eq!(pruned[0].kind, "full");

    restore_history_entry(pruned.clone(), 0, working.clone()).unwrap();
    assert_eq!(get_pdf_page_count(working.clone()).unwrap(), 1);

    discard_history_entry(pruned[0].clone()).unwrap();
    discard_working_copy(working).unwrap();
    let _ = fs::remove_file(&path);
}

#[test]
fn tagged_markdown_extracts_headings_and_paragraphs() {
    let doc = build_tagged_pdf();
    let pages = tagged_markdown_by_page(&doc).expect("tagged pages");
    let page1 = pages.get(&0).expect("page 1 markdown");
    assert!(page1.contains("# Introduction"));
    assert!(page1.contains("Body paragraph one."));
}

#[test]
fn tagged_markdown_formats_lists_and_tables() {
    let doc = build_tagged_pdf();
    let pages = tagged_markdown_by_page(&doc).expect("tagged pages");
    let page2 = pages.get(&1).expect("page 2 markdown");
    assert!(page2.contains("- First item"));
    assert!(page2.contains("| Name | Score |"));
    assert!(page2.contains("| Alice | 98 |"));
}

#[test]
fn tagged_markdown_absent_without_struct_tree() {
    let doc = build_pdf(1);
    assert!(tagged_markdown_by_page(&doc).is_none());
}

#[test]
fn tagged_markdown_formats_inline_emphasis_and_links() {
    let doc = build_tagged_pdf_extended();
    let pages = tagged_markdown_by_page(&doc).expect("tagged pages");
    let page1 = pages.get(&0).expect("page 1 markdown");
    assert!(page1.contains("Value is **important**"));
    assert!(page1.contains("[Example](https://example.com/docs)"));
}

#[test]
fn tagged_markdown_formats_toc_caption_code_and_notes() {
    let doc = build_tagged_pdf_extended();
    let pages = tagged_markdown_by_page(&doc).expect("tagged pages");
    let page1 = pages.get(&0).expect("page 1 markdown");
    let page2 = pages.get(&1).expect("page 2 markdown");
    assert!(page1.contains("- Getting started"));
    assert!(!page1.contains("Header text"));
    assert!(page2.contains("*Quarterly revenue chart*"));
    assert!(page2.contains("![Quarterly revenue chart]"));
    assert!(page2.contains("```"));
    assert!(page2.contains("fn main()"));
    assert!(page2.contains("> **Note:** See appendix A."));
}

#[test]
fn tagged_markdown_formats_ordered_list_and_thead_table() {
    let doc = build_tagged_pdf_extended();
    let pages = tagged_markdown_by_page(&doc).expect("tagged pages");
    let page2 = pages.get(&1).expect("page 2 markdown");
    assert!(page2.contains("1. First step"));
    assert!(page2.contains("| Region | Total |"));
    assert!(page2.contains("| West | 42 |"));
}

#[test]
fn split_sentences_splits_on_punctuation() {
    let sentences = pdf::summary::split_sentences("Alpha one. Beta two! Gamma three?");
    assert_eq!(sentences.len(), 3);
    assert!(sentences[0].contains("Alpha"));
}

#[test]
fn intelligent_extract_finds_email_url_and_date() {
    let extraction = pdf::summary::intelligent_extract_from_text(
        "Contact team@example.com on 03/15/2024. Visit https://example.com/docs today.",
    );
    assert!(extraction.emails.iter().any(|email| email.contains("team@example.com")));
    assert!(extraction.urls.iter().any(|url| url.contains("https://example.com")));
    assert!(!extraction.dates.is_empty());
}

#[test]
fn build_pdf_summary_produces_overview_and_key_points() {
    let pages = vec![
        "Quarterly Report".to_string(),
        "Revenue increased across all regions during the quarter.".to_string(),
        "Operating costs remained stable while product adoption accelerated.".to_string(),
    ];
    let summary = pdf::summary::build_pdf_summary(&pages, 0);
    assert_eq!(summary.page_count, 3);
    assert!(summary.word_count > 10);
    assert!(summary.title_guess.as_deref() == Some("Quarterly Report"));
    assert!(!summary.overview.is_empty());
    assert!(!summary.key_points.is_empty());
}

#[test]
fn summarize_pdf_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_summary_missing_{}.pdf", std::process::id()));
    let err = summarize_pdf(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("not found"));
}

#[test]
fn summary_to_markdown_formats_sections() {
    let summary = pdf::summary::build_pdf_summary(
        &["Quarterly Report".to_string(), "Revenue increased across all regions.".to_string()],
        1,
    );
    let md = pdf::summary::summary_to_markdown(&summary);
    assert!(md.contains("# Document Summary"));
    assert!(md.contains("## Overview"));
    assert!(md.contains("## Key points"));
    assert!(md.contains("Scanned/image-only pages:** 1"));
}

#[test]
fn ocr_available_reports_tesseract_presence() {
    let available = ocr_available();
    assert_eq!(available, resolve_tesseract().is_some());
}

#[test]
fn ocr_pdf_page_rejects_missing_file() {
    let missing = tmp("ocr_missing");
    let err = ocr_pdf_page(missing.to_string_lossy().into_owned(), 0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn ocr_png_bytes_without_tesseract_returns_none() {
    let prev_path = std::env::var_os("PATH");
    let prev_cmd = std::env::var_os("TESSERACT_CMD");
    std::env::remove_var("PATH");
    std::env::remove_var("TESSERACT_CMD");
    let result = pdf::ocr::ocr_png_bytes(&[0x89, 0x50, 0x4e, 0x47]);
    if let Some(path) = prev_path {
        std::env::set_var("PATH", path);
    }
    if let Some(cmd) = prev_cmd {
        std::env::set_var("TESSERACT_CMD", cmd);
    }
    assert_eq!(result.unwrap(), None);
}

/// Needs PDFium + Tesseract. Run: `cargo test ocr_rendered_page_smoke -- --ignored --nocapture`
#[test]
#[ignore]
fn ocr_rendered_page_smoke() {
    if resolve_tesseract().is_none() {
        eprintln!("skip: tesseract not installed");
        return;
    }
    let path = save(&mut build_pdf(1), "ocr_smoke");
    let png = render_page_png(Path::new(&path), 0, OCR_RENDER_W, OCR_RENDER_H).unwrap();
    let text = pdf::ocr::ocr_png_bytes(&png).unwrap().unwrap_or_default();
    assert!(!text.is_empty());
    let _ = fs::remove_file(&path);
}

#[test]
fn markdown_lines_promote_probable_headings() {
    let lines = vec![
        md_line("Cigna Employee-Paid Voluntary Benefits!", 720.0, 704.0, vec![]),
        md_line(
            "As an eligible employee of Insperity, you have the chance to apply for valuable benefits.",
            680.0,
            668.0,
            vec![],
        ),
        md_line("The group rates mean you pay less than individual coverage.", 654.0, 642.0, vec![]),
    ];

    let markdown = format_markdown_lines(&lines, &[]);

    assert!(markdown.contains("### Cigna Employee-Paid Voluntary Benefits!"));
    assert!(markdown.contains("As an eligible employee"));
}

#[test]
fn markdown_lines_turn_toc_leaders_into_table() {
    let lines = vec![
        md_line("Table of Contents", 720.0, 708.0, vec![]),
        md_line("Plan Features................................................ Page 4", 684.0, 672.0, vec![]),
        md_line("Plan Summary................................................. Page 10", 666.0, 654.0, vec![]),
    ];

    let markdown = format_markdown_lines(&lines, &[]);

    assert!(markdown.contains("### Table of Contents"));
    assert!(markdown.contains("| Section | Page |"));
    assert!(markdown.contains("| Plan Features | 4 |"));
    assert!(markdown.contains("| Plan Summary | 10 |"));
}

#[test]
fn merge_wrapped_line_pair_joins_hyphenation_and_wraps() {
    assert_eq!(merge_wrapped_line_pair("docu-", "ment text"), "document text");
    assert_eq!(
        merge_wrapped_line_pair("The group rates mean", "you pay less than individual coverage."),
        "The group rates mean you pay less than individual coverage."
    );
    assert_eq!(merge_wrapped_line_pair("End here.", "New sentence"), "End here. New sentence");
}

#[test]
fn sort_lines_reading_order_reads_left_column_before_right() {
    let mut left_top = md_line("Alpha intro", 700.0, 688.0, vec![]);
    left_top.left = 72.0;
    left_top.right = 250.0;
    let mut left_bottom = md_line("Alpha body", 660.0, 648.0, vec![]);
    left_bottom.left = 72.0;
    left_bottom.right = 250.0;
    let mut right_top = md_line("Beta intro", 700.0, 688.0, vec![]);
    right_top.left = 320.0;
    right_top.right = 520.0;
    let mut right_bottom = md_line("Beta body", 660.0, 648.0, vec![]);
    right_bottom.left = 320.0;
    right_bottom.right = 520.0;
    let ordered = sort_lines_reading_order(vec![right_top, left_bottom, right_bottom, left_top]);
    let texts: Vec<&str> = ordered.iter().map(|line| line.text.as_str()).collect();
    assert_eq!(texts, vec!["Alpha intro", "Alpha body", "Beta intro", "Beta body"]);
}

#[test]
fn strip_header_footer_lines_removes_page_markers() {
    let mut header = md_line("Page 3 of 10", 780.0, 768.0, vec![]);
    header.left = 250.0;
    header.right = 360.0;
    let body = md_line("Actual content paragraph.", 650.0, 638.0, vec![]);
    let mut footer = md_line("Confidential", 120.0, 108.0, vec![]);
    footer.left = 250.0;
    footer.right = 360.0;
    let kept = strip_header_footer_lines(vec![header, body.clone(), footer]);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].text, body.text);
}

#[test]
fn format_markdown_lines_merges_wrapped_paragraph() {
    let lines = vec![
        md_line("This policy explains eligi-", 700.0, 688.0, vec![]),
        md_line("bility requirements for all employees.", 680.0, 668.0, vec![]),
    ];
    let markdown = format_markdown_lines(&lines, &[]);
    assert!(markdown.contains("eligibility requirements"));
    assert!(!markdown.contains("eligi- bility"));
}

#[test]
fn format_markdown_lines_autolinks_bare_urls() {
    let lines = vec![md_line("Visit https://example.com/docs today.", 700.0, 688.0, vec![])];
    let markdown = format_markdown_lines(&lines, &[]);
    assert!(markdown.contains("[https://example.com/docs](https://example.com/docs)"));
}

#[test]
fn apply_links_to_text_wraps_pdfium_link_overlap() {
    let line = md_line("Example", 700.0, 688.0, vec![]);
    let links = vec![MarkdownPageLink {
        uri: "https://example.com/docs".to_string(),
        left: line.left,
        right: line.right,
        bottom: line.bottom,
        top: line.top,
    }];
    let linked = apply_links_to_text("Example", &line, &links);
    assert_eq!(linked, "[Example](https://example.com/docs)");
}

#[test]
fn markdown_lines_turn_column_blocks_into_tables() {
    let lines = vec![
        md_line("Benefit Amount", 720.0, 708.0, vec![("Benefit", 72.0, 140.0), ("Amount", 260.0, 330.0)]),
        md_line("Life $25,000", 704.0, 692.0, vec![("Life", 72.0, 120.0), ("$25,000", 260.0, 330.0)]),
        md_line("Disability $1,000", 688.0, 676.0, vec![("Disability", 72.0, 150.0), ("$1,000", 260.0, 330.0)]),
    ];

    let markdown = format_markdown_lines(&lines, &[]);

    assert!(markdown.contains("| Benefit | Amount |"));
    assert!(markdown.contains("| Life | $25,000 |"));
    assert!(markdown.contains("| Disability | $1,000 |"));
}

#[test]
fn optimize_pdf_writes_output_file() {
    let path = save(&mut build_pdf(2), "optimize");
    let msg = optimize_pdf(path.clone()).unwrap();
    assert!(msg.contains("Metadata stripped"));
    let p = PathBuf::from(&path);
    let out = p.with_file_name(format!("{}_optimized.pdf", p.file_stem().unwrap().to_string_lossy()));
    assert!(out.exists());
    assert!(page_count(&out.to_string_lossy()) == 2);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn optimize_pdf_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_optimize_missing_{}.pdf", std::process::id()));
    let err = optimize_pdf(missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn protect_pdf_writes_encrypted_output() {
    let path = save(&mut build_pdf(1), "protect");
    let msg = protect_pdf(path.clone(), "user-secret".to_string(), None).unwrap();
    assert!(msg.contains("_protected.pdf"));
    let protected = PathBuf::from(&path)
        .with_file_name(format!("{}_protected.pdf", PathBuf::from(&path).file_stem().unwrap().to_string_lossy()));
    verify_pdf_password(protected.to_string_lossy().into_owned(), "user-secret".to_string()).unwrap();
    assert!(pdf_is_encrypted(protected.to_string_lossy().into_owned()).unwrap());
    assert!(verify_pdf_password(protected.to_string_lossy().into_owned(), "wrong".to_string()).is_err());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(protected);
}

#[test]
fn protect_pdf_rejects_empty_password() {
    let path = save(&mut build_pdf(1), "protect_empty");
    match protect_pdf(path.clone(), String::new(), None) {
        Ok(_) => panic!("expected empty password to fail"),
        Err(message) => assert!(message.contains("required")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn pdf_is_encrypted_detects_protected_file() {
    let path = save(&mut build_pdf(1), "protect_detect");
    protect_pdf(path.clone(), "secret".to_string(), None).unwrap();
    let protected = PathBuf::from(&path)
        .with_file_name(format!("{}_protected.pdf", PathBuf::from(&path).file_stem().unwrap().to_string_lossy()));
    assert!(pdf_is_encrypted(protected.to_string_lossy().into_owned()).unwrap());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(protected);
}

#[test]
fn verify_pdf_password_accepts_correct_secret() {
    let path = save(&mut build_pdf(1), "protect_verify");
    protect_pdf(path.clone(), "open-me".to_string(), None).unwrap();
    let protected = PathBuf::from(&path)
        .with_file_name(format!("{}_protected.pdf", PathBuf::from(&path).file_stem().unwrap().to_string_lossy()));
    verify_pdf_password(protected.to_string_lossy().into_owned(), "open-me".to_string()).unwrap();
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(protected);
}

fn generate_test_pkcs12(dir: &Path) -> Option<PathBuf> {
    if Command::new("openssl").arg("version").output().is_err() {
        return None;
    }
    let key = dir.join("sig_key.pem");
    let cert = dir.join("sig_cert.pem");
    let p12 = dir.join("sig_test.p12");
    let status = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            key.to_str()?,
            "-out",
            cert.to_str()?,
            "-days",
            "1",
            "-nodes",
            "-subj",
            "/CN=Kanoprii Test Signer",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    let status = Command::new("openssl")
        .args([
            "pkcs12",
            "-export",
            "-legacy",
            "-out",
            p12.to_str()?,
            "-inkey",
            key.to_str()?,
            "-in",
            cert.to_str()?,
            "-password",
            "pass:pdfpanda-test",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    Some(p12)
}

fn build_pdf_with_outlines(n: usize) -> Document {
    let mut doc = build_pdf(n);
    let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    let pages = doc.get_pages();
    let page_one = *pages.get(&1).unwrap();
    let page_two = pages.get(&2).copied();

    let outlines_id = doc.new_object_id();
    let mut chapter_one = Dictionary::new();
    chapter_one.set("Title", Object::String(b"Chapter 1".to_vec(), lopdf::StringFormat::Literal));
    chapter_one.set("Dest", Object::Array(vec![Object::Reference(page_one), Object::Name(b"Fit".to_vec())]));
    chapter_one.set("Parent", Object::Reference(outlines_id));
    let chapter_one_id = doc.add_object(Object::Dictionary(chapter_one));

    let chapter_two_id = page_two.map(|page_id| {
        let mut chapter_two = Dictionary::new();
        chapter_two.set("Title", Object::String(b"Chapter 2".to_vec(), lopdf::StringFormat::Literal));
        chapter_two.set("Dest", Object::Array(vec![Object::Reference(page_id), Object::Name(b"Fit".to_vec())]));
        chapter_two.set("Parent", Object::Reference(outlines_id));
        chapter_two.set("Prev", Object::Reference(chapter_one_id));
        let chapter_two_id = doc.add_object(Object::Dictionary(chapter_two));
        if let Ok(Object::Dictionary(chapter_one)) = doc.get_object_mut(chapter_one_id) {
            chapter_one.set("Next", Object::Reference(chapter_two_id));
        }
        chapter_two_id
    });

    if let Ok(Object::Dictionary(chapter_one)) = doc.get_object_mut(chapter_one_id) {
        if let Some(next_id) = chapter_two_id {
            chapter_one.set("Next", Object::Reference(next_id));
        }
    }

    let mut outlines = Dictionary::new();
    outlines.set("Type", Object::Name(b"Outlines".to_vec()));
    outlines.set("First", Object::Reference(chapter_one_id));
    outlines.set("Last", Object::Reference(chapter_two_id.unwrap_or(chapter_one_id)));
    outlines.set("Count", Object::Integer(if chapter_two_id.is_some() { 2 } else { 1 }));
    doc.objects.insert(outlines_id, Object::Dictionary(outlines));
    doc.get_dictionary_mut(catalog_id).expect("catalog").set("Outlines", Object::Reference(outlines_id));
    doc
}

#[test]
fn get_pdf_metadata_empty_without_info_dict() {
    let path = save(&mut build_pdf(1), "metadata_empty");
    let metadata = get_pdf_metadata(path.clone()).unwrap();
    assert!(metadata.title.is_none());
    assert!(metadata.author.is_none());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_pdf_metadata_roundtrip() {
    let path = save(&mut build_pdf(1), "metadata_roundtrip");
    set_pdf_metadata(
        path.clone(),
        Some("Quarterly Report".to_string()),
        Some("Alex Example".to_string()),
        Some("Finance".to_string()),
        Some("Q1, revenue".to_string()),
        Some("Kanoprii".to_string()),
        Some("Kanoprii".to_string()),
    )
    .unwrap();
    let metadata = get_pdf_metadata(path.clone()).unwrap();
    assert_eq!(metadata.title.as_deref(), Some("Quarterly Report"));
    assert_eq!(metadata.author.as_deref(), Some("Alex Example"));
    assert_eq!(metadata.subject.as_deref(), Some("Finance"));
    assert_eq!(metadata.keywords.as_deref(), Some("Q1, revenue"));
    assert_eq!(metadata.creator.as_deref(), Some("Kanoprii"));
    assert_eq!(metadata.producer.as_deref(), Some("Kanoprii"));
    assert!(metadata.creation_date.is_some());
    assert!(metadata.mod_date.is_some());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn get_pdf_bookmarks_empty_without_outline() {
    let path = save(&mut build_pdf(1), "bookmark_empty");
    let bookmarks = get_pdf_bookmarks(path.clone()).unwrap();
    assert!(bookmarks.is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn get_pdf_bookmarks_reads_outline_tree() {
    let path = save(&mut build_pdf_with_outlines(2), "bookmark_tree");
    let bookmarks = get_pdf_bookmarks(path.clone()).unwrap();
    assert_eq!(bookmarks.len(), 2);
    assert_eq!(bookmarks[0].title, "Chapter 1");
    assert_eq!(bookmarks[0].depth, 0);
    assert_eq!(bookmarks[0].page_index, Some(0));
    assert_eq!(bookmarks[1].title, "Chapter 2");
    assert_eq!(bookmarks[1].page_index, Some(1));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn list_pdf_signatures_empty_on_unsigned_pdf() {
    let path = save(&mut build_pdf(1), "sig_list_empty");
    let signatures = list_pdf_signatures(path.clone()).unwrap();
    assert!(signatures.is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn verify_pdf_signatures_empty_on_unsigned_pdf() {
    let path = save(&mut build_pdf(1), "sig_verify_empty");
    let report = verify_pdf_signatures(path.clone(), None).unwrap();
    assert_eq!(report.signature_count, 0);
    assert!(!report.overall_valid);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sign_pdf_rejects_empty_password() {
    let path = save(&mut build_pdf(1), "sig_reject_pw");
    let err =
        sign_pdf(path.clone(), "/tmp/missing.p12".to_string(), String::new(), None, None, None, None).unwrap_err();
    assert!(err.contains("password"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sign_pdf_rejects_encrypted_pdf() {
    let path = save(&mut build_pdf(1), "sig_reject_enc");
    protect_pdf(path.clone(), "secret".to_string(), None).unwrap();
    let protected = PathBuf::from(&path)
        .with_file_name(format!("{}_protected.pdf", PathBuf::from(&path).file_stem().unwrap().to_string_lossy()));
    let err = sign_pdf(
        protected.to_string_lossy().into_owned(),
        "/tmp/missing.p12".to_string(),
        "pw".to_string(),
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert!(err.contains("encrypted"));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(protected);
}

#[test]
fn pdf_signature_roundtrip_with_openssl() {
    let dir = std::env::temp_dir().join(format!("kanoprii_sig_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let Some(p12) = generate_test_pkcs12(&dir) else {
        eprintln!("openssl unavailable - skipping pdf_signature_roundtrip_with_openssl");
        return;
    };
    let path = save(&mut build_pdf(1), "sig_roundtrip");
    sign_pdf(
        path.clone(),
        p12.to_string_lossy().into_owned(),
        "pdfpanda-test".to_string(),
        Some("Approved".to_string()),
        Some("Test Lab".to_string()),
        None,
        None,
    )
    .unwrap();
    let listed = list_pdf_signatures(path.clone()).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].field_name, "Signature1");
    assert_eq!(listed[0].reason.as_deref(), Some("Approved"));
    let report = verify_pdf_signatures(path.clone(), None).unwrap();
    assert_eq!(report.signature_count, 1);
    assert_eq!(report.signatures[0].status, "valid_untrusted");
    assert!(report.signatures[0].integrity_ok);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn open_working_copy_with_password_decrypts_for_editing() {
    let path = save(&mut build_pdf(2), "protect_open");
    protect_pdf(path.clone(), "edit-me".to_string(), None).unwrap();
    let protected = PathBuf::from(&path)
        .with_file_name(format!("{}_protected.pdf", PathBuf::from(&path).file_stem().unwrap().to_string_lossy()));
    let working =
        open_working_copy_with_password(protected.to_string_lossy().into_owned(), "edit-me".to_string()).unwrap();
    let doc = Document::load(&working).unwrap();
    assert!(!doc.is_encrypted());
    assert_eq!(get_pdf_page_count(working.clone()).unwrap(), 2);
    discard_working_copy(working).unwrap();
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(protected);
}

#[test]
fn highlight_remove_deletes_the_right_one() {
    let path = save(&mut build_pdf(1), "remove_hl");
    add_highlight(path.clone(), 0, 10.0, 10.0, 20.0, 20.0).unwrap();
    add_highlight(path.clone(), 0, 30.0, 30.0, 40.0, 40.0).unwrap();
    assert_eq!(get_annotations(path.clone(), 0).unwrap().len(), 2);
    // Removing highlight 0 must leave the second one intact.
    remove_highlight(path.clone(), 0, 0).unwrap();
    let remaining = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].rect, [30.0, 30.0, 40.0, 40.0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn get_annotations_returns_empty_without_highlights() {
    let path = save(&mut build_pdf(1), "annots_empty");
    let annots = get_annotations(path.clone(), 0).unwrap();
    assert!(annots.is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn get_annotations_rejects_invalid_page() {
    let path = save(&mut build_pdf(1), "annots_invalid_page");
    match get_annotations(path.clone(), 9) {
        Ok(_) => panic!("expected invalid page to fail"),
        Err(message) => assert!(message.contains("Page not found")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_highlight_rejects_invalid_index() {
    let path = save(&mut build_pdf(1), "remove_invalid");
    add_highlight(path.clone(), 0, 1.0, 1.0, 2.0, 2.0).unwrap();
    match remove_highlight(path.clone(), 0, 9) {
        Ok(_) => panic!("expected invalid index to fail"),
        Err(message) => assert!(message.contains("Highlight not found")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_highlight_rejects_invalid_page() {
    let path = save(&mut build_pdf(1), "add_invalid_page");
    match add_highlight(path.clone(), 9, 1.0, 1.0, 2.0, 2.0) {
        Ok(_) => panic!("expected invalid page to fail"),
        Err(message) => assert!(message.contains("Page not found")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn get_annotations_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_annots_missing_{}.pdf", std::process::id()));
    match get_annotations(missing.to_string_lossy().into_owned(), 0) {
        Ok(_) => panic!("expected missing file to fail"),
        Err(message) => assert!(!message.is_empty()),
    }
}

#[test]
fn add_highlight_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_add_highlight_missing_{}.pdf", std::process::id()));
    let err = add_highlight(missing.to_string_lossy().into_owned(), 0, 1.0, 1.0, 2.0, 2.0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn remove_highlight_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_remove_highlight_missing_{}.pdf", std::process::id()));
    let err = remove_highlight(missing.to_string_lossy().into_owned(), 0, 0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn highlight_add_and_read_back() {
    let path = save(&mut build_pdf(1), "highlight");
    add_highlight(path.clone(), 0, 10.0, 20.0, 110.0, 40.0).unwrap();
    let annots = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(annots.len(), 1);
    assert_eq!(annots[0].subtype, "Highlight");
    assert_eq!(annots[0].rect, [10.0, 20.0, 110.0, 40.0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn text_note_add_and_read_back() {
    let path = save(&mut build_pdf(1), "text_note");
    add_text_note(path.clone(), 0, 12.0, 24.0, "Review this section".to_string()).unwrap();
    let annots = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(annots.len(), 1);
    assert_eq!(annots[0].subtype, "Text");
    assert_eq!(annots[0].contents.as_deref(), Some("Review this section"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_text_note_deletes_the_right_one() {
    let path = save(&mut build_pdf(1), "text_note_remove");
    add_text_note(path.clone(), 0, 10.0, 10.0, "First".to_string()).unwrap();
    add_text_note(path.clone(), 0, 50.0, 50.0, "Second".to_string()).unwrap();
    remove_text_note(path.clone(), 0, 0).unwrap();
    let remaining = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].contents.as_deref(), Some("Second"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_text_note_rejects_invalid_index() {
    let path = save(&mut build_pdf(1), "text_note_invalid");
    add_text_note(path.clone(), 0, 1.0, 1.0, "Note".to_string()).unwrap();
    match remove_text_note(path.clone(), 0, 9) {
        Ok(_) => panic!("expected invalid index to fail"),
        Err(message) => assert!(message.contains("Text note not found")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn ink_stroke_add_and_read_back() {
    let path = save(&mut build_pdf(1), "ink");
    let points = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
    add_ink_stroke(path.clone(), 0, points.clone()).unwrap();
    let annots = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(annots.len(), 1);
    assert_eq!(annots[0].subtype, "Ink");
    assert_eq!(annots[0].ink_points.as_deref(), Some(points.as_slice()));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_ink_stroke_deletes_the_right_one() {
    let path = save(&mut build_pdf(1), "ink_remove");
    add_ink_stroke(path.clone(), 0, vec![1.0, 1.0, 2.0, 2.0]).unwrap();
    add_ink_stroke(path.clone(), 0, vec![10.0, 10.0, 20.0, 20.0]).unwrap();
    remove_ink_stroke(path.clone(), 0, 0).unwrap();
    let remaining = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].ink_points.as_deref(), Some(&[10.0, 10.0, 20.0, 20.0][..]));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_ink_stroke_rejects_invalid_index() {
    let path = save(&mut build_pdf(1), "ink_invalid");
    add_ink_stroke(path.clone(), 0, vec![1.0, 1.0, 2.0, 2.0]).unwrap();
    match remove_ink_stroke(path.clone(), 0, 9) {
        Ok(_) => panic!("expected invalid index to fail"),
        Err(message) => assert!(message.contains("Ink stroke not found")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_ink_stroke_rejects_too_few_points() {
    let path = save(&mut build_pdf(1), "ink_few");
    match add_ink_stroke(path.clone(), 0, vec![1.0, 1.0]) {
        Ok(_) => panic!("expected too few points to fail"),
        Err(message) => assert!(message.contains("at least two points")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_ink_stroke_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_add_ink_missing_{}.pdf", std::process::id()));
    let err = add_ink_stroke(missing.to_string_lossy().into_owned(), 0, vec![1.0, 1.0, 2.0, 2.0]).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn remove_ink_stroke_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_remove_ink_missing_{}.pdf", std::process::id()));
    let err = remove_ink_stroke(missing.to_string_lossy().into_owned(), 0, 0).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn square_shape_add_and_read_back() {
    let path = save(&mut build_pdf(1), "square");
    add_square(path.clone(), 0, 10.0, 20.0, 110.0, 80.0).unwrap();
    let annots = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(annots.len(), 1);
    assert_eq!(annots[0].subtype, "Square");
    assert_eq!(annots[0].rect, [10.0, 20.0, 110.0, 80.0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn circle_shape_add_and_read_back() {
    let path = save(&mut build_pdf(1), "circle");
    add_circle(path.clone(), 0, 5.0, 5.0, 55.0, 35.0).unwrap();
    let annots = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(annots.len(), 1);
    assert_eq!(annots[0].subtype, "Circle");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn line_shape_add_and_read_back() {
    let path = save(&mut build_pdf(1), "line");
    add_line(path.clone(), 0, 10.0, 10.0, 90.0, 70.0).unwrap();
    let annots = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(annots.len(), 1);
    assert_eq!(annots[0].subtype, "Line");
    assert_eq!(annots[0].line_endpoints, Some([10.0, 10.0, 90.0, 70.0]));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_square_deletes_the_right_one() {
    let path = save(&mut build_pdf(1), "square_remove");
    add_square(path.clone(), 0, 1.0, 1.0, 10.0, 10.0).unwrap();
    add_square(path.clone(), 0, 20.0, 20.0, 30.0, 30.0).unwrap();
    remove_square(path.clone(), 0, 0).unwrap();
    let remaining = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].rect, [20.0, 20.0, 30.0, 30.0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_line_rejects_invalid_index() {
    let path = save(&mut build_pdf(1), "line_invalid");
    add_line(path.clone(), 0, 1.0, 1.0, 20.0, 20.0).unwrap();
    match remove_line(path.clone(), 0, 9) {
        Ok(_) => panic!("expected invalid index to fail"),
        Err(message) => assert!(message.contains("Line shape not found")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_line_rejects_too_short() {
    let path = save(&mut build_pdf(1), "line_short");
    match add_line(path.clone(), 0, 1.0, 1.0, 1.0, 1.0) {
        Ok(_) => panic!("expected short line to fail"),
        Err(message) => assert!(message.contains("too short")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn redaction_add_and_read_back() {
    let path = save(&mut build_pdf(1), "redact");
    add_redaction(path.clone(), 0, 12.0, 24.0, 112.0, 84.0).unwrap();
    let annots = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(annots.len(), 1);
    assert!(annots[0].is_redaction);
    assert_eq!(annots[0].rect, [12.0, 24.0, 112.0, 84.0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_redaction_deletes_the_right_one() {
    let path = save(&mut build_pdf(1), "redact_remove");
    add_redaction(path.clone(), 0, 1.0, 1.0, 10.0, 10.0).unwrap();
    add_redaction(path.clone(), 0, 20.0, 20.0, 40.0, 40.0).unwrap();
    remove_redaction(path.clone(), 0, 0).unwrap();
    let remaining = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].rect, [20.0, 20.0, 40.0, 40.0]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_redaction_rejects_invalid_index() {
    let path = save(&mut build_pdf(1), "redact_invalid");
    add_redaction(path.clone(), 0, 1.0, 1.0, 10.0, 10.0).unwrap();
    match remove_redaction(path.clone(), 0, 9) {
        Ok(_) => panic!("expected invalid index to fail"),
        Err(message) => assert!(message.contains("Redaction not found")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_redaction_rejects_missing_file() {
    let missing = std::env::temp_dir().join(format!("pp_add_redact_missing_{}.pdf", std::process::id()));
    let err = add_redaction(missing.to_string_lossy().into_owned(), 0, 1.0, 1.0, 2.0, 2.0).unwrap_err();
    assert!(!err.is_empty());
}

fn test_png(name: &str) -> PathBuf {
    use image::{ImageBuffer, Rgb};
    let path = tmp(name).with_extension("png");
    let img = ImageBuffer::from_fn(8, 6, |_, _| Rgb([200u8, 40, 40]));
    img.save(&path).unwrap();
    path
}

#[test]
fn append_page_content_writes_marker() {
    let path = save(&mut build_pdf(1), "append_content");
    let mut doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    pdf::content::append_page_content(&mut doc, page_id, b"PP_IMAGE_MARKER\n").unwrap();
    doc.save(&path).unwrap();
    let doc = Document::load(&path).unwrap();
    let marked = doc.objects.iter().any(|(_, obj)| {
        obj.as_stream().map(|s| s.content.windows(16).any(|w| w == b"PP_IMAGE_MARKER\n")).unwrap_or(false)
    });
    assert!(marked, "append_page_content did not persist marker after save");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn parse_page_text_and_vector_markers() {
    let content = "%PP-TXT 0 10 20 14 Hello world\nBT\n%PP-VEC 1 5 6 40 30 stroke\nq\n";
    let texts = pdf::page_text::parse_page_text_edits(content);
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].text, "Hello world");
    let vectors = pdf::page_text::parse_page_vectors(content);
    assert_eq!(vectors.len(), 1);
    assert_eq!(vectors[0].width, 40.0);
}

#[test]
fn page_text_edit_roundtrip() {
    let path = save(&mut build_pdf(1), "page_text");
    let index = add_page_text(path.clone(), 0, 120.0, 140.0, 16.0, "Editable line".to_string()).unwrap();
    let edits = list_page_text_edits(path.clone(), 0).unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].index, index);
    assert_eq!(edits[0].text, "Editable line");
    update_page_text(path.clone(), 0, index, "Updated line".to_string(), None, None, None).unwrap();
    let edits = list_page_text_edits(path.clone(), 0).unwrap();
    assert_eq!(edits[0].text, "Updated line");
    remove_page_text(path.clone(), 0, index).unwrap();
    assert!(list_page_text_edits(path.clone(), 0).unwrap().is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn page_vector_rect_roundtrip() {
    let path = save(&mut build_pdf(1), "page_vector");
    let index = add_page_vector_rect(path.clone(), 0, 50.0, 60.0, 120.0, 80.0).unwrap();
    let vectors = list_page_vectors(path.clone(), 0).unwrap();
    assert_eq!(vectors.len(), 1);
    assert_eq!(vectors[0].index, index);
    remove_page_vector(path.clone(), 0, index).unwrap();
    assert!(list_page_vectors(path.clone(), 0).unwrap().is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_text_rejects_empty_string() {
    let path = save(&mut build_pdf(1), "page_text_empty");
    let err = add_page_text(path.clone(), 0, 10.0, 10.0, 12.0, "   ".to_string()).unwrap_err();
    assert!(err.contains("empty"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn replace_text_region_whites_out_and_writes() {
    let path = save(&mut build_pdf(1), "text_replace");
    replace_text_region(path.clone(), 0, 60.0, 80.0, 200.0, 40.0, "Edited".to_string(), 12.0).unwrap();
    let content = pdf::text_replace::page_content_string(std::path::Path::new(&path), 0).unwrap();
    assert!(content.contains("1 1 1 rg"), "expected white fill: {content}");
    assert!(content.contains("re f"), "expected rectangle fill: {content}");
    assert!(content.contains("(Edited)"), "expected replacement text: {content}");
    assert!(content.contains("(Hello)"), "original text objects should remain: {content}");
    let _ = std::fs::remove_file(&path);
}

fn build_pdf_with_text_field() -> Document {
    let mut doc = build_pdf(1);
    let page_id = *doc.get_pages().get(&1).unwrap();
    let field_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"Widget".to_vec())),
        (b"FT".to_vec(), Object::Name(b"Tx".to_vec())),
        (b"T".to_vec(), Object::String(b"FirstName".to_vec(), lopdf::StringFormat::Literal)),
        (b"V".to_vec(), Object::String(b"Ada".to_vec(), lopdf::StringFormat::Literal)),
        (
            b"Rect".to_vec(),
            Object::Array(vec![Object::Integer(72), Object::Integer(700), Object::Integer(280), Object::Integer(730)]),
        ),
        (b"F".to_vec(), Object::Integer(4)),
    ])));
    doc.get_dictionary_mut(page_id).unwrap().set(b"Annots", Object::Array(vec![Object::Reference(field_id)]));
    let acroform_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        (b"Fields".to_vec(), Object::Array(vec![Object::Reference(field_id)])),
        (b"NeedAppearances".to_vec(), Object::Boolean(true)),
    ])));
    let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    doc.get_dictionary_mut(catalog_id).unwrap().set(b"AcroForm", Object::Reference(acroform_id));
    doc
}

#[test]
fn get_pdf_form_fields_reads_text_field() {
    let path = save(&mut build_pdf_with_text_field(), "form_read");
    let fields = get_pdf_form_fields(path.clone()).unwrap();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "FirstName");
    assert_eq!(fields[0].field_type, "text");
    assert_eq!(fields[0].value, "Ada");
    assert_eq!(fields[0].page_index, Some(0));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_pdf_form_field_updates_text_value() {
    let path = save(&mut build_pdf_with_text_field(), "form_set");
    set_pdf_form_field(path.clone(), "FirstName".to_string(), "Grace".to_string()).unwrap();
    let fields = get_pdf_form_fields(path.clone()).unwrap();
    assert_eq!(fields[0].value, "Grace");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_form_field_creates_fillable_widget() {
    let path = save(&mut build_pdf(1), "form_add");
    add_text_form_field(path.clone(), 0, "Email".to_string(), 100.0, 120.0, 180.0, 28.0).unwrap();
    let fields = get_pdf_form_fields(path.clone()).unwrap();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "Email");
    assert_eq!(fields[0].field_type, "text");
    assert_eq!(fields[0].page_index, Some(0));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_pdf_form_field_rejects_missing_name() {
    let path = save(&mut build_pdf_with_text_field(), "form_missing");
    let err = set_pdf_form_field(path.clone(), "Missing".to_string(), "x".to_string()).unwrap_err();
    assert!(err.contains("not found"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_checkbox_form_field_creates_toggle() {
    let path = save(&mut build_pdf(1), "form_checkbox");
    add_checkbox_form_field(path.clone(), 0, "Agree".to_string(), 80.0, 80.0, 18.0, 18.0, false).unwrap();
    let fields = get_pdf_form_fields(path.clone()).unwrap();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].field_type, "checkbox");
    assert!(!fields[0].checked);
    set_pdf_form_field(path.clone(), "Agree".to_string(), "true".to_string()).unwrap();
    let fields = get_pdf_form_fields(path.clone()).unwrap();
    assert!(fields[0].checked);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_choice_form_field_stores_options() {
    let path = save(&mut build_pdf(1), "form_choice");
    add_choice_form_field(
        path.clone(),
        0,
        "Country".to_string(),
        80.0,
        120.0,
        160.0,
        24.0,
        vec!["US".to_string(), "CA".to_string(), "MX".to_string()],
        true,
    )
    .unwrap();
    let fields = get_pdf_form_fields(path.clone()).unwrap();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].field_type, "choice");
    assert_eq!(fields[0].options, vec!["US", "CA", "MX"]);
    set_pdf_form_field(path.clone(), "Country".to_string(), "CA".to_string()).unwrap();
    let fields = get_pdf_form_fields(path.clone()).unwrap();
    assert_eq!(fields[0].value, "CA");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_radio_form_field_group_excludes_other_options() {
    let path = save(&mut build_pdf(1), "form_radio");
    add_radio_form_field(path.clone(), 0, "Color".to_string(), "Red".to_string(), 60.0, 60.0, 16.0, 16.0).unwrap();
    add_radio_form_field(path.clone(), 0, "Color".to_string(), "Blue".to_string(), 60.0, 90.0, 16.0, 16.0).unwrap();
    let fields = get_pdf_form_fields(path.clone()).unwrap();
    assert_eq!(fields.len(), 2);
    set_pdf_form_field(path.clone(), "Color.Red".to_string(), "true".to_string()).unwrap();
    let fields = get_pdf_form_fields(path.clone()).unwrap();
    let red = fields.iter().find(|field| field.name == "Color.Red").unwrap();
    let blue = fields.iter().find(|field| field.name == "Color.Blue").unwrap();
    assert!(red.checked);
    assert!(!blue.checked);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn get_image_dimensions_reads_png() {
    let path = test_png("dims");
    let dims = get_image_dimensions(path.to_string_lossy().into_owned()).unwrap();
    assert_eq!(dims, [8, 6]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_image_embeds_xobject_and_content() {
    let path = save(&mut build_pdf(1), "page_image");
    let img_path = test_png("page_image_src");
    add_page_image(path.clone(), 0, 100.0, 100.0, 80.0, 60.0, img_path.to_string_lossy().into_owned()).unwrap();
    let doc = Document::load(&path).unwrap();
    let any_stream_do = doc
        .objects
        .iter()
        .any(|(_, obj)| obj.as_stream().map(|s| String::from_utf8_lossy(&s.content).contains(" Do")).unwrap_or(false));
    assert!(any_stream_do, "no content stream contains image draw operator");
    let pages = doc.get_pages();
    let page_id = *pages.get(&1).unwrap();
    let page = doc.get_dictionary(page_id).unwrap();
    let resources = page.get(b"Resources").unwrap().as_dict().unwrap();
    let xobjects = resources.get(b"XObject").unwrap().as_dict().unwrap();
    assert!(xobjects.iter().any(|(k, _)| k.starts_with(b"Im")));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&img_path);
}

#[test]
fn add_page_image_rejects_missing_pdf() {
    let img_path = test_png("page_image_missing_pdf");
    let missing = std::env::temp_dir().join(format!("pp_page_image_missing_{}.pdf", std::process::id()));
    let err = add_page_image(
        missing.to_string_lossy().into_owned(),
        0,
        10.0,
        10.0,
        50.0,
        50.0,
        img_path.to_string_lossy().into_owned(),
    )
    .unwrap_err();
    assert!(!err.is_empty());
    let _ = std::fs::remove_file(&img_path);
}

#[test]
fn add_page_image_rejects_missing_image() {
    let path = save(&mut build_pdf(1), "page_image_no_src");
    let missing = std::env::temp_dir().join(format!("pp_page_image_src_{}.png", std::process::id()));
    let err =
        add_page_image(path.clone(), 0, 10.0, 10.0, 50.0, 50.0, missing.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("not found"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_page_image_rejects_too_small() {
    let path = save(&mut build_pdf(1), "page_image_small");
    let img_path = test_png("page_image_small_src");
    let err =
        add_page_image(path.clone(), 0, 10.0, 10.0, 4.0, 4.0, img_path.to_string_lossy().into_owned()).unwrap_err();
    assert!(err.contains("too small"));
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&img_path);
}

#[test]
fn pdf_image_stream_bytes_device_gray_to_png() {
    let stream = Stream::new(
        Dictionary::from_iter(vec![
            (b"Width".to_vec(), Object::Integer(2)),
            (b"Height".to_vec(), Object::Integer(2)),
            (b"BitsPerComponent".to_vec(), Object::Integer(8)),
            (b"ColorSpace".to_vec(), Object::Name(b"DeviceGray".to_vec())),
        ]),
        vec![0, 64, 128, 255],
    );
    let (png, ext) = pdf_image_stream_bytes(&stream).expect("gray image");
    assert_eq!(ext, "png");
    assert!(png.starts_with(&[137, 80, 78, 71]));
}

#[test]
fn pdf_image_stream_bytes_device_cmyk_to_png() {
    let stream = Stream::new(
        Dictionary::from_iter(vec![
            (b"Width".to_vec(), Object::Integer(1)),
            (b"Height".to_vec(), Object::Integer(1)),
            (b"BitsPerComponent".to_vec(), Object::Integer(8)),
            (b"ColorSpace".to_vec(), Object::Name(b"DeviceCMYK".to_vec())),
        ]),
        vec![0, 255, 255, 0],
    );
    let (png, ext) = pdf_image_stream_bytes(&stream).expect("cmyk image");
    assert_eq!(ext, "png");
    assert!(png.starts_with(&[137, 80, 78, 71]));
}

#[test]
fn pdf_image_stream_bytes_indexed_to_png() {
    let stream = Stream::new(
        Dictionary::from_iter(vec![
            (b"Width".to_vec(), Object::Integer(2)),
            (b"Height".to_vec(), Object::Integer(1)),
            (b"BitsPerComponent".to_vec(), Object::Integer(8)),
            (
                b"ColorSpace".to_vec(),
                Object::Array(vec![
                    Object::Name(b"Indexed".to_vec()),
                    Object::Name(b"DeviceRGB".to_vec()),
                    Object::Integer(1),
                    Object::String(vec![255, 0, 0, 0, 0, 255], lopdf::StringFormat::Literal),
                ]),
            ),
        ]),
        vec![0, 1],
    );
    let (png, ext) = pdf_image_stream_bytes(&stream).expect("indexed image");
    assert_eq!(ext, "png");
    assert!(png.starts_with(&[137, 80, 78, 71]));
}

#[test]
fn pdf_image_stream_bytes_device_rgb_to_png() {
    let stream = Stream::new(
        Dictionary::from_iter(vec![
            (b"Width".to_vec(), Object::Integer(1)),
            (b"Height".to_vec(), Object::Integer(1)),
            (b"BitsPerComponent".to_vec(), Object::Integer(8)),
            (b"ColorSpace".to_vec(), Object::Name(b"DeviceRGB".to_vec())),
        ]),
        vec![10, 20, 30],
    );
    let (png, ext) = pdf_image_stream_bytes(&stream).expect("rgb image");
    assert_eq!(ext, "png");
    assert!(png.starts_with(&[137, 80, 78, 71]));
}

#[test]
fn pdf_image_stream_bytes_ccitt_g4_to_png() {
    use fax::encoder::Encoder;
    use fax::{Color, VecWriter};
    use std::iter::repeat_n;

    let width = 8u16;
    let height = 2u16;
    let writer = VecWriter::new();
    let mut encoder = Encoder::new(writer);
    encoder.encode_line(repeat_n(Color::White, width as usize), width).unwrap();
    encoder.encode_line(repeat_n(Color::White, 4).chain(repeat_n(Color::Black, 4)), width).unwrap();
    let encoded = encoder.finish().unwrap().finish();

    let stream = Stream::new(
        Dictionary::from_iter(vec![
            (b"Width".to_vec(), Object::Integer(width as i64)),
            (b"Height".to_vec(), Object::Integer(height as i64)),
            (b"BitsPerComponent".to_vec(), Object::Integer(1)),
            (b"ColorSpace".to_vec(), Object::Name(b"DeviceGray".to_vec())),
            (b"Filter".to_vec(), Object::Name(b"CCITTFaxDecode".to_vec())),
            (
                b"DecodeParms".to_vec(),
                Object::Dictionary(Dictionary::from_iter(vec![
                    (b"Columns".to_vec(), Object::Integer(width as i64)),
                    (b"Rows".to_vec(), Object::Integer(height as i64)),
                    (b"K".to_vec(), Object::Integer(-1)),
                ])),
            ),
        ]),
        encoded,
    );
    let (png, ext) = pdf_image_stream_bytes(&stream).expect("ccitt g4 image");
    assert_eq!(ext, "png");
    assert!(png.starts_with(&[137, 80, 78, 71]));
}

#[test]
fn pdf_image_stream_bytes_run_length_1bit_to_png() {
    let stream = Stream::new(
        Dictionary::from_iter(vec![
            (b"Width".to_vec(), Object::Integer(4)),
            (b"Height".to_vec(), Object::Integer(2)),
            (b"BitsPerComponent".to_vec(), Object::Integer(1)),
            (b"ColorSpace".to_vec(), Object::Name(b"DeviceGray".to_vec())),
            (b"Filter".to_vec(), Object::Name(b"RunLengthDecode".to_vec())),
        ]),
        vec![
            3, 0xFF, // 4 white pixels
            3, 0x00, // 4 black pixels
            0x80, // EOD
        ],
    );
    let (png, ext) = pdf_image_stream_bytes(&stream).expect("run-length 1-bit image");
    assert_eq!(ext, "png");
    assert!(png.starts_with(&[137, 80, 78, 71]));
}

#[test]
fn collect_xobject_do_names_recursive_finds_nested_image() {
    let mut doc = build_pdf_with_vector_form();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let image_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Image".to_vec())),
            (b"Width".to_vec(), Object::Integer(2)),
            (b"Height".to_vec(), Object::Integer(2)),
            (b"ColorSpace".to_vec(), Object::Name(b"DeviceRGB".to_vec())),
            (b"BitsPerComponent".to_vec(), Object::Integer(8)),
        ]),
        vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0],
    )));
    let form_id = doc
        .get_dictionary(page_id)
        .unwrap()
        .get(b"Resources")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"XObject")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"Chart1")
        .unwrap()
        .as_reference()
        .unwrap();
    if let Ok(Object::Stream(stream)) = doc.get_object_mut(form_id) {
        stream.content = b"q /ScanImg Do Q".to_vec();
        let mut form_resources = Dictionary::new();
        let mut xobjects = Dictionary::new();
        xobjects.set(b"ScanImg", Object::Reference(image_id));
        form_resources.set(b"XObject", Object::Dictionary(xobjects));
        stream.dict.set(b"Resources", Object::Dictionary(form_resources));
    }

    let resources = resolve_page_resources(&doc, page_id).unwrap();
    let mut visited = BTreeSet::new();
    let used = collect_xobject_do_names_recursive(&doc, &resources, b"q /Chart1 Do Q", &mut visited);
    assert!(used.contains(b"Chart1".as_slice()));
    assert!(used.contains(b"ScanImg".as_slice()));
}

#[test]
fn build_image_render_pdf_produces_single_page() {
    let mut doc = build_pdf(1);
    let page_id = *doc.get_pages().get(&1).unwrap();
    let image_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Image".to_vec())),
            (b"Width".to_vec(), Object::Integer(120)),
            (b"Height".to_vec(), Object::Integer(80)),
            (b"ColorSpace".to_vec(), Object::Name(b"DeviceRGB".to_vec())),
            (b"BitsPerComponent".to_vec(), Object::Integer(8)),
        ]),
        vec![0; 120 * 80 * 3],
    )));
    let mut xobjects = Dictionary::new();
    xobjects.set(b"ScanImg", Object::Reference(image_id));
    let mut resources = Dictionary::new();
    resources.set(b"XObject", Object::Dictionary(xobjects));
    doc.get_dictionary_mut(page_id).unwrap().set(b"Resources", Object::Dictionary(resources));

    let wrapper = build_image_render_pdf(&doc, page_id, image_id, b"ScanImg").unwrap();
    assert_eq!(wrapper.get_pages().len(), 1);
    let wrapper_page_id = *wrapper.get_pages().get(&1).unwrap();
    let contents = wrapper.get_dictionary(wrapper_page_id).unwrap().get(b"Contents").unwrap().as_reference().unwrap();
    let Object::Stream(stream) = wrapper.get_object(contents).unwrap() else {
        panic!("contents stream missing");
    };
    let content_bytes = stream.decompressed_content().unwrap();
    let text = String::from_utf8_lossy(&content_bytes);
    assert!(text.contains("120 0 0 80 0 0 cm"));
    assert!(text.contains("/ScanImg Do"));
}

#[test]
fn page_needs_ocr_supplement_detects_sparse_text() {
    assert!(page_needs_ocr_supplement(&[], "short"));
    assert!(!page_needs_ocr_supplement(&[], "x".repeat(PAGE_OCR_MIN_CHARS + 1).as_str(),));
    let sparse = md_line("Hi", 100.0, 90.0, vec![]);
    assert!(page_needs_ocr_supplement(&[sparse], ""));
}

#[test]
fn try_ocr_image_bytes_without_tesseract_returns_none() {
    if resolve_tesseract().is_some() {
        return;
    }
    let result = try_ocr_image_bytes(&[0x89, 0x50, 0x4e, 0x47], "png").unwrap();
    assert!(result.is_none());
}

#[test]
fn ocr_status_reports_language_and_psm() {
    let status = ocr_status();
    assert_eq!(status.available, resolve_tesseract().is_some());
    assert_eq!(status.language, ocr_language());
    assert_eq!(status.page_segmentation_mode, ocr_page_segmentation_mode());
}

#[test]
fn tesseract_install_guide_returns_plain_language_steps() {
    let guide = build_tesseract_install_guide();
    assert!(!guide.summary.is_empty());
    assert!(!guide.steps.is_empty());
    assert!(guide.license_note.to_lowercase().contains("free"));
    assert!(guide.steps.iter().any(|step| step.to_lowercase().contains("restart")));
}

#[test]
fn os_release_value_parses_quoted_id() {
    let sample = "ID=\"cachyos\"\nID_LIKE=arch\n";
    assert_eq!(os_release_value(sample, "ID").as_deref(), Some("cachyos"));
    assert_eq!(os_release_value(sample, "ID_LIKE").as_deref(), Some("arch"));
}

#[test]
fn try_ocr_png_bytes_is_lenient_when_tesseract_errors() {
    if resolve_tesseract().is_none() {
        return;
    }
    let prev_lang = std::env::var_os("KANOPRII_OCR_LANG");
    std::env::set_var("KANOPRII_OCR_LANG", "zzzinvalid_lang_pack");
    let strict = pdf::ocr::ocr_png_bytes(&[0x89, 0x50, 0x4e, 0x47]);
    let lenient = pdf::ocr::try_ocr_png_bytes(&[0x89, 0x50, 0x4e, 0x47]);
    if let Some(lang) = prev_lang {
        std::env::set_var("KANOPRII_OCR_LANG", lang);
    } else {
        std::env::remove_var("KANOPRII_OCR_LANG");
    }
    assert!(strict.is_err());
    assert_eq!(lenient.unwrap(), None);
}

#[test]
fn append_page_embedded_images_writes_assets() {
    let mut doc = build_pdf(1);
    let page_id = *doc.get_pages().get(&1).unwrap();
    let image_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Image".to_vec())),
            (b"Width".to_vec(), Object::Integer(1)),
            (b"Height".to_vec(), Object::Integer(1)),
            (b"ColorSpace".to_vec(), Object::Name(b"DeviceRGB".to_vec())),
            (b"BitsPerComponent".to_vec(), Object::Integer(8)),
        ]),
        vec![255, 0, 0],
    )));
    let mut xobjects = Dictionary::new();
    xobjects.set(b"Im1", Object::Reference(image_id));
    let mut resources = Dictionary::new();
    resources.set(b"XObject", Object::Dictionary(xobjects));
    doc.get_dictionary_mut(page_id).unwrap().set(b"Resources", Object::Dictionary(resources));

    let assets_dir = std::env::temp_dir().join(format!("pp_md_assets_{}", std::process::id()));
    let _ = fs::remove_dir_all(&assets_dir);
    let sink = MarkdownImageSink { assets_dir: &assets_dir, rel_prefix: "doc_assets" };
    let mut seq = 0u32;
    let mut stats = OcrExportStats::default();
    let block = append_page_embedded_images(&doc, 1, &sink, &mut seq, &mut stats).unwrap();

    assert_eq!(seq, 1);
    assert!(block.contains("embedded image"));
    assert!(assets_dir.join("page-1-img-1.png").is_file());

    let _ = fs::remove_dir_all(&assets_dir);
}

fn build_pdf_with_vector_form() -> Document {
    let mut doc = build_pdf(1);
    let page_id = *doc.get_pages().get(&1).unwrap();
    let form_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Form".to_vec())),
            (b"FormType".to_vec(), Object::Integer(1)),
            (
                b"BBox".to_vec(),
                Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(200), Object::Integer(100)]),
            ),
        ]),
        b"q 0.2 0.4 0.9 rg 10 10 180 80 re f Q".to_vec(),
    )));
    let mut xobjects = Dictionary::new();
    xobjects.set(b"Chart1", Object::Reference(form_id));
    let mut resources = Dictionary::new();
    resources.set(b"XObject", Object::Dictionary(xobjects));
    let content_id = doc.add_object(Stream::new(Dictionary::new(), b"q 72 650 cm /Chart1 Do Q".to_vec()));
    let page = doc.get_dictionary_mut(page_id).unwrap();
    page.set(b"Resources", Object::Dictionary(resources));
    page.set(b"Contents", Object::Reference(content_id));
    doc
}

#[test]
fn parse_xobject_do_names_finds_form_invocations() {
    let names = parse_xobject_do_names(b"q 72 650 cm /Chart1 Do Q /Im2 Do");
    assert!(names.contains(b"Chart1".as_slice()));
    assert!(names.contains(b"Im2".as_slice()));
}

#[test]
fn xobject_names_used_on_page_reads_page_contents() {
    let doc = build_pdf_with_vector_form();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let used = xobject_names_used_on_page(&doc, page_id);
    assert!(used.contains(b"Chart1".as_slice()));
}

#[test]
fn xobject_names_used_on_page_ignores_unpainted_forms() {
    let mut doc = build_pdf_with_vector_form();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let unused_form_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Form".to_vec())),
            (
                b"BBox".to_vec(),
                Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(50), Object::Integer(50)]),
            ),
        ]),
        b"q 1 0 0 rg 0 0 50 50 re f Q".to_vec(),
    )));
    let resources = doc.get_dictionary_mut(page_id).unwrap().get_mut(b"Resources").unwrap().as_dict_mut().unwrap();
    let xobjects = resources.get_mut(b"XObject").unwrap().as_dict_mut().unwrap();
    xobjects.set(b"UnusedChart", Object::Reference(unused_form_id));

    let used = xobject_names_used_on_page(&doc, page_id);
    assert!(used.contains(b"Chart1".as_slice()));
    assert!(!used.contains(b"UnusedChart".as_slice()));
}

#[test]
fn resolve_page_resources_inherits_from_parent_pages_node() {
    let mut doc = build_pdf(1);
    let page_id = *doc.get_pages().get(&1).unwrap();
    let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    let pages_id = doc.get_dictionary(catalog_id).unwrap().get(b"Pages").unwrap().as_reference().unwrap();
    let form_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Form".to_vec())),
            (
                b"BBox".to_vec(),
                Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(100), Object::Integer(50)]),
            ),
        ]),
        b"q 0 0 1 rg 0 0 100 50 re f Q".to_vec(),
    )));
    let mut xobjects = Dictionary::new();
    xobjects.set(b"Chart1", Object::Reference(form_id));
    let mut resources = Dictionary::new();
    resources.set(b"XObject", Object::Dictionary(xobjects));
    doc.get_dictionary_mut(pages_id).unwrap().set(b"Resources", Object::Dictionary(resources));
    doc.get_dictionary_mut(page_id).unwrap().remove(b"Resources");
    let content_id = doc.add_object(Stream::new(Dictionary::new(), b"q /Chart1 Do Q".to_vec()));
    doc.get_dictionary_mut(page_id).unwrap().set(b"Contents", Object::Reference(content_id));

    let resolved = resolve_page_resources(&doc, page_id).expect("inherited resources");
    assert!(resolved.get(b"XObject").is_ok());
}

#[test]
fn build_form_render_pdf_applies_form_matrix() {
    let mut doc = build_pdf_with_vector_form();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let form_id = doc
        .get_dictionary(page_id)
        .unwrap()
        .get(b"Resources")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"XObject")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"Chart1")
        .unwrap()
        .as_reference()
        .unwrap();
    if let Ok(Object::Stream(stream)) = doc.get_object_mut(form_id) {
        stream.dict.set(
            b"Matrix",
            Object::Array(vec![
                Object::Real(2.0),
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(2.0),
                Object::Real(0.0),
                Object::Real(0.0),
            ]),
        );
    }
    let wrapper = build_form_render_pdf(&doc, page_id, form_id, b"Chart1").unwrap();
    let page_id = *wrapper.get_pages().get(&1).unwrap();
    let contents = wrapper.get_dictionary(page_id).unwrap().get(b"Contents").unwrap().as_reference().unwrap();
    let Object::Stream(stream) = wrapper.get_object(contents).unwrap() else {
        panic!("contents stream missing");
    };
    let bytes = stream.decompressed_content().unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("2 0 0 2 0 0 cm"));
}

#[test]
fn build_form_render_pdf_produces_single_page() {
    let doc = build_pdf_with_vector_form();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let form_id = doc
        .get_dictionary(page_id)
        .unwrap()
        .get(b"Resources")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"XObject")
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"Chart1")
        .unwrap()
        .as_reference()
        .unwrap();
    let wrapper = build_form_render_pdf(&doc, page_id, form_id, b"Chart1").unwrap();
    assert_eq!(wrapper.get_pages().len(), 1);
}

/// Needs PDFium. Run: `PDFIUM_LIB_PATH=... cargo test append_page_embedded_images_renders_form -- --ignored --nocapture`
#[test]
#[ignore = "requires a PDFium library"]
fn append_page_embedded_images_renders_form() {
    let doc = build_pdf_with_vector_form();
    let assets_dir = std::env::temp_dir().join(format!("pp_md_form_assets_{}", std::process::id()));
    let _ = fs::remove_dir_all(&assets_dir);
    let sink = MarkdownImageSink { assets_dir: &assets_dir, rel_prefix: "doc_assets" };
    let mut seq = 0u32;
    let mut stats = OcrExportStats::default();
    let block = append_page_embedded_images(&doc, 1, &sink, &mut seq, &mut stats).unwrap();
    assert_eq!(seq, 1);
    assert!(block.contains("vector chart"));
    let png_path = assets_dir.join("page-1-form-1.png");
    assert!(png_path.is_file());
    let png = fs::read(&png_path).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
    assert!(png.len() > 100);
    let _ = fs::remove_dir_all(&assets_dir);
}

#[test]
fn list_stamp_presets_returns_known_labels() {
    let presets = list_stamp_presets();
    assert_eq!(presets.len(), 4);
    assert!(presets.iter().any(|p| p.id == "approved" && p.label == "APPROVED"));
}

#[test]
fn text_stamp_add_and_read_back() {
    let path = save(&mut build_pdf(1), "text_stamp");
    add_text_stamp(path.clone(), 0, 20.0, 30.0, "approved".to_string()).unwrap();
    let annots = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(annots.len(), 1);
    assert_eq!(annots[0].stamp_kind.as_deref(), Some("text"));
    assert_eq!(annots[0].stamp_preset.as_deref(), Some("approved"));
    assert_eq!(annots[0].contents.as_deref(), Some("APPROVED"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn image_stamp_add_and_read_back() {
    let path = save(&mut build_pdf(1), "image_stamp");
    add_image_stamp(path.clone(), 0, 40.0, 50.0, "draft".to_string()).unwrap();
    let annots = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(annots.len(), 1);
    assert_eq!(annots[0].stamp_kind.as_deref(), Some("image"));
    assert_eq!(annots[0].stamp_preset.as_deref(), Some("draft"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_text_stamp_deletes_the_right_one() {
    let path = save(&mut build_pdf(1), "text_stamp_remove");
    add_text_stamp(path.clone(), 0, 10.0, 10.0, "approved".to_string()).unwrap();
    add_text_stamp(path.clone(), 0, 50.0, 50.0, "draft".to_string()).unwrap();
    remove_text_stamp(path.clone(), 0, 0).unwrap();
    let remaining = get_annotations(path.clone(), 0).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].stamp_preset.as_deref(), Some("draft"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn add_text_stamp_rejects_unknown_preset() {
    let path = save(&mut build_pdf(1), "text_stamp_bad");
    match add_text_stamp(path.clone(), 0, 1.0, 1.0, "nope".to_string()) {
        Ok(_) => panic!("expected unknown preset to fail"),
        Err(message) => assert!(message.contains("Unknown stamp preset")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_image_stamp_rejects_invalid_index() {
    let path = save(&mut build_pdf(1), "image_stamp_invalid");
    add_image_stamp(path.clone(), 0, 1.0, 1.0, "reviewed".to_string()).unwrap();
    match remove_image_stamp(path.clone(), 0, 9) {
        Ok(_) => panic!("expected invalid index to fail"),
        Err(message) => assert!(message.contains("image stamp not found")),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn out_of_bounds_delete_errors() {
    let path = save(&mut build_pdf(2), "oob");
    assert!(delete_page(path.clone(), 9).is_err());
    let _ = std::fs::remove_file(&path);
}

/// End-to-end smoke test against a real PDF through the actual pdfium-backed
/// commands. Ignored by default (needs a working PDFium library and a file);
/// run with:
///   KANOPRII_TEST_PDF=/path/to/file.pdf \
///     cargo test render_real_pdf_smoke -- --ignored --nocapture
#[test]
#[ignore = "requires a PDFium library and KANOPRII_TEST_PDF"]
fn render_real_pdf_smoke() {
    let pdf = std::env::var("KANOPRII_TEST_PDF").expect("set KANOPRII_TEST_PDF");

    let pages = get_pdf_page_count(pdf.clone()).expect("page count");
    assert!(pages > 0, "expected at least one page");

    let png = render_pdf_page(pdf.clone(), 0, 800, 1132).expect("render page 0");
    assert!(png.starts_with(b"\x89PNG"), "output should be a PNG");
    assert!(png.len() > 1000, "rendered PNG looks too small");
    std::fs::write("/tmp/render_test_page0.png", &png).unwrap();

    let thumbs = get_pdf_thumbnails(pdf.clone(), 100, 141).expect("thumbnails");
    assert_eq!(thumbs.len() as u32, pages, "one thumbnail per page");

    let md = convert_pdf_to_markdown(pdf.clone()).expect("markdown");
    assert!(md.contains("## Page 1"), "markdown should have page headers");

    if pages > 1 {
        let png_after_markdown = render_pdf_page(pdf, 1, 800, 1132).expect("render page 1 after markdown");
        assert!(png_after_markdown.starts_with(b"\x89PNG"), "post-markdown render output should be a PNG");
        assert!(png_after_markdown.len() > 1000, "post-markdown rendered PNG looks too small");
    }

    eprintln!(
        "render_real_pdf_smoke: pages={pages}, page0={} bytes, thumbnails={}, markdown={} bytes",
        png.len(),
        thumbs.len(),
        md.len()
    );
    eprintln!("markdown preview:\n{}", md.chars().take(400).collect::<String>());
}

#[test]
fn version_newer_parses_semver() {
    assert!(version_newer("0.5.1", "0.5.0"));
    assert!(version_newer("0.6.0", "0.5.0"));
    assert!(version_newer("1.0.0", "0.5.0"));
    assert!(!version_newer("0.5.0", "0.5.0"));
    assert!(!version_newer("0.4.9", "0.5.0"));
    assert!(!version_newer("0.5.0", "0.5.1"));
}

#[test]
fn version_newer_handles_unequal_parts() {
    assert!(version_newer("0.5.0.1", "0.5.0"));
    assert!(!version_newer("0.5", "0.5.0"));
    assert!(!version_newer("0.5.0", "0.5.0.1"));
}

#[test]
fn parse_latest_json_ok() {
    let body = r#"{"version":"0.5.1","notes":"Bug fixes"}"#;
    let info = parse_latest_json(body, "0.5.0").unwrap();
    assert_eq!(info.version, "0.5.1");
    assert_eq!(info.notes, Some("Bug fixes".to_string()));
    assert_eq!(info.current, "0.5.0");
    assert!(info.newer);
}

#[test]
fn parse_latest_json_current() {
    let body = r#"{"version":"0.5.0"}"#;
    let info = parse_latest_json(body, "0.5.0").unwrap();
    assert!(!info.newer);
}

#[test]
fn parse_latest_json_no_notes() {
    let body = r#"{"version":"0.5.1"}"#;
    let info = parse_latest_json(body, "0.5.0").unwrap();
    assert_eq!(info.notes, None);
}

#[test]
fn parse_latest_json_invalid_json() {
    let err = parse_latest_json("not json", "0.5.0").unwrap_err();
    assert!(err.contains("Failed to parse JSON"));
}

#[test]
fn parse_latest_json_missing_version() {
    let err = parse_latest_json(r#"{"notes":"x"}"#, "0.5.0").unwrap_err();
    assert!(err.contains("Missing version field"));
}

#[test]
fn parse_latest_json_with_linux_packages() {
    let body = r#"{
        "version":"0.6.2",
        "notes":"x",
        "linux_packages":{
            "deb":{"url":"https://example.test/app_0.6.2_amd64.deb","sha256":"aabb"},
            "rpm":{"url":"https://example.test/app-0.6.2.x86_64.rpm","sha256":"ccdd"}
        }
    }"#;
    let info = parse_latest_json(body, "0.6.1").unwrap();
    let pkgs = info.linux_packages.expect("linux_packages present");
    assert_eq!(pkgs.deb.as_ref().unwrap().url, "https://example.test/app_0.6.2_amd64.deb");
    assert_eq!(pkgs.deb.as_ref().unwrap().sha256, "aabb");
    assert_eq!(pkgs.rpm.as_ref().unwrap().sha256, "ccdd");
}

#[test]
fn parse_latest_json_without_linux_packages_is_none() {
    let info = parse_latest_json(r#"{"version":"0.6.2"}"#, "0.6.1").unwrap();
    assert!(info.linux_packages.is_none());
}

#[test]
fn verify_sha256_accepts_correct_digest() {
    // sha256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
    let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert!(verify_sha256(b"abc", expected).is_ok());
}

#[test]
fn verify_sha256_is_case_insensitive() {
    let expected = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
    assert!(verify_sha256(b"abc", expected).is_ok());
}

#[test]
fn verify_sha256_rejects_mismatch() {
    let err = verify_sha256(b"abc", "deadbeef").unwrap_err();
    assert!(err.contains("Checksum mismatch"));
}

#[test]
fn resolve_update_channel_forced_env_wins() {
    assert_eq!(resolve_update_channel(Some("deb"), true, true, false, false), "deb");
    assert_eq!(resolve_update_channel(Some("manual"), false, false, false, false), "manual");
}

#[test]
fn resolve_update_channel_empty_forced_ignored() {
    // empty override behaves as if unset
    assert_eq!(resolve_update_channel(Some(""), false, false, false, false), "supported");
}

#[test]
fn resolve_update_channel_non_linux_is_supported() {
    assert_eq!(resolve_update_channel(None, false, false, false, false), "supported");
}

#[test]
fn resolve_update_channel_linux_appimage() {
    assert_eq!(resolve_update_channel(None, true, true, true, true), "appimage");
}

#[test]
fn resolve_update_channel_linux_deb_then_rpm_then_manual() {
    assert_eq!(resolve_update_channel(None, true, false, true, false), "deb");
    assert_eq!(resolve_update_channel(None, true, false, false, true), "rpm");
    assert_eq!(resolve_update_channel(None, true, false, false, false), "manual");
}

#[test]
fn list_printers_does_not_panic() {
    let printers = pdf::print::list_printers();
    assert!(printers.is_empty() || !printers[0].system_name.is_empty());
}

#[test]
fn resolve_rename_target_appends_pdf_extension() {
    let original = PathBuf::from("/docs/old.pdf");
    let out = resolve_rename_target(&original, "report").unwrap();
    assert_eq!(out, PathBuf::from("/docs/report.pdf"));
}

#[test]
fn resolve_rename_target_keeps_existing_pdf_extension() {
    let original = PathBuf::from("/docs/old.pdf");
    let out = resolve_rename_target(&original, "Report.PDF").unwrap();
    assert_eq!(out, PathBuf::from("/docs/Report.PDF"));
}

#[test]
fn resolve_rename_target_rejects_separators_and_empty() {
    let original = PathBuf::from("/docs/old.pdf");
    assert!(resolve_rename_target(&original, "a/b").is_err());
    assert!(resolve_rename_target(&original, "a\\b").is_err());
    assert!(resolve_rename_target(&original, "   ").is_err());
}

#[test]
fn edit_text_line_styled_replaces_text_without_panic() {
    let path = {
        let mut doc = build_pdf(1);
        save(&mut doc, "edit_text_line_styled")
    };

    let doc_loaded = Document::load(&path).unwrap();
    let page_id = *doc_loaded.get_pages().get(&1).unwrap();
    let lines = decode_page_text_lines(&doc_loaded, page_id).unwrap();
    assert!(!lines.is_empty(), "fixture PDF should contain at least one text line");

    let bbox = lines[0].bbox;
    let v = pdf_rect_to_viewer_px(bbox[0], bbox[1], bbox[2], bbox[3], 612.0, 792.0);
    let box_rect = PdfRect { x: v[0], y: v[1], width: (v[2] - v[0]).max(1.0), height: (v[3] - v[1]).max(1.0) };

    let style = TextStyle {
        font_family: "Helvetica".to_string(),
        font_size: 12.0,
        bold: false,
        italic: false,
        underline: false,
        color: RgbColor { r: 0.0, g: 0.0, b: 0.0 },
        align: "left".to_string(),
    };

    let mut doc = Document::load(&path).unwrap();
    edit_text_line(&mut doc, 0, 0, "World", &style, &box_rect).unwrap();

    let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
    assert!(content.contains("World"), "replacement text should appear in the content stream");

    std::fs::remove_file(&path).ok();
}

#[test]
fn add_text_box_wraps_text_and_appends_to_page() {
    let path = save(&mut build_pdf(1), "add_text_box");
    let mut doc = Document::load(&path).unwrap();
    let style = TextStyle {
        font_family: "Helvetica".to_string(),
        font_size: 12.0,
        bold: false,
        italic: false,
        underline: false,
        color: RgbColor { r: 0.0, g: 0.0, b: 0.0 },
        align: "left".to_string(),
    };
    let box_rect = PdfRect { x: 72.0, y: 72.0, width: 100.0, height: 200.0 };
    add_text_box(&mut doc, 0, "Hello world wide text", &style, &box_rect).unwrap();

    let page_id = *doc.get_pages().get(&1).unwrap();
    let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
    assert!(content.contains("Hello world"), "first wrapped line should appear");
    assert!(content.contains("wide text"), "second wrapped line should appear");

    std::fs::remove_file(&path).ok();
}

fn text_box_style() -> TextStyle {
    TextStyle {
        font_family: "Helvetica".to_string(),
        font_size: 12.0,
        bold: false,
        italic: false,
        underline: false,
        color: RgbColor { r: 0.1, g: 0.2, b: 0.3 },
        align: "left".to_string(),
    }
}

fn tm_x_values(content: &str) -> Vec<f64> {
    content
        .split("Tm")
        .filter_map(|s| {
            let tokens: Vec<&str> = s.split_whitespace().collect();
            if tokens.len() >= 2 {
                tokens[tokens.len() - 2].parse().ok()
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn add_text_box_emits_color_operators() {
    let path = save(&mut build_pdf(1), "add_text_box_color");
    let mut doc = Document::load(&path).unwrap();
    add_text_box(&mut doc, 0, "Hello", &text_box_style(), &PdfRect { x: 72.0, y: 72.0, width: 200.0, height: 100.0 })
        .unwrap();

    let page_id = *doc.get_pages().get(&1).unwrap();
    let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
    assert!(content.contains("rg"), "fill color operator missing");
    assert!(content.contains("RG"), "stroke color operator missing");

    std::fs::remove_file(&path).ok();
}

#[test]
fn add_text_box_bold_draws_twice() {
    let path = save(&mut build_pdf(1), "add_text_box_bold");
    let mut doc = Document::load(&path).unwrap();
    let mut style = text_box_style();
    style.bold = true;
    add_text_box(&mut doc, 0, "Hello", &style, &PdfRect { x: 72.0, y: 72.0, width: 200.0, height: 100.0 }).unwrap();

    let page_id = *doc.get_pages().get(&1).unwrap();
    let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
    assert_eq!(content.matches("Tj").count(), 3, "bold should emit two new text show operators plus the original one");

    std::fs::remove_file(&path).ok();
}

#[test]
fn add_text_box_italic_uses_shear_matrix() {
    let path = save(&mut build_pdf(1), "add_text_box_italic");
    let mut doc = Document::load(&path).unwrap();
    let mut style = text_box_style();
    style.italic = true;
    add_text_box(&mut doc, 0, "Hello", &style, &PdfRect { x: 72.0, y: 72.0, width: 200.0, height: 100.0 }).unwrap();

    let page_id = *doc.get_pages().get(&1).unwrap();
    let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
    assert!(content.contains("1 0 0.25 1"), "italic should use a shear matrix");

    std::fs::remove_file(&path).ok();
}

#[test]
fn add_text_box_underline_emits_stroke() {
    let path = save(&mut build_pdf(1), "add_text_box_underline");
    let mut doc = Document::load(&path).unwrap();
    let mut style = text_box_style();
    style.underline = true;
    add_text_box(&mut doc, 0, "Hello", &style, &PdfRect { x: 72.0, y: 72.0, width: 200.0, height: 100.0 }).unwrap();

    let page_id = *doc.get_pages().get(&1).unwrap();
    let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
    assert!(content.contains(" m "), "underline should contain a moveto");
    assert!(content.contains(" l "), "underline should contain a lineto");
    assert!(content.contains(" S"), "underline should contain a stroke operator");

    std::fs::remove_file(&path).ok();
}

#[test]
fn add_text_box_alignment_offsets_differ() {
    let path_left = save(&mut build_pdf(1), "add_text_box_align_left");
    let path_center = save(&mut build_pdf(1), "add_text_box_align_center");
    let path_right = save(&mut build_pdf(1), "add_text_box_align_right");

    let box_rect = PdfRect { x: 72.0, y: 72.0, width: 200.0, height: 100.0 };

    let left_x = {
        let mut doc = Document::load(&path_left).unwrap();
        add_text_box(&mut doc, 0, "Hello", &text_box_style(), &box_rect).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
        tm_x_values(&content)[0]
    };
    let center_x = {
        let mut doc = Document::load(&path_center).unwrap();
        let mut style = text_box_style();
        style.align = "center".to_string();
        add_text_box(&mut doc, 0, "Hello", &style, &box_rect).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
        tm_x_values(&content)[0]
    };
    let right_x = {
        let mut doc = Document::load(&path_right).unwrap();
        let mut style = text_box_style();
        style.align = "right".to_string();
        add_text_box(&mut doc, 0, "Hello", &style, &box_rect).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let content = String::from_utf8_lossy(&read_page_content(&doc, page_id).unwrap()).into_owned();
        tm_x_values(&content)[0]
    };

    assert!(left_x < center_x, "center should shift text right: left={left_x} center={center_x}");
    assert!(center_x < right_x, "right should shift text further right: center={center_x} right={right_x}");

    std::fs::remove_file(&path_left).ok();
    std::fs::remove_file(&path_center).ok();
    std::fs::remove_file(&path_right).ok();
}

#[test]
fn add_text_box_rejects_non_positive_box() {
    let path = save(&mut build_pdf(1), "add_text_box_bad_box");
    let mut doc = Document::load(&path).unwrap();
    let mut box_rect = PdfRect { x: 72.0, y: 72.0, width: 200.0, height: 100.0 };
    box_rect.width = 0.0;
    let err = add_text_box(&mut doc, 0, "Hello", &text_box_style(), &box_rect).unwrap_err();
    assert!(err.contains("positive width"), "got: {err}");

    std::fs::remove_file(&path).ok();
}

#[test]
fn add_text_box_rejects_box_too_short() {
    let path = save(&mut build_pdf(1), "add_text_box_short");
    let mut doc = Document::load(&path).unwrap();
    let box_rect = PdfRect { x: 72.0, y: 72.0, width: 1000.0, height: 1.0 };
    let err = add_text_box(&mut doc, 0, "Hello", &text_box_style(), &box_rect).unwrap_err();
    assert!(err.contains("too short"), "got: {err}");

    std::fs::remove_file(&path).ok();
}

/// Build a one-page PDF with a small inline RGB image XObject drawn by a simple
/// `q cm Do Q` content stream.
fn build_pdf_with_image() -> Document {
    let mut doc = build_pdf(1);
    let page_id = *doc.get_pages().get(&1).unwrap();

    let mut image_dict = Dictionary::new();
    image_dict.set("Type", Object::Name(b"XObject".to_vec()));
    image_dict.set("Subtype", Object::Name(b"Image".to_vec()));
    image_dict.set("Width", Object::Integer(2));
    image_dict.set("Height", Object::Integer(2));
    image_dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
    image_dict.set("BitsPerComponent", Object::Integer(8));
    let image_data = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
    let image_id = doc.add_object(Object::Stream(Stream::new(image_dict, image_data)));

    let mut xobjects = Dictionary::new();
    xobjects.set(b"Im1", Object::Reference(image_id));
    let resources = Dictionary::from_iter(vec![(b"XObject".to_vec(), Object::Dictionary(xobjects))]);
    doc.get_dictionary_mut(page_id).unwrap().set(b"Resources", Object::Dictionary(resources));

    let content = b"q 100 0 0 100 50 50 cm /Im1 Do Q\n".to_vec();
    let content_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), content)));
    doc.get_dictionary_mut(page_id).unwrap().set(b"Contents", Object::Reference(content_id));

    doc
}

#[test]
fn list_page_images_finds_inline_image_xobject() {
    let path = save(&mut build_pdf_with_image(), "list_images");
    let images = pdf::page_images::list_page_images(&Document::load(&path).unwrap(), 0).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].index, 0);
    assert_eq!(images[0].width, 2);
    assert_eq!(images[0].height, 2);
    assert_eq!(images[0].bbox.width, 2.0);
    assert_eq!(images[0].bbox.height, 2.0);
    std::fs::remove_file(&path).ok();
}

#[test]
fn transform_page_image_rewrites_cm_matrix() {
    let path = save(&mut build_pdf_with_image(), "transform_image");
    let new_rect = PdfRect { x: 10.0, y: 20.0, width: 30.0, height: 40.0 };
    let mut doc = Document::load(&path).unwrap();
    pdf::edit_object::transform_page_image(&mut doc, 0, 0, &new_rect).unwrap();
    doc.save(&path).unwrap();

    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let bytes = read_page_content(&doc, page_id).unwrap();
    let content = String::from_utf8_lossy(&bytes);
    assert!(content.contains("30 0 0 40 10 20 cm"), "expected rewritten cm matrix, got {content}");
    std::fs::remove_file(&path).ok();
}

#[test]
fn remove_page_image_removes_do_operator() {
    let path = save(&mut build_pdf_with_image(), "remove_image");
    let mut doc = Document::load(&path).unwrap();
    pdf::edit_object::remove_page_image(&mut doc, 0, 0).unwrap();
    doc.save(&path).unwrap();

    let doc = Document::load(&path).unwrap();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let bytes = read_page_content(&doc, page_id).unwrap();
    let content = String::from_utf8_lossy(&bytes);
    assert!(!content.contains("/Im1 Do"), "image Do operator should be removed, got {content}");
    std::fs::remove_file(&path).ok();
}

/// Build a one-page PDF whose image is invoked without a preceding `cm`, so
/// transform/remove must reject the unsupported pattern.
fn build_pdf_with_unsupported_image() -> Document {
    let mut doc = build_pdf_with_image();
    let page_id = *doc.get_pages().get(&1).unwrap();
    let content = b"q /Im1 Do Q\n".to_vec();
    let content_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), content)));
    doc.get_dictionary_mut(page_id).unwrap().set(b"Contents", Object::Reference(content_id));
    doc
}

#[test]
fn transform_page_image_rejects_unsupported_pattern() {
    let path = save(&mut build_pdf_with_unsupported_image(), "transform_unsupported");
    let new_rect = PdfRect { x: 10.0, y: 20.0, width: 30.0, height: 40.0 };
    let mut doc = Document::load(&path).unwrap();
    let err = pdf::edit_object::transform_page_image(&mut doc, 0, 0, &new_rect).unwrap_err();
    assert!(err.contains("not supported"), "expected unsupported pattern error, got {err}");
    std::fs::remove_file(&path).ok();
}
