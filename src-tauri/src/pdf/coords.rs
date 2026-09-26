use lopdf::{Document, Object, ObjectId};

/// Max viewer bitmap box. Pages are aspect-fitted inside this so landscape (and
/// other non-A4) pages are not stretched. A4 portrait still lands on 1600×2264.
/// Keep aligned with `VIEWER_PAGE_W` / `VIEWER_PAGE_H` in the frontend.
pub const VIEWER_PAGE_W: f64 = 1600.0;
pub const VIEWER_PAGE_H: f64 = 2264.0;

/// Aspect-fit `dw`×`dh` (already post-rotation display size) into the viewer max box.
pub fn viewer_size_for_dims(dw: f64, dh: f64) -> (f64, f64) {
    if !(dw > 0.0 && dh > 0.0) {
        return (VIEWER_PAGE_W, VIEWER_PAGE_H);
    }
    let scale = (VIEWER_PAGE_W / dw).min(VIEWER_PAGE_H / dh);
    ((dw * scale).round().max(1.0), (dh * scale).round().max(1.0))
}

/// Aspect-fit viewer size for an unrotated MediaBox plus `/Rotate`.
pub fn viewer_size_for_media(mw: f64, mh: f64, rotation: i64) -> (f64, f64) {
    let (dw, dh) = match rotation.rem_euclid(360) {
        90 | 270 => (mh, mw),
        _ => (mw, mh),
    };
    viewer_size_for_dims(dw, dh)
}

/// Reject NaN/±Infinity on a coordinate-sized input. Returns the value
/// unchanged when finite so call sites stay trivial. Used at every public
/// Rust entry point that accepts a frontend `f64` and casts to `f32` for
/// the PDF dictionary — NaN/Inf written into CropBox/MediaBox/Rect produces
/// PDFs that most renderers refuse to open.
pub fn finite_f64(value: f64, name: &str) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{name} must be a finite number"))
    }
}

/// Map PDF-point bounds to raster pixel rect `(x, y, w, h)` with top-left origin.
pub fn pdf_rect_to_render_px(
    rect: [f64; 4],
    page_w: f32,
    page_h: f32,
    render_w: f64,
    render_h: f64,
) -> (i32, i32, i32, i32) {
    let viewer = pdf_rect_to_viewer_px(rect[0], rect[1], rect[2], rect[3], page_w, page_h);
    let (vw, vh) = viewer_size_for_dims(f64::from(page_w).max(1.0), f64::from(page_h).max(1.0));
    let sx = render_w / vw;
    let sy = render_h / vh;
    let x = (viewer[0] * sx).round() as i32;
    let y = (viewer[1] * sy).round() as i32;
    let w = ((viewer[2] - viewer[0]) * sx).round().max(1.0) as i32;
    let h = ((viewer[3] - viewer[1]) * sy).round().max(1.0) as i32;
    (x, y, w, h)
}

/// Map PDF-point bounds to viewer pixel rect `[left, top, right, bottom]`.
pub fn pdf_rect_to_viewer_px(left: f64, bottom: f64, right: f64, top: f64, page_w: f32, page_h: f32) -> [f64; 4] {
    let pw = f64::from(page_w).max(1.0);
    let ph = f64::from(page_h).max(1.0);
    let (sw, sh) = viewer_size_for_dims(pw, ph);
    let left_px = left / pw * sw;
    let right_px = right / pw * sw;
    let top_px = (ph - top) / ph * sh;
    let bottom_px = (ph - bottom) / ph * sh;
    [left_px, top_px, right_px, bottom_px]
}

/// Coerce a PDF numeric object to f64.
pub fn obj_to_f64(o: &Object) -> f64 {
    match o {
        Object::Real(r) => *r as f64,
        Object::Integer(i) => *i as f64,
        _ => 0.0,
    }
}

pub fn page_media_box(doc: &Document, page_id: ObjectId) -> Result<[f64; 4], String> {
    let page = doc.get_dictionary(page_id).map_err(|e| e.to_string())?;
    let media = page
        .get(b"MediaBox")
        .ok()
        .cloned()
        .or_else(|| crate::pdf::page_tree::inherited_page_attr(doc, page_id, b"MediaBox"))
        .ok_or_else(|| "Missing MediaBox".to_string())?;
    let arr = match &media {
        Object::Reference(id) => {
            doc.get_object(*id).map_err(|e| e.to_string())?.as_array().map_err(|_| "Bad MediaBox")?
        }
        _ => media.as_array().map_err(|_| "Bad MediaBox")?,
    };
    let get = |i: usize| arr.get(i).map(obj_to_f64).unwrap_or(0.0);
    Ok([get(0), get(1), get(2), get(3)])
}

