use pdfium_render::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use lopdf::Document;

const CACHE_CAPACITY: usize = 8;

/// Maximum width or height (in pixels) accepted for any render command.
/// 8× the base viewer size (800×1132) is plenty for any sane preview; PDFium
/// happily allocates multi-GB bitmaps on i32::MAX, which is reachable from a
/// hostile JS frontend. Mirrors the cap in render_pdf_page_with_all_layers_visible.
pub(crate) const MAX_RENDER_DIM: i32 = 800 * 8;

struct CacheEntry {
    document: Document,
    mtime: Option<SystemTime>,
    size: Option<u64>,
    generation: u64,
}

struct DocumentCache {
    entries: HashMap<PathBuf, CacheEntry>,
    order: Vec<PathBuf>,
    generations: HashMap<PathBuf, u64>,
    capacity: usize,
}

impl DocumentCache {
    fn new(capacity: usize) -> Self {
        Self { entries: HashMap::new(), order: Vec::with_capacity(capacity), generations: HashMap::new(), capacity }
    }

    fn generation(&self, path: &Path) -> u64 {
        self.generations.get(path).copied().unwrap_or(0)
    }

    fn touch(&mut self, path: &Path) {
        self.order.retain(|p| p != path);
        self.order.insert(0, path.to_path_buf());
    }

    #[cfg(test)]
    fn remove(&mut self, path: &Path) {
        self.entries.remove(path);
        self.order.retain(|p| p != path);
        self.generations.remove(path);
    }

    fn invalidate(&mut self, path: &Path) {
        let next = self.generation(path).wrapping_add(1);
        self.entries.remove(path);
        self.order.retain(|p| p != path);
        self.generations.insert(path.to_path_buf(), next);
    }

    #[cfg(test)]
    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.generations.clear();
    }

    fn load_and_insert(&mut self, path: &Path) -> Result<Document, String> {
        let meta = std::fs::metadata(path).ok();
        let current_mtime = meta.as_ref().and_then(|m| m.modified().ok());
        let current_size = meta.as_ref().map(|m| m.len());
        let current_gen = self.generation(path);

        let document = Document::load(path).map_err(|e| e.to_string())?;
        if document.is_encrypted() {
            // Encrypted documents are never cached because callers currently do
            // not pass decryption passwords and serialising an encrypted lopdf
            // document back to bytes would not help pdfium-render.
            return Ok(document);
        }

        if self.entries.len() >= self.capacity && !self.entries.contains_key(path) {
            if let Some(oldest) = self.order.pop() {
                self.entries.remove(&oldest);
                self.generations.remove(&oldest);
            }
        }

        self.entries.insert(
            path.to_path_buf(),
            CacheEntry {
                document: document.clone(),
                mtime: current_mtime,
                size: current_size,
                generation: current_gen,
            },
        );
        self.touch(path);
        Ok(document)
    }

    fn get(&mut self, path: &Path) -> Result<Document, String> {
        let meta = std::fs::metadata(path).ok();
        let current_mtime = meta.as_ref().and_then(|m| m.modified().ok());
        let current_size = meta.as_ref().map(|m| m.len());
        let current_gen = self.generation(path);

        if let Some(entry) = self.entries.get(path) {
            // Validate on size too: a save that lands in the same mtime tick
            // (coarse-resolution filesystems) almost always changes the byte
            // length, so this catches stale reads the mtime check alone misses.
            if entry.mtime == current_mtime && entry.size == current_size && entry.generation == current_gen {
                let document = entry.document.clone();
                self.touch(path);
                return Ok(document);
            }
        }

        self.load_and_insert(path)
    }
}

fn document_cache() -> std::sync::MutexGuard<'static, DocumentCache> {
    static CACHE: OnceLock<Mutex<DocumentCache>> = OnceLock::new();
    CACHE
        .get_or_init(|| Mutex::new(DocumentCache::new(CACHE_CAPACITY)))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Return a fresh clone of the cached `lopdf::Document` for `path`, reloading
/// from disk when the file changes or when the entry is explicitly invalidated.
pub fn cached_document(path: &Path) -> Result<Document, String> {
    document_cache().get(path)
}

