use lopdf::{Document, Object, ObjectId};

/// Viewer render size - must stay aligned with `BASE_W` / `BASE_H` in `App.tsx`.
pub const VIEWER_PAGE_W: f64 = 800.0;
pub const VIEWER_PAGE_H: f64 = 1132.0;

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
    let sx = render_w / VIEWER_PAGE_W;
    let sy = render_h / VIEWER_PAGE_H;
    let x = (viewer[0] * sx).round() as i32;
    let y = (viewer[1] * sy).round() as i32;
    let w = ((viewer[2] - viewer[0]) * sx).round().max(1.0) as i32;
    let h = ((viewer[3] - viewer[1]) * sy).round().max(1.0) as i32;
    (x, y, w, h)
}

/// Map PDF-point bounds to viewer pixel rect `[left, top, right, bottom]` at 800×1132.
pub fn pdf_rect_to_viewer_px(left: f64, bottom: f64, right: f64, top: f64, page_w: f32, page_h: f32) -> [f64; 4] {
    let sw = VIEWER_PAGE_W;
    let sh = VIEWER_PAGE_H;
    let pw = f64::from(page_w).max(1.0);
    let ph = f64::from(page_h).max(1.0);
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
    let arr = page.get(b"MediaBox").map_err(|e| e.to_string())?.as_array().map_err(|_| "Bad MediaBox")?;
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
    let px = x * mw / VIEWER_PAGE_W;
    let pw = w * mw / VIEWER_PAGE_W;
    let ph = h * mh / VIEWER_PAGE_H;
    let py = mh - (y * mh / VIEWER_PAGE_H) - ph;
    Ok((px, py, pw, ph))
}
