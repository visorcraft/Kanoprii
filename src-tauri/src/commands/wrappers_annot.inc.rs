#[tauri::command]
fn protect_pdf(path: String, user_password: String, owner_password: Option<String>) -> Result<String, String> {
    pdf::security::protect_pdf(path, user_password, owner_password)
}
#[tauri::command]
fn list_pdf_signatures(path: String) -> Result<Vec<PdfSignatureInfo>, String> {
    pdf::security::list_pdf_signatures(path)
}
#[tauri::command]
fn verify_pdf_signatures(
    path: String,
    trust_pem_path: Option<String>,
) -> Result<PdfSignatureVerificationSummary, String> {
    pdf::security::verify_pdf_signatures(path, trust_pem_path)
}
#[tauri::command]
fn sign_pdf(
    path: String,
    cert_path: String,
    cert_password: String,
    reason: Option<String>,
    location: Option<String>,
    field_name: Option<String>,
    output_path: Option<String>,
) -> Result<String, String> {
    pdf::security::sign_pdf(path, cert_path, cert_password, reason, location, field_name, output_path)
}
#[tauri::command]
fn open_working_copy(original: String) -> Result<String, String> {
    pdf::history::open_working_copy(original)
}
#[tauri::command]
fn save_working_copy(working: String, target: String) -> Result<(), String> {
    pdf::history::save_working_copy(working, target)
}
#[tauri::command]
fn discard_working_copy(working: String) -> Result<(), String> {
    pdf::history::discard_working_copy(working)
}
#[tauri::command]
fn snapshot_pdf(source: String) -> Result<String, String> {
    pdf::history::snapshot_pdf(source)
}
#[tauri::command]
fn snapshot_pdf_entry(
    history: Vec<HistorySnapshot>,
    source: String,
) -> Result<(Vec<HistorySnapshot>, HistorySnapshot), String> {
    pdf::history::snapshot_pdf_entry(history, source)
}
#[tauri::command]
fn restore_history_entry(history: Vec<HistorySnapshot>, index: usize, target: String) -> Result<(), String> {
    pdf::history::restore_history_entry(history, index, target)
}
#[tauri::command]
fn discard_history_entry(entry: HistorySnapshot) -> Result<(), String> {
    pdf::history::discard_history_entry(entry)
}
#[tauri::command]
fn prune_history_entry(history: Vec<HistorySnapshot>, drop_index: usize) -> Result<Vec<HistorySnapshot>, String> {
    pdf::history::prune_history_entry(history, drop_index)
}
#[tauri::command]
fn add_highlight(path: String, page_index: u32, x1: f64, y1: f64, x2: f64, y2: f64) -> Result<(), String> {
    pdf::annotations::add_highlight(&PathBuf::from(path), page_index, x1, y1, x2, y2)
}
/// Remove the `index`-th highlight annotation (0-based, in document order) from a
/// page. The index matches the order highlights are returned by
/// `get_annotations` after filtering to the `Highlight` subtype.
#[tauri::command]
fn remove_highlight(path: String, page_index: u32, index: u32) -> Result<(), String> {
    pdf::annotations::remove_highlight(&PathBuf::from(path), page_index, index)
}
#[tauri::command]
fn add_text_note(path: String, page_index: u32, x: f64, y: f64, content: String) -> Result<(), String> {
    pdf::annotations::add_text_note(&PathBuf::from(path), page_index, x, y, content)
}
/// Remove the `index`-th text-note annotation (0-based among `Text` subtypes).
#[tauri::command]
fn remove_text_note(path: String, page_index: u32, index: u32) -> Result<(), String> {
    pdf::annotations::remove_text_note(&PathBuf::from(path), page_index, index)
}
#[tauri::command]
fn add_ink_stroke(path: String, page_index: u32, points: Vec<f64>) -> Result<(), String> {
    pdf::annotation_markup::add_ink_stroke(&PathBuf::from(path), page_index, points)
}
#[tauri::command]
fn remove_ink_stroke(path: String, page_index: u32, index: u32) -> Result<(), String> {
    pdf::annotation_markup::remove_ink_stroke(&PathBuf::from(path), page_index, index)
}
#[tauri::command]
fn add_square(path: String, page_index: u32, x1: f64, y1: f64, x2: f64, y2: f64) -> Result<(), String> {
    pdf::annotation_markup::add_square(&PathBuf::from(path), page_index, x1, y1, x2, y2)
}
#[tauri::command]
fn add_circle(path: String, page_index: u32, x1: f64, y1: f64, x2: f64, y2: f64) -> Result<(), String> {
    pdf::annotation_markup::add_circle(&PathBuf::from(path), page_index, x1, y1, x2, y2)
}
#[tauri::command]
fn add_line(path: String, page_index: u32, x1: f64, y1: f64, x2: f64, y2: f64) -> Result<(), String> {
    pdf::annotation_markup::add_line(&PathBuf::from(path), page_index, x1, y1, x2, y2)
}
#[tauri::command]
fn remove_square(path: String, page_index: u32, index: u32) -> Result<(), String> {
    pdf::annotation_markup::remove_square(&PathBuf::from(path), page_index, index)
}
#[tauri::command]
fn remove_circle(path: String, page_index: u32, index: u32) -> Result<(), String> {
    pdf::annotation_markup::remove_circle(&PathBuf::from(path), page_index, index)
}
#[tauri::command]
fn remove_line(path: String, page_index: u32, index: u32) -> Result<(), String> {
    pdf::annotation_markup::remove_line(&PathBuf::from(path), page_index, index)
}
#[tauri::command]
fn list_stamp_presets() -> Vec<pdf::annotation_markup::StampPresetInfo> {
    pdf::annotation_markup::list_stamp_presets()
}
#[tauri::command]
fn add_text_stamp(path: String, page_index: u32, x: f64, y: f64, preset: String) -> Result<(), String> {
    pdf::annotation_markup::add_text_stamp(&PathBuf::from(path), page_index, x, y, preset)
}
#[tauri::command]
fn add_image_stamp(path: String, page_index: u32, x: f64, y: f64, preset: String) -> Result<(), String> {
    pdf::annotation_markup::add_image_stamp(&PathBuf::from(path), page_index, x, y, preset)
}
#[tauri::command]
fn remove_text_stamp(path: String, page_index: u32, index: u32) -> Result<(), String> {
    pdf::annotation_markup::remove_text_stamp(&PathBuf::from(path), page_index, index)
}
#[tauri::command]
fn remove_image_stamp(path: String, page_index: u32, index: u32) -> Result<(), String> {
    pdf::annotation_markup::remove_image_stamp(&PathBuf::from(path), page_index, index)
}
#[tauri::command]
fn add_redaction(path: String, page_index: u32, x1: f64, y1: f64, x2: f64, y2: f64) -> Result<(), String> {
    pdf::annotation_markup::add_redaction(&PathBuf::from(path), page_index, x1, y1, x2, y2)
}
#[tauri::command]
fn remove_redaction(path: String, page_index: u32, index: u32) -> Result<(), String> {
    pdf::annotation_markup::remove_redaction(&PathBuf::from(path), page_index, index)
}
#[tauri::command]
fn get_annotations(path: String, page_index: u32) -> Result<Vec<pdf::annotations::AnnotationData>, String> {
    pdf::annotations::get_annotations(&PathBuf::from(path), page_index)
}
#[tauri::command]
fn list_document_annotations(path: String) -> Result<Vec<pdf::annotations::DocAnnotation>, String> {
    pdf::annotations::list_document_annotations(&PathBuf::from(path))
}
#[tauri::command]
fn updater_supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("APPIMAGE").is_some()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

