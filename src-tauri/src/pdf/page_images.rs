use crate::pdf::content::{append_page_content, embed_jpeg_xobject, next_image_xobject_name};
use crate::pdf::coords::viewer_rect_to_pdf;
use crate::pdf::edit_types::{PageImageInfo, PdfRect};
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

/// List each image XObject placement in page drawing order.
pub fn list_page_images(doc: &Document, page_index: u32) -> Result<Vec<PageImageInfo>, String> {
    let page_id = doc.page_iter().nth(page_index as usize).ok_or_else(|| "page index out of range".to_string())?;
    let resources = page_resources(doc, page_id)?;
    let xobjects = resources.get(b"XObject").map_err(|_| "missing XObject resources".to_string())?;
    let xobjects = object_dictionary(doc, xobjects).ok_or_else(|| "XObject not dict".to_string())?;
    let mut image_resources = std::collections::BTreeMap::new();
    for (name, obj) in xobjects.iter() {
        let id = obj.as_reference().map_err(|_| "xobject not reference".to_string())?;
        let xobj =
            doc.get_object(id).map_err(|e| e.to_string())?.as_stream().map_err(|_| "xobject not stream".to_string())?;
        if xobj.dict.get(b"Subtype").ok().and_then(|s| s.as_name().ok()) != Some(b"Image".as_slice()) {
            continue;
        }
        let width = xobj.dict.get(b"Width").and_then(|w| w.as_i64()).unwrap_or(0) as u32;
        let height = xobj.dict.get(b"Height").and_then(|h| h.as_i64()).unwrap_or(0) as u32;
        image_resources.insert(name.clone(), (id, width, height));
    }

    let bytes = doc.get_page_content(page_id).map_err(|e| e.to_string())?;
    let content = lopdf::content::Content::decode(&bytes).map_err(|e| e.to_string())?;
    let mut ctm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut stack = Vec::new();
    let mut occurrence_by_name = std::collections::BTreeMap::<Vec<u8>, usize>::new();
    let mut images = Vec::new();
    for op in content.operations {
        match op.operator.as_str() {
            "q" => stack.push(ctm),
            "Q" => ctm = stack.pop().unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
            "cm" => {
                let matrix: Vec<f64> =
                    op.operands.iter().filter_map(|value| value.as_float().ok().map(f64::from)).collect();
                if let [a, b, c, d, e, f] = matrix.as_slice() {
                    ctm = multiply_matrices(ctm, [*a, *b, *c, *d, *e, *f]);
                }
            }
            "Do" => {
                let Some(name) = op.operands.first().and_then(|value| value.as_name().ok()) else {
                    continue;
                };
                let Some(&(id, width, height)) = image_resources.get(name) else {
                    continue;
                };
                let occurrence = occurrence_by_name.entry(name.to_vec()).or_default();
                let [(x0, y0), (x1, y1), (x2, y2), (x3, y3)] = [
                    transform_point(ctm, 0.0, 0.0),
                    transform_point(ctm, 1.0, 0.0),
                    transform_point(ctm, 0.0, 1.0),
                    transform_point(ctm, 1.0, 1.0),
                ];
                let left = x0.min(x1).min(x2).min(x3);
                let right = x0.max(x1).max(x2).max(x3);
                let bottom = y0.min(y1).min(y2).min(y3);
                let top = y0.max(y1).max(y2).max(y3);
                let placement_width = ctm[0].hypot(ctm[1]);
                let placement_height = ctm[2].hypot(ctm[3]);
                let center = transform_point(ctm, 0.5, 0.5);
                images.push(PageImageInfo {
                    index: images.len(),
                    object_id: (id.0, id.1),
                    resource_name: String::from_utf8_lossy(name).into_owned(),
                    occurrence: *occurrence,
                    bbox: PdfRect { x: left, y: bottom, width: right - left, height: top - bottom },
                    rect: PdfRect {
                        x: center.0 - placement_width / 2.0,
                        y: center.1 - placement_height / 2.0,
                        width: placement_width,
                        height: placement_height,
                    },
                    rotation: ctm[1].atan2(ctm[0]).to_degrees(),
                    width,
                    height,
                });
                *occurrence += 1;
            }
            _ => {}
        }
    }

    Ok(images)
}

fn multiply_matrices(left: [f64; 6], right: [f64; 6]) -> [f64; 6] {
    [
        left[0] * right[0] + left[2] * right[1],
        left[1] * right[0] + left[3] * right[1],
        left[0] * right[2] + left[2] * right[3],
        left[1] * right[2] + left[3] * right[3],
        left[0] * right[4] + left[2] * right[5] + left[4],
        left[1] * right[4] + left[3] * right[5] + left[5],
    ]
}

fn transform_point(matrix: [f64; 6], x: f64, y: f64) -> (f64, f64) {
    (matrix[0] * x + matrix[2] * y + matrix[4], matrix[1] * x + matrix[3] * y + matrix[5])
}

fn object_dictionary(doc: &Document, object: &Object) -> Option<Dictionary> {
    match object {
        Object::Dictionary(dict) => Some(dict.clone()),
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        _ => None,
    }
}

/// Resolve page resources, including inherited and indirect dictionaries.
pub fn page_resources(doc: &Document, page_id: lopdf::ObjectId) -> Result<lopdf::Dictionary, String> {
    let mut current = page_id;
    for _ in 0..64 {
        let dict = doc.get_dictionary(current).map_err(|e| e.to_string())?;
        if let Ok(resources) = dict.get(b"Resources") {
            return object_dictionary(doc, resources).ok_or_else(|| "resources not dict".to_string());
        }
        let Some(parent) = dict.get(b"Parent").ok().and_then(|object| object.as_reference().ok()) else {
            break;
        };
        current = parent;
    }
    Err("missing resources".to_string())
}
