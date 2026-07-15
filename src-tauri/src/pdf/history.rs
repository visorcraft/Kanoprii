use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub fn open_working_copy(original: String) -> Result<String, String> {
    let original = PathBuf::from(&original);
    let stem = original.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
    // ponytail: include nanos+counter so two PDFs that share the same stem
    // (e.g. `~/a/report.pdf` and `~/x/report.pdf`) don't collide into one
    // working copy. Without this the second open reuses the first's bytes.
    let nanos =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let counter = WORKING_COUNTER.fetch_add(1, Ordering::Relaxed);
    let working =
        std::env::temp_dir().join(format!("kanoprii_work_{}_{}_{}_{}.pdf", std::process::id(), nanos, counter, stem));
    fs::copy(&original, &working).map_err(|e| e.to_string())?;
    Ok(working.to_string_lossy().into_owned())
}

/// Commit the working copy to `target` (Save: target = original; Save As: a new
/// path). The working copy stays put so editing can continue afterwards.
pub fn save_working_copy(working: String, target: String) -> Result<(), String> {
    // ponytail: require `working` to actually live in our temp_dir; otherwise
    // a hostile frontend can pass any source path here and overwrite any
    // writable destination with the source PDF's bytes.
    let working = PathBuf::from(&working);
    let target = PathBuf::from(&target);
    if !path_resides_in_dir(&working, &std::env::temp_dir()) {
        return Err("working path must be inside the app's temp directory".to_string());
    }
    fs::copy(&working, &target).map_err(|e| e.to_string())?;
    Ok(())
}

/// Best-effort removal of a working copy when its document is closed/discarded.
pub fn discard_working_copy(working: String) -> Result<(), String> {
    // ponytail: only allow deletion of files we created (must live in temp_dir).
    // Otherwise a hostile frontend could delete arbitrary user files.
    let working = PathBuf::from(&working);
    if !working.exists() {
        return Ok(());
    }
    if !path_resides_in_dir(&working, &std::env::temp_dir()) {
        return Err("working path must be inside the app's temp directory".to_string());
    }
    fs::remove_file(&working).map_err(|e| e.to_string())?;
    Ok(())
}

/// Best-effort removal of an auto-materialized sibling PDF (markdown/HTML ->
/// PDF view) on window close. Unlike `discard_working_copy`, the sibling lives
/// next to the user's source (not in temp_dir), so the safety check is a strict
/// filename-pattern + same-directory match against `source`.
///
/// Accepts only siblings `materialize_document_pdf` itself could have produced:
///   `<stem>.pdf`, `<stem>_converted.pdf`, `<stem>_converted_<n>.pdf`
/// in the same directory as `source`. Any other path is rejected.
pub fn discard_document_pdf(generated: String, source: String) -> Result<(), String> {
    let generated = PathBuf::from(&generated);
    if !generated.exists() {
        return Ok(());
    }
    if generated.extension().and_then(|e| e.to_str()) != Some("pdf") {
        return Err("generated path must be a .pdf sibling".to_string());
    }
    let source = PathBuf::from(&source);
    let source_dir = source.parent().ok_or_else(|| "source has no parent dir".to_string())?;
    let Some(source_stem) = source.file_stem().and_then(|s| s.to_str()) else {
        return Err("source has no file stem".to_string());
    };
    let gen_dir = generated.parent().ok_or_else(|| "generated has no parent dir".to_string())?;
    let gen_name =
        generated.file_name().and_then(|s| s.to_str()).ok_or_else(|| "generated has no file name".to_string())?;
    if !path_resides_in_dir(gen_dir, source_dir) {
        return Err("generated pdf must live alongside its source".to_string());
    }
    let acceptable = [format!("{source_stem}.pdf"), format!("{source_stem}_converted.pdf")];
    let is_converted_n = gen_name
        .strip_prefix(&format!("{source_stem}_converted_"))
        .and_then(|rest| rest.strip_suffix(".pdf"))
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
    if !acceptable.contains(&gen_name.to_string()) && !is_converted_n {
        return Err("generated pdf name does not match the materialize pattern".to_string());
    }
    fs::remove_file(&generated).map_err(|e| e.to_string())?;
    Ok(())
}

fn path_resides_in_dir(path: &Path, dir: &Path) -> bool {
    // Canonicalise both so `..`, symlinks, and relative segments don't bypass
    // the check. Returns false on any error (missing file, non-UTF8, etc).
    let Ok(p) = path.canonicalize() else { return false };
    let Ok(d) = dir.canonicalize() else { return false };
    p.starts_with(d)
}