pub fn viewer_rect_to_pdf(
    doc: &Document,
    page_id: ObjectId,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(f64, f64, f64, f64), String> {
    let media = page_media_box(doc, page_id)?;
    let mw = media[2] - media[0];
    let mh = media[3] - media[1];
    if mw <= 0.0 || mh <= 0.0 || w <= 0.0 || h <= 0.0 {
        return Err("Invalid page or image size".to_string());
    }
    let rotation = crate::pdf::rotation::page_rotation(doc, page_id);
    let (vw, vh) = viewer_size_for_media(mw, mh, rotation);
    let px = x * mw / vw;
    let pw = w * mw / vw;
    let ph = h * mh / vh;
    let py = mh - (y * mh / vh) - ph;
    Ok((px, py, pw, ph))
}

/// Map a viewer-space point (top-left origin) to a PDF user-space point
/// (bottom-left origin), honouring `/Rotate` (clockwise 0/90/180/270).
/// `mw`/`mh` are the unrotated MediaBox dimensions. Viewer size is the
/// aspect-fitted bitmap for that page, so landscape pages are not stretched.
///
/// The viewer bitmap is the page rendered with `/Rotate` applied, so on a
/// 90/270 page the viewer axes map to the swapped MediaBox axes. Only the
/// location is rotated; callers still draw glyphs in page space, so text
/// continues to rotate with the page as before.
pub fn viewer_point_to_pdf_with_rotation(mw: f64, mh: f64, vx: f64, vy: f64, rotation: i64) -> (f64, f64) {
    let (vw, vh) = viewer_size_for_media(mw, mh, rotation);
    match rotation.rem_euclid(360) {
        90 => (vy * mw / vh, vx * mh / vw),
        180 => (mw - vx * mw / vw, vy * mh / vh),
        270 => (mw - vy * mw / vh, mh - vx * mh / vw),
        _ => (vx * mw / vw, mh - vy * mh / vh),
    }
}

pub fn viewer_point_to_pdf_on_page(media: [f64; 4], vx: f64, vy: f64, rotation: i64) -> Result<(f64, f64), String> {
    let mw = media[2] - media[0];
    let mh = media[3] - media[1];
    if mw <= 0.0 || mh <= 0.0 {
        return Err("Invalid page size".to_string());
    }
    let (x, y) = viewer_point_to_pdf_with_rotation(mw, mh, vx, vy, rotation);
    Ok((x + media[0], y + media[1]))
}

pub fn pdf_point_to_viewer_on_page(media: [f64; 4], px: f64, py: f64, rotation: i64) -> Result<(f64, f64), String> {
    let mw = media[2] - media[0];
    let mh = media[3] - media[1];
    if mw <= 0.0 || mh <= 0.0 {
        return Err("Invalid page size".to_string());
    }
    let (vw, vh) = viewer_size_for_media(mw, mh, rotation);
    let x = px - media[0];
    let y = py - media[1];
    let point = match rotation.rem_euclid(360) {
        90 => (y * vw / mh, x * vh / mw),
        180 => ((mw - x) * vw / mw, y * vh / mh),
        270 => ((mh - y) * vw / mh, (mw - x) * vh / mw),
        _ => (x * vw / mw, (mh - y) * vh / mh),
    };
    Ok(point)
}

fn normalized_bounds(points: [(f64, f64); 4]) -> [f64; 4] {
    let xs = [points[0].0, points[1].0, points[2].0, points[3].0];
    let ys = [points[0].1, points[1].1, points[2].1, points[3].1];
    [
        xs.into_iter().fold(f64::INFINITY, f64::min),
        ys.into_iter().fold(f64::INFINITY, f64::min),
        xs.into_iter().fold(f64::NEG_INFINITY, f64::max),
        ys.into_iter().fold(f64::NEG_INFINITY, f64::max),
    ]
}

pub fn viewer_bounds_to_pdf_on_page(media: [f64; 4], bounds: [f64; 4], rotation: i64) -> Result<[f64; 4], String> {
    let [left, top, right, bottom] = bounds;
    Ok(normalized_bounds([
        viewer_point_to_pdf_on_page(media, left, top, rotation)?,
        viewer_point_to_pdf_on_page(media, right, top, rotation)?,
        viewer_point_to_pdf_on_page(media, left, bottom, rotation)?,
        viewer_point_to_pdf_on_page(media, right, bottom, rotation)?,
    ]))
}

pub fn pdf_bounds_to_viewer_on_page(media: [f64; 4], bounds: [f64; 4], rotation: i64) -> Result<[f64; 4], String> {
    let [left, bottom, right, top] = bounds;
    let pdf_corners = [(left, bottom), (right, bottom), (left, top), (right, top)];
    let mut viewer_corners = [(0.0, 0.0); 4];
    for (index, (x, y)) in pdf_corners.into_iter().enumerate() {
        viewer_corners[index] = pdf_point_to_viewer_on_page(media, x, y, rotation)?;
    }
    Ok(normalized_bounds(viewer_corners))
}
