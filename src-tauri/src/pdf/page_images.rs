use crate::pdf::content::{append_page_content, embed_jpeg_xobject, next_image_xobject_name};
use crate::pdf::coords::viewer_rect_to_pdf;
use crate::pdf::merge_split::extract_pdf_pages;
use crate::pdf::page_tree::{flatten_pages, get_pages_kids, set_pages_kids};
use lopdf::{Dictionary, Document, Object, Stream};
use std::path::Path;

/// Build a PDF from a slice of image page buffers. JPEG inputs (0xFF 0xD8 magic)
/// are embedded directly via DCTDecode with header-only dimension reads, skipping
/// the decode/re-encode round-trip. Other inputs fall back to decode + JPEG re-encode.
pub fn create_pdf_from_image_pages(pages: &[Vec<u8>], output: &Path) -> Result<(), String> {
    if pages.is_empty() {
        return Err("Document produced no pages".to_string());
    }
    let mut doc = Document::with_version("1.4");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::with_capacity(pages.len());
    for page_bytes in pages {
        let (jpeg, width, height) = if page_bytes.starts_with(&[0xFF, 0xD8]) {
            let (w, h) = image::ImageReader::new(std::io::Cursor::new(page_bytes))
                .with_guessed_format()
                .map_err(|e| e.to_string())?
                .into_dimensions()
                .map_err(|e| e.to_string())?;
            (page_bytes.to_vec(), w, h)
        } else {
            let img = image::load_from_memory(page_bytes).map_err(|e| e.to_string())?.to_rgb8();
            let (width, height) = img.dimensions();
            let mut jpeg = Vec::new();
            image::DynamicImage::ImageRgb8(img)
                .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
                .map_err(|e| e.to_string())?;
            (jpeg, width, height)
        };
        let image_id = embed_jpeg_xobject(&mut doc, jpeg, width, height);
        let mut xobjects = Dictionary::new();
        xobjects.set(b"Im1", Object::Reference(image_id));
        let mut resources = Dictionary::new();
        resources.set(b"XObject", Object::Dictionary(xobjects));
        let content_id = doc
            .add_object(Object::Stream(Stream::new(Dictionary::new(), b"q 612 0 0 792 0 0 cm /Im1 Do Q\n".to_vec())));
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set("Resources", Object::Dictionary(resources));
        page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
        page.set("Contents", Object::Reference(content_id));
        kids.push(Object::Reference(doc.add_object(Object::Dictionary(page))));
    }
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Count", Object::Integer(kids.len() as i64));
    pages_dict.set("Kids", Object::Array(kids));
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));
    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer.set("Root", Object::Reference(catalog_id));
    crate::pdf::io::save_atomic(&mut doc, output)
}