/// Mark any cached document for `path` as stale. Callers that mutate and save a
/// PDF should invalidate the cache so subsequent reads see the new contents
/// immediately, even when filesystem mtime granularity would otherwise hide the
/// change.
pub fn invalidate_document_cache(path: &Path) {
    document_cache().invalidate(path);
}

/// Load a pdfium-render document for `path` directly from disk.
///
/// Read-only rendering does not use the `lopdf` cache: cloning and serialising a
/// large `lopdf::Document` for every page render/thumbnail was the main source
/// of the reported memory churn and gradual slowdown. The lopdf cache is kept
/// for mutating callers that need a `Document` without re-parsing.
fn pdfium_document_for_path<'a>(
    pdfium: &'a Pdfium,
    path: &Path,
    password: Option<&str>,
) -> Result<PdfDocument<'a>, String> {
    pdfium.load_pdf_from_file(path, password).map_err(|e| e.to_string())
}

/// Render one PDF page to encoded image bytes at the given dimensions.
pub fn render_page_bytes(
    pdfium: &Pdfium,
    path: &Path,
    page_index: u32,
    width: i32,
    height: i32,
    format: image::ImageFormat,
) -> Result<Vec<u8>, String> {
    // ponytail: cap dimensions at the public boundary so a hostile JS caller
    // can't request width=i32::MAX and force PDFium to allocate a multi-GB
    // bitmap (OOM DoS). Mirrors the cap on render_pdf_page_with_all_layers_visible.
    if !(1..=MAX_RENDER_DIM).contains(&width) || !(1..=MAX_RENDER_DIM).contains(&height) {
        return Err(format!("Render dimensions must be 1..={MAX_RENDER_DIM}"));
    }
    let document = pdfium_document_for_path(pdfium, path, None)?;
    if page_index > i32::MAX as u32 {
        return Err("Page index out of range".to_string());
    }
    let page = document.pages().get(page_index as PdfPageIndex).map_err(|e| e.to_string())?;
    let bitmap = page.render(width as Pixels, height as Pixels, None).map_err(|e| e.to_string())?;
    let image = bitmap.as_image().map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    image.write_to(&mut std::io::Cursor::new(&mut buffer), format).map_err(|e| e.to_string())?;
    Ok(buffer)
}

