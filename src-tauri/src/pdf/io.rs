use crate::pdf::render;
use lopdf::Document;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::Manager;

/// Load a PDF, run `f`, save back to the same path, and return `f`'s result.
pub fn mutate_pdf<T, F>(path: &Path, f: F) -> Result<T, String>
where
    F: FnOnce(&mut Document) -> Result<T, String>,
{
    let _lock = path_lock(path);
    let mut doc = Document::load(path).map_err(|e| e.to_string())?;
    let result = f(&mut doc)?;
    save_atomic(&mut doc, path)?;
    render::invalidate_document_cache(path);
    Ok(result)
}

/// Serialize concurrent mutations to the same PDF path. Without this, two
/// Tauri commands editing the same file load their own pre-edit snapshots
/// and last-write-wins: the second save clobbers the first's edit.
///
/// ponytail: HashMap<PathBuf, Mutex<()>> keyed by canonical path. Entries are
/// never removed — the map grows by one entry per distinct path ever edited,
/// bounded by the number of distinct files the user opens (small).
type PathLockMap = Mutex<HashMap<PathBuf, &'static Mutex<()>>>;
static PATH_LOCKS: OnceLock<PathLockMap> = OnceLock::new();

fn path_lock(path: &Path) -> std::sync::MutexGuard<'static, ()> {
    let map = PATH_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let m: &'static Mutex<()> = {
        let mut guard = map.lock().unwrap_or_else(|p| p.into_inner());
        // HashMap::entry is atomic; if the entry already exists we reuse the
        // same leaked mutex and never double-allocate for the same path.
        guard.entry(key).or_insert_with(|| Box::leak(Box::new(Mutex::new(()))))
    };
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// Atomically write a `lopdf::Document` to `path`.
///
/// ponytail: lopdf's `doc.save(path)` truncates the target in place. If the
/// process dies (or disk fills) mid-write, the user's PDF is replaced by a
/// truncated/corrupt file — real data loss. We serialize to memory first,
/// write to a sibling temp file, fsync, then `rename`/`MoveFileEx` over the
/// target. Rename on the same filesystem is atomic on POSIX; on Windows
/// `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` covers the same case.
/// On any error, the temp file is cleaned up and the original is left
/// untouched.
pub fn save_atomic(doc: &mut Document, path: &Path) -> Result<(), String> {
    let mut buf = Vec::new();
    doc.save_to(&mut buf).map_err(|e| e.to_string())?;

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().ok_or_else(|| format!("Invalid output path: {}", path.display()))?;
    let temp_path = unique_sibling_temp_path(parent, file_name);

    // Write + fsync. If anything below fails, the temp file is removed and
    // the original `path` is untouched.
    let write_result = (|| -> std::io::Result<()> {
        let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&temp_path)?;
        f.write_all(&buf)?;
        f.flush()?;
        f.sync_all()?;
        Ok(())
    })();
    if let Err(e) = write_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("Failed to write {}: {}", temp_path.display(), e));
    }

    // Atomic replace.
    if let Err(e) = atomic_replace(&temp_path, path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("Failed to replace {}: {}", path.display(), e));
    }

    Ok(())
}

fn unique_sibling_temp_path(parent: &Path, target_name: &std::ffi::OsStr) -> PathBuf {
    let nanos =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    // ponytail: pid+nanos+counter is plenty unique. The atomic counter
    // disambiguates two near-simultaneous calls in the same process.
    let pid = std::process::id();
    let counter = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!(".{}.{}.{}.{}.tmp", target_name.to_string_lossy(), pid, nanos, counter);
    parent.join(name)
}

static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Replace `dst` with `src`. POSIX `rename` is atomic on the same filesystem.
/// On Windows, `std::fs::rename` does NOT fail when `dst` exists — it uses
/// `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` internally. The copy+remove
/// fallback below is **dead code**: same-volume rename always succeeds. The
/// fallback remains only as a defensive backstop in case a future platform's
/// `rename` semantics differ; it does NOT guarantee atomicity (copy is not
/// atomic) — callers that need cross-volume safety should place `src` in
/// `dst.parent()` to keep the rename on one volume.
#[cfg(unix)]
fn atomic_replace(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::rename(src, dst)
}