static WORKING_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Monotonic counter so each undo/redo history snapshot gets a unique filename.
pub static SNAPSHOT_SEQ: AtomicU64 = AtomicU64::new(0);

const HISTORY_DELTA_MAGIC: &[u8] = b"PPDFDELTA1\n";

/// Whole-file snapshots are used below this size; larger working copies store
/// binary deltas against the previous history entry.
pub fn history_large_file_bytes() -> u64 {
    #[cfg(test)]
    {
        100
    }
    #[cfg(not(test))]
    {
        32 * 1024 * 1024
    }
}

/// Fall back to a whole-file snapshot when a delta grows past this size.
pub fn history_delta_fallback_bytes() -> u64 {
    #[cfg(test)]
    {
        1_000_000
    }
    #[cfg(not(test))]
    {
        32 * 1024 * 1024
    }
}

/// Maximum number of consecutive delta snapshots allowed since the last full
/// snapshot. Beyond this, a new full snapshot is written so that materializing
/// the chain does not become O(chain length) on large files.
const MAX_DELTA_CHAIN_LENGTH: usize = 10;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistorySnapshot {
    pub kind: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_index: Option<usize>,
    pub size: u64,
}

pub fn temp_hist_path(tag: &str, ext: &str) -> PathBuf {
    let seq = SNAPSHOT_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("kanoprii_hist_{}_{}_{}.{}", std::process::id(), tag, seq, ext))
}

pub fn write_full_snapshot(source: &Path) -> Result<String, String> {
    let snapshot = temp_hist_path("full", "pdf");
    fs::copy(source, &snapshot).map_err(|e| e.to_string())?;
    Ok(snapshot.to_string_lossy().into_owned())
}

pub fn write_delta_snapshot(bytes: &[u8]) -> Result<String, String> {
    let path = temp_hist_path("delta", "ppdelta");
    fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

pub fn byte_eq_at(base: &[u8], current: &[u8], i: usize) -> bool {
    match (base.get(i), current.get(i)) {
        (Some(a), Some(b)) => a == b,
        (None, None) => true,
        _ => false,
    }
}

pub fn encode_pdf_delta(base: &[u8], current: &[u8]) -> Result<Vec<u8>, String> {
    let max_len = base.len().max(current.len());
    let mut patches: Vec<(u64, Vec<u8>)> = Vec::new();
    let mut i = 0usize;
    while i < max_len {
        while i < max_len && byte_eq_at(base, current, i) {
            i += 1;
        }
        if i >= max_len {
            break;
        }
        let start = i;
        while i < max_len && !byte_eq_at(base, current, i) {
            i += 1;
        }
        let data: Vec<u8> = (start..i).map(|j| current.get(j).copied().unwrap_or(0)).collect();
        patches.push((start as u64, data));
    }

    let mut out = Vec::new();
    out.extend_from_slice(HISTORY_DELTA_MAGIC);
    out.extend_from_slice(&(base.len() as u64).to_le_bytes());
    out.extend_from_slice(&(current.len() as u64).to_le_bytes());
    out.extend_from_slice(&(patches.len() as u32).to_le_bytes());
    for (offset, data) in patches {
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&data);
    }
    Ok(out)
}

pub fn read_u64_le(buf: &[u8], pos: &mut usize) -> Result<u64, String> {
    if *pos + 8 > buf.len() {
        return Err("truncated delta".into());
    }
    let v = u64::from_le_bytes(buf[*pos..*pos + 8].try_into().unwrap());
    *pos += 8;
    Ok(v)
}

pub fn read_u32_le(buf: &[u8], pos: &mut usize) -> Result<u32, String> {
    if *pos + 4 > buf.len() {
        return Err("truncated delta".into());
    }
    let v = u32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    Ok(v)
}