fn parse_latest_json(body: &str, current: &str) -> Result<LatestVersionInfo, String> {
    let json: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    let version = json
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or("Missing version field")?
        .to_string();
    let notes = json
        .get("notes")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let newer = version_newer(&version, current);
    let linux_packages = json.get("linux_packages").map(|lp| {
        let parse_ref = |key: &str| -> Option<LinuxPackageRef> {
            let entry = lp.get(key)?;
            Some(LinuxPackageRef {
                url: entry.get("url")?.as_str()?.to_string(),
                sha256: entry.get("sha256")?.as_str()?.to_string(),
            })
        };
        LinuxPackages {
            deb: parse_ref("deb"),
            rpm: parse_ref("rpm"),
        }
    });
    Ok(LatestVersionInfo {
        version,
        notes,
        current: current.to_string(),
        newer,
        linux_packages,
    })
}

#[tauri::command]
fn fetch_latest_version() -> Result<LatestVersionInfo, String> {
    const URL: &str = "https://github.com/visorcraft/Kanoprii/releases/latest/download/latest.json";
    let body = if let Ok(path) = std::env::var("KANOPRII_LATEST_JSON_PATH") {
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read latest version override: {}", e))?
    } else if let Ok(body) = std::env::var("KANOPRII_LATEST_JSON") {
        body
    } else {
        ureq::get(URL)
            .call()
            .map_err(|e| format!("Failed to fetch latest version: {}", e))?
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("Failed to read response: {}", e))?
    };
    let current = env!("CARGO_PKG_VERSION").to_string();
    parse_latest_json(&body, &current)
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Invalid URL scheme: only http and https are allowed".into());
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }
    Ok(())
}