pub fn insert_image_page(path: &Path, at_index: u32, image_path: &Path) -> Result<u32, String> {
    let image_path = image_path.to_path_buf();
    if !image_path.is_file() {
        return Err("Image file not found".to_string());
    }
    let img = image::open(&image_path).map_err(|e| e.to_string())?;
    let rgb = img.to_rgb8();
    let (img_w, img_h) = rgb.dimensions();
    if img_w == 0 || img_h == 0 {
        return Err("Image has no pixels".to_string());
    }
    let mut jpeg = Vec::new();
    image::DynamicImage::ImageRgb8(rgb)
        .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
        .map_err(|e| e.to_string())?;

    let path_buf = path.to_path_buf();
    let mut doc = Document::load(&path_buf).map_err(|e| e.to_string())?;
    let pages_ref = flatten_pages(&mut doc)?;
    let (mut kids, _) = get_pages_kids(&doc)?;
    let at = at_index as usize;
    if at > kids.len() {
        return Err("Insert index out of bounds".to_string());
    }

    const PAGE_W: f64 = 612.0;
    const PAGE_H: f64 = 792.0;
    let scale = (PAGE_W / img_w as f64).min(PAGE_H / img_h as f64);
    let draw_w = img_w as f64 * scale;
    let draw_h = img_h as f64 * scale;
    let offset_x = (PAGE_W - draw_w) / 2.0;
    let offset_y = (PAGE_H - draw_h) / 2.0;

    let image_id = embed_jpeg_xobject(&mut doc, jpeg, img_w, img_h);
    let mut xobjects = Dictionary::new();
    xobjects.set(b"Im1", Object::Reference(image_id));
    let mut resources = Dictionary::new();
    resources.set(b"XObject", Object::Dictionary(xobjects));

    let ops = format!("q {draw_w} 0 0 {draw_h} {offset_x} {offset_y} cm /Im1 Do Q\n");
    let content_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), ops.into_bytes())));

    let mut page = Dictionary::new();
    page.set("Type", Object::Name(b"Page".to_vec()));
    page.set("Parent", Object::Reference(pages_ref));
    page.set("Resources", Object::Dictionary(resources));
    page.set(
        "MediaBox",
        Object::Array(vec![Object::Integer(0), Object::Integer(0), Object::Integer(612), Object::Integer(792)]),
    );
    page.set("Contents", Object::Reference(content_id));
    let page_id = doc.add_object(Object::Dictionary(page));
    kids.insert(at, Object::Reference(page_id));
    set_pages_kids(&mut doc, pages_ref, kids)?;
    crate::pdf::io::save_atomic(&mut doc, &path_buf)?;
    Ok(at_index)
}

pub fn get_image_dimensions(path: &Path) -> Result<[u32; 2], String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    Ok([img.width(), img.height()])
}

pub fn add_page_image(
    path: &Path,
    page_index: u32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    image_path: &Path,
) -> Result<(), String> {
    if width < 5.0 || height < 5.0 {
        return Err("Image placement is too small".to_string());
    }

    let image_path = image_path.to_path_buf();
    if !image_path.is_file() {
        return Err("Image file not found".to_string());
    }

    let img = image::open(&image_path).map_err(|e| e.to_string())?;
    let rgb = img.to_rgb8();
    let (img_w, img_h) = rgb.dimensions();
    let mut jpeg = Vec::new();
    image::DynamicImage::ImageRgb8(rgb)
        .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
        .map_err(|e| e.to_string())?;

    let path = path.to_path_buf();
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&(page_index + 1)).ok_or("Page not found".to_string())?;

    let (px, py, pw, ph) = viewer_rect_to_pdf(&doc, page_id, x, y, width, height)?;
    let image_id = embed_jpeg_xobject(&mut doc, jpeg, img_w, img_h);

    if !matches!(doc.get_dictionary(page_id).map_err(|e| e.to_string())?.get(b"Resources"), Ok(Object::Dictionary(_))) {
        doc.get_dictionary_mut(page_id)
            .map_err(|e| e.to_string())?
            .set(b"Resources", Object::Dictionary(Dictionary::new()));
    }

    let xobject_name = {
        let page_dict = doc.get_dictionary_mut(page_id).map_err(|e| e.to_string())?;
        let resources = page_dict
            .get_mut(b"Resources")
            .map_err(|e| e.to_string())?
            .as_dict_mut()
            .map_err(|_| "Bad Resources".to_string())?;
        match resources.get_mut(b"XObject") {
            Ok(Object::Dictionary(dict)) => {
                let name = next_image_xobject_name(dict);
                dict.set(name.as_bytes(), Object::Reference(image_id));
                name
            }
            _ => {
                let mut dict = Dictionary::new();
                dict.set(b"Im1", Object::Reference(image_id));
                resources.set(b"XObject", Object::Dictionary(dict));
                "Im1".to_string()
            }
        }
    };

    let ops = format!("q {pw} 0 0 {ph} {px} {py} cm /{xobject_name} Do Q\n");
    append_page_content(&mut doc, page_id, ops.as_bytes())?;

    crate::pdf::io::save_atomic(&mut doc, &path)?;
    Ok(())
}

pub fn export_page_as_pdf(path: &Path, page_index: u32, output_path: &Path) -> Result<String, String> {
    extract_pdf_pages(path, output_path, page_index, page_index)
}