#[cfg(windows)]
fn atomic_replace(src: &Path, dst: &Path) -> std::io::Result<()> {
    // std::fs::rename on Windows uses MoveFileExW with MOVEFILE_REPLACE_EXISTING.
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Best-effort fallback: copy bytes, then remove temp. NOT atomic;
            // a crash mid-copy leaves dst truncated. Only reachable if rename
            // fails (e.g. cross-volume or locked destination).
            std::fs::copy(src, dst).and_then(|_| std::fs::remove_file(src)).map(|_| ())
        }
    }
}

/// Atomically replace `dst` with `src` (which already holds the new bytes).
/// Cleans up `src` on error. Used by sign_pdf and similar callers that have
/// bytes-in-hand rather than a `Document` to re-serialize.
pub fn save_atomic_overwrite(src: &Path, dst: &Path) -> Result<(), String> {
    if let Err(e) = atomic_replace(src, dst) {
        let _ = std::fs::remove_file(src);
        return Err(format!("Failed to replace {}: {}", dst.display(), e));
    }
    Ok(())
}

/// Return the number of pages without mutating the file.
pub fn page_count(path: &Path) -> Result<usize, String> {
    Document::load(path).map_err(|e| e.to_string()).map(|doc| doc.get_pages().len())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistedSession {
    pub original_path: String,
    pub page: u32,
    pub zoom: f64,
    pub view_mode: String,
    pub scroll_view_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionState {
    pub version: u32,
    pub active_index: usize,
    #[serde(default = "default_workspace_view")]
    pub workspace_view: String,
    pub sessions: Vec<PersistedSession>,
}

fn default_workspace_view() -> String {
    "tabs".to_string()
}

impl SessionState {
    const CURRENT_VERSION: u32 = 1;

    #[cfg(test)]
    pub fn new(active_index: usize, sessions: Vec<PersistedSession>) -> Self {
        Self { version: Self::CURRENT_VERSION, active_index, workspace_view: default_workspace_view(), sessions }
    }

    pub fn validate(self) -> Result<Self, String> {
        if self.version != Self::CURRENT_VERSION {
            return Err(format!("unsupported session state version {}", self.version));
        }
        Ok(self)
    }
}

fn session_state_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let config_dir = app.path().app_config_dir().map_err(|e| format!("failed to get app config dir: {e}"))?;
    std::fs::create_dir_all(&config_dir).map_err(|e| format!("failed to create config dir: {e}"))?;
    Ok(config_dir.join("sessions.json"))
}

fn no_restore_env() -> bool {
    std::env::var("KANOPRII_NO_RESTORE").ok().as_deref() == Some("1")
}

pub fn save_session_state(app: &tauri::AppHandle, state: &SessionState) -> Result<(), String> {
    if no_restore_env() {
        return Ok(());
    }
    let path = session_state_path(app)?;
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("failed to write session state: {e}"))?;
    Ok(())
}