pub fn version_newer(a: &str, b: &str) -> bool {
    let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
    for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
        match a_part.cmp(b_part) {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => continue,
        }
    }
    a_parts.len() > b_parts.len()
}

/// Pure update-channel classifier so the decision logic is unit-testable.
/// Returns one of: "appimage", "deb", "rpm", "manual", "supported".
pub fn resolve_update_channel(
    forced: Option<&str>,
    is_linux: bool,
    appimage: bool,
    dpkg_owns: bool,
    rpm_owns: bool,
) -> String {
    if let Some(f) = forced {
        if !f.is_empty() {
            return f.to_string();
        }
    }
    if !is_linux {
        return "supported".to_string();
    }
    if appimage {
        return "appimage".to_string();
    }
    if dpkg_owns {
        return "deb".to_string();
    }
    if rpm_owns {
        return "rpm".to_string();
    }
    "manual".to_string()
}

#[cfg(target_os = "linux")]
fn path_owned_by_package(program: &str, query_arg: &str, path: &std::path::Path) -> bool {
    std::process::Command::new(program)
        .arg(query_arg)
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[tauri::command]
fn update_channel() -> String {
    let forced = std::env::var("KANOPRII_UPDATE_CHANNEL").ok();
    #[cfg(target_os = "linux")]
    {
        let appimage = std::env::var_os("APPIMAGE").is_some();
        let exe = std::env::current_exe().ok();
        let dpkg_owns = exe
            .as_deref()
            .map(|p| path_owned_by_package("dpkg", "-S", p))
            .unwrap_or(false);
        let rpm_owns = exe
            .as_deref()
            .map(|p| path_owned_by_package("rpm", "-qf", p))
            .unwrap_or(false);
        resolve_update_channel(forced.as_deref(), true, appimage, dpkg_owns, rpm_owns)
    }
    #[cfg(not(target_os = "linux"))]
    {
        resolve_update_channel(forced.as_deref(), false, false, false, false)
    }
}

pub fn verify_sha256(bytes: &[u8], expected_hex: &str) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let got: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    if got.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(format!(
            "Checksum mismatch: expected {}, got {}",
            expected_hex, got
        ))
    }
}

/// Download a deb/rpm package, verify its SHA-256, then hand it to the
/// desktop's GUI installer (`xdg-open`) which performs the privileged install.
/// Returns the downloaded temp path on success.
/// Validate that a package download URL targets a GitHub release host.
/// The URL originates from the parsed `latest.json` manifest, so without
/// this guard a compromised manifest could redirect (ureq follows redirects
/// by default) to internal services or cloud metadata endpoints.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn check_package_url(url: &str) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("Invalid URL scheme: only https is allowed".into());
    }
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    let host_allowed = host == "github.com"
        || host == "objects.githubusercontent.com"
        || host == "github-releases.githubusercontent.com"
        || host.ends_with(".githubusercontent.com");
    if !host_allowed {
        return Err(format!("Package host not allowed: {host}"));
    }
    if let Some(port) = parsed.port() {
        if port != 443 {
            return Err("Non-default port not allowed".into());
        }
    }
    Ok(())
}