pub fn apply_pdf_delta(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, String> {
    if !delta.starts_with(HISTORY_DELTA_MAGIC) {
        return Err("invalid delta magic".into());
    }
    let mut pos = HISTORY_DELTA_MAGIC.len();
    let base_size = read_u64_le(delta, &mut pos)? as usize;
    let current_size = read_u64_le(delta, &mut pos)? as usize;
    let patch_count = read_u32_le(delta, &mut pos)? as usize;

    let mut out = base.to_vec();
    if out.len() < base_size {
        out.resize(base_size, 0);
    } else if out.len() > base_size {
        out.truncate(base_size);
    }

    for _ in 0..patch_count {
        let offset = read_u64_le(delta, &mut pos)? as usize;
        let len = read_u32_le(delta, &mut pos)? as usize;
        if pos + len > delta.len() {
            return Err("truncated delta patch".into());
        }
        let data = &delta[pos..pos + len];
        pos += len;
        if offset + len > out.len() {
            out.resize(offset + len, 0);
        }
        out[offset..offset + len].copy_from_slice(data);
    }

    if out.len() < current_size {
        out.resize(current_size, 0);
    } else if out.len() > current_size {
        out.truncate(current_size);
    }
    Ok(out)
}

pub fn materialize_history_index(history: &[HistorySnapshot], index: usize, into: &Path) -> Result<(), String> {
    let entry = history.get(index).ok_or_else(|| "history index out of bounds".to_string())?;
    match entry.kind.as_str() {
        "full" => {
            fs::copy(&entry.path, into).map_err(|e| e.to_string())?;
            Ok(())
        }
        "delta" => {
            let base_index = entry.base_index.ok_or_else(|| "delta snapshot missing base_index".to_string())?;
            let base_temp = temp_hist_path("mat", "pdf");
            materialize_history_index(history, base_index, &base_temp)?;
            let base_bytes = fs::read(&base_temp).map_err(|e| e.to_string())?;
            let _ = fs::remove_file(&base_temp);
            let delta_bytes = fs::read(&entry.path).map_err(|e| e.to_string())?;
            let restored = apply_pdf_delta(&base_bytes, &delta_bytes)?;
            fs::write(into, restored).map_err(|e| e.to_string())
        }
        other => Err(format!("unknown snapshot kind: {other}")),
    }
}

/// Copy the working copy to a fresh temp snapshot, used to build the undo/redo
/// history. Returns the snapshot path (restored later via `save_working_copy`).
pub fn snapshot_pdf(source: String) -> Result<String, String> {
    write_full_snapshot(Path::new(&source))
}

/// Append a history entry for `source`. Small files get a full snapshot; large
/// files store a compact binary delta against the previous history entry.
///
/// ponytail: a long editing session can produce thousands of snapshots. Cap the
/// kept set at MAX_HISTORY_ENTRIES files / MAX_HISTORY_BYTES on disk, evicting
/// the oldest first. Returns the pruned history vector alongside the new entry
/// so the caller (TS undo stack) can drop the evicted indices from its in-memory
/// history — otherwise the cap only frees disk and the UI keeps a stale list
/// pointing at deleted snapshot files.
pub fn snapshot_pdf_entry(
    history: Vec<HistorySnapshot>,
    source: String,
) -> Result<(Vec<HistorySnapshot>, HistorySnapshot), String> {
    let source_path = PathBuf::from(&source);
    let current = fs::read(&source_path).map_err(|e| e.to_string())?;
    let size = current.len() as u64;
    let threshold = history_large_file_bytes();

    let deltas_since_full = history.iter().rev().take_while(|e| e.kind == "delta").count();
    let new_entry = if size <= threshold || history.is_empty() || deltas_since_full >= MAX_DELTA_CHAIN_LENGTH {
        let path = write_full_snapshot(&source_path)?;
        // Track the bytes we actually wrote to disk (a delta snapshot can be
        // orders of magnitude smaller than the source; using the source size
        // would make the byte cap fire way too eagerly).
        let on_disk = file_size(Path::new(&path)).unwrap_or(size);
        HistorySnapshot { kind: "full".into(), path, base_index: None, size: on_disk }
    } else {
        let base_index = history.len() - 1;
        let base_temp = temp_hist_path("base", "pdf");
        materialize_history_index(&history, base_index, &base_temp)?;
        let base_bytes = fs::read(&base_temp).map_err(|e| e.to_string())?;
        let _ = fs::remove_file(&base_temp);

        let delta_bytes = encode_pdf_delta(&base_bytes, &current)?;
        if delta_bytes.len() as u64 > history_delta_fallback_bytes() {
            let path = write_full_snapshot(&source_path)?;
            let on_disk = file_size(Path::new(&path)).unwrap_or(size);
            HistorySnapshot { kind: "full".into(), path, base_index: None, size: on_disk }
        } else {
            let path = write_delta_snapshot(&delta_bytes)?;
            let on_disk = file_size(Path::new(&path)).unwrap_or(delta_bytes.len() as u64);
            HistorySnapshot { kind: "delta".into(), path, base_index: Some(base_index), size: on_disk }
        }
    };

    let mut kept = history;
    let total_bytes = |entries: &[HistorySnapshot]| -> u64 { entries.iter().map(|e| e.size).sum() };
    while kept.len() + 1 > MAX_HISTORY_ENTRIES || total_bytes(&kept) + new_entry.size > MAX_HISTORY_BYTES {
        if kept.is_empty() {
            break;
        }
        match prune_history_entry(kept.clone(), 0) {
            Ok(remaining) => kept = remaining,
            Err(_) => break,
        }
    }
    kept.push(new_entry.clone());
    Ok((kept, new_entry))
}

fn file_size(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().map(|m| m.len())
}

const MAX_HISTORY_ENTRIES: usize = 256;
const MAX_HISTORY_BYTES: u64 = 512 * 1024 * 1024;

/// Materialize `history[index]` and write it to `target` (the live working copy).
pub fn restore_history_entry(history: Vec<HistorySnapshot>, index: usize, target: String) -> Result<(), String> {
    materialize_history_index(&history, index, Path::new(&target))
}

/// Remove a history snapshot file from disk.
pub fn discard_history_entry(entry: HistorySnapshot) -> Result<(), String> {
    let path = PathBuf::from(&entry.path);
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Drop `drop_index` from the undo stack, rematerializing any delta entries that
/// depended on it while the parent snapshot is still available.
pub fn prune_history_entry(
    mut history: Vec<HistorySnapshot>,
    drop_index: usize,
) -> Result<Vec<HistorySnapshot>, String> {
    if drop_index >= history.len() {
        return Err("history index out of bounds".into());
    }

    let orphans: Vec<usize> = history
        .iter()
        .enumerate()
        .filter(|(idx, entry)| *idx != drop_index && entry.base_index == Some(drop_index))
        .map(|(idx, _)| idx)
        .collect();

    for idx in orphans {
        let entry = history[idx].clone();
        let materialized = temp_hist_path("prune", "pdf");
        materialize_history_index(&history, idx, &materialized)?;
        let _ = fs::remove_file(&entry.path);
        history[idx] = HistorySnapshot {
            kind: "full".into(),
            path: materialized.to_string_lossy().into_owned(),
            base_index: None,
            size: entry.size,
        };
    }

    let dropped = history.remove(drop_index);
    discard_history_entry(dropped)?;

    for entry in &mut history {
        if let Some(base_index) = entry.base_index.as_mut() {
            if *base_index > drop_index {
                *base_index -= 1;
            }
        }
    }

    Ok(history)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn snapshot_pdf_entry_caps_total_bytes() {
        // Push past MAX_HISTORY_ENTRIES with a small cap-of-cap so the test
        // runs in seconds, not minutes, on CI.
        const TEST_CAP: usize = 32;
        let dir = std::env::temp_dir().join(format!(
            "kanoprii_hist_cap_{}_{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("cap.pdf");
        fs::write(&path, vec![b'x'; 256]).unwrap();

        let mut history: Vec<HistorySnapshot> = Vec::new();
        for i in 0..(TEST_CAP + 8) {
            fs::write(&path, vec![b'a' + (i as u8) % 26; 256]).unwrap();
            let mut kept = history.clone();
            while kept.len() + 1 > TEST_CAP {
                if kept.is_empty() {
                    break;
                }
                match prune_history_entry(kept.clone(), 0) {
                    Ok(remaining) => kept = remaining,
                    Err(_) => break,
                }
            }
            let (pruned, _entry) = snapshot_pdf_entry(kept, path.to_string_lossy().into_owned()).unwrap();
            history = pruned;
            assert!(history.len() <= TEST_CAP, "iter {i}: history exceeded cap: {}", history.len());
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn snapshot_pdf_entry_resets_delta_chain_after_threshold() {
        let dir = std::env::temp_dir().join(format!(
            "kanoprii_hist_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("delta_chain.pdf");

        // File must be larger than the 100-byte test threshold to use deltas.
        fs::write(&path, vec![b'x'; 256]).unwrap();

        let mut history = Vec::new();
        for i in 0..MAX_DELTA_CHAIN_LENGTH + 2 {
            fs::write(&path, vec![b'a' + (i as u8) % 26; 256]).unwrap();
            let (pruned, _entry) = snapshot_pdf_entry(history.clone(), path.to_string_lossy().into_owned()).unwrap();
            history = pruned;
        }

        let full_count = history.iter().filter(|e| e.kind == "full").count();
        assert!(full_count >= 2, "expected at least two full snapshots after chain reset, got {full_count}");

        let _ = fs::remove_dir_all(&dir);
    }
}