/// Render every page of `path` to thumbnail PNG bytes at the requested size.
pub fn render_pdf_thumbnails(pdfium: &Pdfium, path: &Path, width: i32, height: i32) -> Result<Vec<Vec<u8>>, String> {
    let document = pdfium_document_for_path(pdfium, path, None)?;
    let page_count = document.pages().len();
    let mut thumbnails = Vec::with_capacity(page_count as usize);

    for i in 0..page_count {
        let page = document.pages().get(i as PdfPageIndex).map_err(|e| e.to_string())?;
        let bitmap = page.render(width as Pixels, height as Pixels, None).map_err(|e| e.to_string())?;
        let image = bitmap.as_image().map_err(|e| e.to_string())?;
        let mut buffer = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png).map_err(|e| e.to_string())?;
        thumbnails.push(buffer);
    }

    Ok(thumbnails)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Object, Stream};
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "kanoprii_render_test_{}_{}",
            std::process::id(),
            TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn minimal_pdf(path: &Path) {
        let _ = fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")));
        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();
        doc.objects.insert(content_id, Object::Stream(Stream::new(Dictionary::new(), b"BT ET".to_vec())));
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
        page.set("Contents", Object::Reference(content_id));
        doc.objects.insert(page_id, Object::Dictionary(page));
        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name(b"Pages".to_vec()));
        pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let catalog_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.save(path).unwrap();
    }

    #[test]
    fn cached_document_reuses_loaded_copy() {
        let dir = test_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("cached.pdf");
        minimal_pdf(&path);

        let first = cached_document(&path).unwrap();
        let second = cached_document(&path).unwrap();
        assert_eq!(first.get_pages().len(), second.get_pages().len());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cached_document_reloads_after_invalidation() {
        let dir = test_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("reload.pdf");
        minimal_pdf(&path);

        let before = cached_document(&path).unwrap();
        assert_eq!(before.get_pages().len(), 1);

        // Simulate a structural edit by replacing the file with a two-page PDF.
        let mut doc = Document::load(&path).unwrap();
        let pages_id = doc.catalog().unwrap().get(b"Pages").unwrap().as_reference().unwrap();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();
        doc.objects.insert(content_id, Object::Stream(Stream::new(Dictionary::new(), b"BT ET".to_vec())));
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set("MediaBox", Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
        page.set("Contents", Object::Reference(content_id));
        doc.objects.insert(page_id, Object::Dictionary(page));
        {
            let pages = doc.get_dictionary_mut(pages_id).unwrap();
            let kids = pages.get_mut(b"Kids").unwrap().as_array_mut().unwrap();
            kids.push(Object::Reference(page_id));
            let count = pages.get_mut(b"Count").unwrap().as_i64().unwrap();
            pages.set("Count", Object::Integer(count + 1));
        }
        doc.save(&path).unwrap();

        invalidate_document_cache(&path);
        let after = cached_document(&path).unwrap();
        assert_eq!(after.get_pages().len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_evicts_oldest_when_full() {
        document_cache().clear();

        let dir = test_dir();
        let _ = fs::create_dir_all(&dir);

        let mut paths = Vec::new();
        for i in 0..CACHE_CAPACITY + 2 {
            let path = dir.join(format!("evict_{i}.pdf"));
            minimal_pdf(&path);
            paths.push(path);
        }

        for path in &paths {
            let _ = cached_document(path).unwrap();
        }

        // The cache is shared across tests, so we only assert the invariant that
        // it never grows beyond its configured capacity and that the most recent
        // entries are still reachable.
        assert!(document_cache().entries.len() <= CACHE_CAPACITY, "cache grew beyond capacity");
        if let Some(last) = paths.last() {
            assert!(cached_document(last).is_ok(), "most recent entry should still be reachable");
        }
        let entries_len = document_cache().entries.len();
        assert!(
            document_cache().generations.len() <= entries_len,
            "evicted paths should also drop their generation records"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_remove_prunes_generation_record() {
        document_cache().clear();

        let dir = test_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("gen_prune.pdf");
        minimal_pdf(&path);

        let _ = cached_document(&path).unwrap();
        invalidate_document_cache(&path);
        assert!(document_cache().generations.contains_key(&path));

        document_cache().remove(&path);
        assert!(!document_cache().entries.contains_key(&path));
        assert!(!document_cache().generations.contains_key(&path));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_invalidate_keeps_bumped_generation() {
        document_cache().clear();

        let dir = test_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("gen_bump.pdf");
        minimal_pdf(&path);

        let _ = cached_document(&path).unwrap();
        let before = document_cache().generation(&path);
        invalidate_document_cache(&path);
        let after = document_cache().generation(&path);
        assert_eq!(after, before.wrapping_add(1));
        assert!(!document_cache().entries.contains_key(&path));

        let _ = fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod render_cap_tests {
    use super::*;

    #[test]
    fn render_page_bytes_rejects_zero_width() {
        // The cap check fires before document load, so a non-existent path
        // is fine and we don't need to construct a real PDFium instance.
        // render_page_bytes takes &Pdfium; we can't construct one without
        // PDFium init, so test the cap through the public command boundary
        // by exercising pdf::print's render path which calls render_page_bytes.
        // Easier: call pdf::render::render_page_bytes directly using a dummy.
        let pdfium = crate::pdf::pdfium_bind::get_pdfium().unwrap();
        let p = std::path::Path::new("/nonexistent.pdf");
        let res = render_page_bytes(&pdfium, p, 0, 0, 100, image::ImageFormat::Png);
        let err = match res {
            Ok(_) => panic!("expected Err"),
            Err(e) => e,
        };
        assert!(err.contains("Render dimensions must be"), "got: {err}");
    }

    #[test]
    fn render_page_bytes_rejects_oversize_dim() {
        let pdfium = crate::pdf::pdfium_bind::get_pdfium().unwrap();
        let p = std::path::Path::new("/nonexistent.pdf");
        let res = render_page_bytes(&pdfium, p, 0, i32::MAX, 100, image::ImageFormat::Png);
        let err = match res {
            Ok(_) => panic!("expected Err"),
            Err(e) => e,
        };
        assert!(err.contains("Render dimensions must be"), "got: {err}");
    }
}