#[tauri::command]
fn download_and_open_package(url: String, sha256: String) -> Result<String, String> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (url, sha256);
        Err("Package install handoff is only supported on Linux".into())
    }
    #[cfg(target_os = "linux")]
    {
        const MAX_PACKAGE_BYTES: usize = 500 * 1024 * 1024;
        check_package_url(&url)?;
        let mut bytes: Vec<u8> = Vec::new();
        use std::io::Read;
        // ponytail: GitHub redirects releases/download/... →
        // objects.githubusercontent.com (302), so we MUST follow redirects.
        // SSRF guard: redirects(5) and re-validate each Location against the
        // host allow-list before the agent follows it.
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(300)))
            .max_redirects(0)
            .build()
            .new_agent();
        const MAX_REDIRECTS: usize = 5;
        let mut current = url.clone();
        let mut redirect_count = 0usize;
        let response = loop {
            let r = agent
                .get(&current)
                .call()
                .map_err(|_| "Failed to download package".to_string())?;
            if r.status().is_redirection() {
                if redirect_count >= MAX_REDIRECTS {
                    return Err("Too many redirects".to_string());
                }
                let loc = r
                    .headers()
                    .get("Location")
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| "Redirect with no Location header".to_string())?
                    .to_string();
                check_package_url(&loc)?;
                current = loc;
                redirect_count += 1;
                continue;
            }
            break r;
        };
        let mut reader = response.into_body().into_reader();
        let mut buf = [0u8; 65536];
        loop {
            let n = reader
                .read(&mut buf)
                .map_err(|_| "Failed to read package".to_string())?;
            if n == 0 {
                break;
            }
            if bytes.len() + n > MAX_PACKAGE_BYTES {
                return Err("Package exceeds maximum allowed size (500 MB)".into());
            }
            bytes.extend_from_slice(&buf[..n]);
        }
        verify_sha256(&bytes, &sha256)?;
        let file_name = url
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .and_then(|s| std::path::Path::new(s).file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "kanoprii-update".into());
        let unique_name = format!(
            "{}.{}",
            file_name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let dest = std::env::temp_dir().join(unique_name);
        {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&dest)
                .map_err(|e| format!("Failed to create temp file: {}", e))?;
            std::io::Write::write_all(&mut f, &bytes)
                .map_err(|e| format!("Failed to write package: {}", e))?;
        }
        std::process::Command::new("xdg-open")
            .arg(&dest)
            .spawn()
            .map_err(|e| format!("Failed to launch system installer: {}", e))?;
        Ok(dest.to_string_lossy().into_owned())
    }
}

#[tauri::command]
fn list_printers() -> Vec<crate::PrinterInfo> {
    pdf::print::list_printers()
}

#[tauri::command]
fn print_document(
    source_path: String,
    opts: crate::PrintOptions,
) -> Result<crate::PrintDocumentResult, String> {
    let temp_dir = crate::print_temp_dir();
    let _ = std::fs::create_dir_all(&temp_dir);
    pdf::print::print_document(Path::new(&source_path), &opts, &temp_dir)
}

#[tauri::command]
fn print_to_pdf(
    source_path: String,
    opts: crate::PrintOptions,
    output_path: String,
) -> Result<(), String> {
    pdf::print::print_to_pdf(Path::new(&source_path), &opts, Path::new(&output_path))
}