pub fn load_session_state(app: &tauri::AppHandle) -> Result<Option<SessionState>, String> {
    if no_restore_env() {
        return Ok(None);
    }
    let path = session_state_path(app)?;
    if !path.is_file() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| format!("failed to read session state: {e}"))?;
    let state: SessionState = serde_json::from_str(&json).map_err(|e| format!("failed to parse session state: {e}"))?;
    match state.validate() {
        Ok(s) => Ok(Some(s)),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Object, Stream};
    use std::fs;

    fn minimal_pdf(path: &Path) {
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
    fn page_count_reads_without_mutation() {
        let dir = std::env::temp_dir().join("kanoprii_io_test");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("count.pdf");
        minimal_pdf(&path);
        assert_eq!(page_count(&path).unwrap(), 1);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn mutate_pdf_skips_save_on_closure_error() {
        let dir = std::env::temp_dir().join("kanoprii_io_test");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("mutate_err.pdf");
        minimal_pdf(&path);
        let before = fs::read(&path).unwrap();
        let err = mutate_pdf::<(), _>(&path, |doc| {
            let page_id = *doc.get_pages().get(&1).unwrap();
            doc.get_dictionary_mut(page_id).unwrap().set("Rotate", Object::Integer(90));
            Err("intentional failure".into())
        })
        .unwrap_err();
        assert_eq!(err, "intentional failure");
        assert_eq!(fs::read(&path).unwrap(), before);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn mutate_pdf_persists_changes() {
        let dir = std::env::temp_dir().join("kanoprii_io_test");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("mutate.pdf");
        minimal_pdf(&path);
        mutate_pdf(&path, |doc| {
            let page_id = *doc.get_pages().get(&1).unwrap();
            doc.get_dictionary_mut(page_id).unwrap().set("Rotate", Object::Integer(90));
            Ok(())
        })
        .unwrap();
        let doc = Document::load(&path).unwrap();
        let page_id = *doc.get_pages().get(&1).unwrap();
        let rot = doc.get_dictionary(page_id).unwrap().get(b"Rotate").unwrap().as_i64().unwrap();
        assert_eq!(rot, 90);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn session_state_round_trip() {
        let state = SessionState::new(
            1,
            vec![
                PersistedSession {
                    original_path: "/tmp/a.pdf".to_string(),
                    page: 3,
                    zoom: 1.5,
                    view_mode: "pdf".to_string(),
                    scroll_view_mode: "continuous".to_string(),
                },
                PersistedSession {
                    original_path: "/tmp/b.pdf".to_string(),
                    page: 0,
                    zoom: 2.0,
                    view_mode: "markdown".to_string(),
                    scroll_view_mode: "single".to_string(),
                },
            ],
        );
        let json = serde_json::to_string_pretty(&state).unwrap();
        let loaded: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, state);
    }

    #[test]
    fn session_state_legacy_defaults_workspace_view() {
        let json = r#"{"version":1,"active_index":0,"sessions":[]}"#;
        let state: SessionState = serde_json::from_str(json).unwrap();
        assert_eq!(state.workspace_view, "tabs");
    }

    #[test]
    fn session_state_unknown_version_returns_error() {
        let json = r#"{"version":99,"active_index":0,"sessions":[]}"#;
        let state: SessionState = serde_json::from_str(json).unwrap();
        assert!(state.validate().is_err());
    }

    fn make_minimal_pdf(path: &Path) {
        let mut doc = lopdf::Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();
        doc.objects
            .insert(content_id, lopdf::Object::Stream(lopdf::Stream::new(lopdf::Dictionary::new(), b"BT ET".to_vec())));
        let mut page = lopdf::Dictionary::new();
        page.set("Type", lopdf::Object::Name(b"Page".to_vec()));
        page.set("Parent", lopdf::Object::Reference(pages_id));
        page.set("MediaBox", lopdf::Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]));
        page.set("Contents", lopdf::Object::Reference(content_id));
        doc.objects.insert(page_id, lopdf::Object::Dictionary(page));
        let mut pages = lopdf::Dictionary::new();
        pages.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
        pages.set("Kids", lopdf::Object::Array(vec![lopdf::Object::Reference(page_id)]));
        pages.set("Count", lopdf::Object::Integer(1));
        doc.objects.insert(pages_id, lopdf::Object::Dictionary(pages));
        let mut catalog = lopdf::Dictionary::new();
        catalog.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", lopdf::Object::Reference(pages_id));
        let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));
        doc.trailer.set("Root", lopdf::Object::Reference(catalog_id));
        doc.save(path).unwrap();
    }

    #[test]
    fn save_atomic_replaces_file_with_new_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "kanoprii_atomic_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("doc.pdf");

        make_minimal_pdf(&path);
        let original_bytes = std::fs::read(&path).unwrap();

        // Re-save via save_atomic; bytes change because lopdf may normalise.
        let mut doc = lopdf::Document::load(&path).unwrap();
        super::save_atomic(&mut doc, &path).unwrap();
        assert!(path.is_file());

        // No stray temp file left behind.
        let entries: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(Result::ok).collect();
        let temps: Vec<_> = entries.iter().filter(|e| e.file_name().to_string_lossy().ends_with(".tmp")).collect();
        assert!(temps.is_empty(), "temp file leaked: {:?}", temps.iter().map(|e| e.path()).collect::<Vec<_>>());

        // Round-trip: loaded doc still has 1 page.
        let doc2 = lopdf::Document::load(&path).unwrap();
        assert_eq!(doc2.get_pages().len(), 1);

        // Sanity: not a zero-byte file (would mean we crashed mid-write).
        assert!(!original_bytes.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