#[tauri::command]
fn render_print_preview(
    source_path: String,
    page_index: u32,
    opts: crate::PrintOptions,
    width: i32,
    height: i32,
) -> Result<Vec<u8>, String> {
    let temp_dir = crate::print_temp_dir();
    let _ = std::fs::create_dir_all(&temp_dir);
    pdf::print::render_print_preview(Path::new(&source_path), page_index, &opts, width, height, &temp_dir)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn replace_text_region(
    path: String,
    page_index: u32,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    new_text: String,
    font_size: f64,
) -> Result<(), String> {
    pdf::text_replace::replace_text_region(
        &PathBuf::from(path),
        page_index,
        x,
        y,
        w,
        h,
        &new_text,
        font_size,
    )
}

#[tauri::command]
fn get_page_text_lines(path: String, page_index: u32) -> Result<Vec<TextLineInfo>, String> {
    let path = PathBuf::from(path);
    let doc = lopdf::Document::load(&path).map_err(|e| e.to_string())?;
    let page_id = *doc.get_pages().get(&(page_index + 1)).ok_or_else(|| "Page not found".to_string())?;
    let lines = pdf::text_lines::decode_page_text_lines(&doc, page_id)?;
    let media = pdf::coords::page_media_box(&doc, page_id)?;
    let page_w = (media[2] - media[0]).max(1.0) as f32;
    let page_h = (media[3] - media[1]).max(1.0) as f32;

    let mut out = Vec::new();
    for line in lines {
        let [left, bottom, right, top] = line.bbox;
        let viewer = pdf::coords::pdf_rect_to_viewer_px(left, bottom, right, top, page_w, page_h);
        let pdf_font_name = pdf::fonts::page_font_name_for_resource(&doc, page_id, &line.font_name)
            .unwrap_or_else(|| line.font_name.clone());
        let (font_family, bold, italic) = pdf::fonts::editable_font_style(&pdf_font_name);
        out.push(TextLineInfo {
            text: line.text,
            x: viewer[0],
            y: viewer[1],
            w: (viewer[2] - viewer[0]).max(1.0),
            h: (viewer[3] - viewer[1]).max(1.0),
            font_family: font_family.to_string(),
            font_size: line.font_size,
            bold,
            italic,
        });
    }
    Ok(out)
}

#[tauri::command]
fn replace_text_line(
    path: String,
    page_index: u32,
    line_index: usize,
    new_text: String,
) -> Result<(), String> {
    pdf::text_replace::replace_text_line(
        &PathBuf::from(path),
        page_index,
        line_index,
        &new_text,
    )
}

#[cfg(test)]
mod ssrf_tests {
    use super::check_package_url;

    #[test]
    fn allows_github_release_hosts() {
        assert!(check_package_url("https://github.com/visorcraft/Kanoprii/releases/download/v1.6.0/foo.deb").is_ok());
        assert!(check_package_url("https://objects.githubusercontent.com/foo").is_ok());
        assert!(check_package_url("https://github-releases.githubusercontent.com/foo").is_ok());
        assert!(check_package_url("https://release-assets.githubusercontent.com/foo").is_ok());
    }

    #[test]
    fn rejects_non_https_scheme() {
        assert!(check_package_url("http://github.com/foo.deb").is_err());
        assert!(check_package_url("ftp://github.com/foo").is_err());
        assert!(check_package_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn rejects_ssrf_targets() {
        // Cloud metadata / loopback / private ranges — must all be rejected
        // even when reached via redirect (the helper is what stops that).
        assert!(check_package_url("https://169.254.169.254/latest/meta-data/").is_err());
        assert!(check_package_url("https://localhost/foo.deb").is_err());
        assert!(check_package_url("https://127.0.0.1/foo").is_err());
        assert!(check_package_url("https://10.0.0.1/foo").is_err());
        assert!(check_package_url("https://192.168.1.1/foo").is_err());
        assert!(check_package_url("https://attacker.example/foo").is_err());
    }

    #[test]
    fn rejects_non_default_port() {
        assert!(check_package_url("https://github.com:8443/foo").is_err());
        assert!(check_package_url("https://github.com:80/foo").is_err());
    }

    #[test]
    fn rejects_malformed_url() {
        assert!(check_package_url("https://").is_err());
        assert!(check_package_url("not a url").is_err());
    }

    #[test]
    fn host_check_is_case_insensitive() {
        assert!(check_package_url("https://GitHub.com/foo").is_ok());
        assert!(check_package_url("https://GitHub.COM/foo").is_ok());
    }
}
