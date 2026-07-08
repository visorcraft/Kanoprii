use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub const OCR_RENDER_W: i32 = 1200;
pub const OCR_RENDER_H: i32 = 1697;

pub fn ocr_language() -> String {
    std::env::var("KANOPRII_OCR_LANG").unwrap_or_else(|_| "eng".into())
}

pub fn tessdata_prefix() -> Option<String> {
    std::env::var("KANOPRII_TESSDATA_PREFIX").ok().or_else(|| std::env::var("TESSDATA_PREFIX").ok())
}

pub fn ocr_page_segmentation_mode() -> u8 {
    std::env::var("KANOPRII_OCR_PSM")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .filter(|mode| *mode <= 13)
        .unwrap_or(1)
}

pub fn resolve_tesseract() -> Option<PathBuf> {
    if let Ok(cmd) = std::env::var("TESSERACT_CMD") {
        let path = PathBuf::from(cmd);
        if path.is_file() {
            return Some(path);
        }
    }
    let name = if cfg!(windows) { "tesseract.exe" } else { "tesseract" };
    std::env::var_os("PATH")
        .and_then(|paths| std::env::split_paths(&paths).map(|dir| dir.join(name)).find(|candidate| candidate.is_file()))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrStatus {
    pub available: bool,
    pub binary: Option<String>,
    pub language: String,
    pub tessdata_prefix: Option<String>,
    pub page_segmentation_mode: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TesseractInstallGuide {
    pub platform: String,
    pub summary: String,
    pub steps: Vec<String>,
    pub install_command: Option<String>,
    pub download_url: Option<String>,
    pub license_note: String,
}

pub fn os_release_value(contents: &str, key: &str) -> Option<String> {
    for line in contents.lines() {
        let (k, v) = line.split_once('=')?;
        if k.trim() == key {
            return Some(v.trim().trim_matches('"').to_lowercase());
        }
    }
    None
}

fn linux_tesseract_install() -> (Vec<String>, Option<String>) {
    let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let id = os_release_value(&os_release, "ID");
    let id_like = os_release_value(&os_release, "ID_LIKE").unwrap_or_default();
    let ids: Vec<&str> = id.iter().map(String::as_str).chain(id_like.split_whitespace()).collect();

    if ids.iter().any(|&id| matches!(id, "arch" | "cachyos" | "manjaro" | "endeavouros")) {
        return (
            vec![
                "Open a terminal.".into(),
                "Run the command below (includes English).".into(),
                "Restart Kanoprii.".into(),
            ],
            Some("sudo pacman -S tesseract tesseract-data-eng".into()),
        );
    }
    if ids.iter().any(|&id| matches!(id, "ubuntu" | "debian" | "pop" | "linuxmint" | "elementary")) {
        return (
            vec![
                "Open a terminal.".into(),
                "Run the command below (includes English).".into(),
                "Restart Kanoprii.".into(),
            ],
            Some("sudo apt install tesseract-ocr tesseract-ocr-eng".into()),
        );
    }
    if ids.iter().any(|&id| matches!(id, "fedora" | "rhel" | "centos" | "rocky" | "almalinux")) {
        return (
            vec![
                "Open a terminal.".into(),
                "Run the command below (includes English).".into(),
                "Restart Kanoprii.".into(),
            ],
            Some("sudo dnf install tesseract tesseract-langpack-eng".into()),
        );
    }
    if ids.iter().any(|&id| matches!(id, "opensuse-leap" | "opensuse-tumbleweed" | "suse")) {
        return (
            vec![
                "Open a terminal.".into(),
                "Run the command below (includes English).".into(),
                "Restart Kanoprii.".into(),
            ],
            Some("sudo zypper install tesseract tesseract-traineddata-english".into()),
        );
    }

    (
        vec![
            "Open a terminal.".into(),
            "Install Tesseract plus English using your package manager:".into(),
            "Arch / CachyOS: sudo pacman -S tesseract tesseract-data-eng".into(),
            "Ubuntu / Debian: sudo apt install tesseract-ocr tesseract-ocr-eng".into(),
            "Fedora: sudo dnf install tesseract tesseract-langpack-eng".into(),
            "Restart Kanoprii.".into(),
        ],
        None,
    )
}

pub fn build_tesseract_install_guide() -> TesseractInstallGuide {
    let license_note = "Tesseract is free, open-source software (Apache 2.0). You do not need to pay for it.".into();
    if cfg!(target_os = "macos") {
        return TesseractInstallGuide {
            platform: "macos".into(),
            summary: "Tesseract lets Kanoprii read text from scanned PDF pages. Normal PDFs with selectable text work without it.".into(),
            steps: vec![
                "Open Terminal.".into(),
                "Run the command below (Homebrew installs English by default).".into(),
                "Restart Kanoprii.".into(),
            ],
            install_command: Some("brew install tesseract".into()),
            download_url: None,
            license_note,
        };
    }
    if cfg!(target_os = "windows") {
        return TesseractInstallGuide {
            platform: "windows".into(),
            summary: "Tesseract lets Kanoprii read text from scanned PDF pages. Normal PDFs with selectable text work without it.".into(),
            steps: vec![
                "Download the Windows installer (link below).".into(),
                "Run the installer and keep the default English language pack.".into(),
                "Restart Kanoprii.".into(),
            ],
            install_command: None,
            download_url: Some("https://github.com/UB-Mannheim/tesseract/wiki".into()),
            license_note,
        };
    }

    let (steps, install_command) = linux_tesseract_install();
    TesseractInstallGuide {
        platform: "linux".into(),
        summary: "Tesseract lets Kanoprii read text from scanned PDF pages. Normal PDFs with selectable text work without it.".into(),
        steps,
        install_command,
        download_url: None,
        license_note,
    }
}

#[derive(Default, Debug, Clone)]
pub struct OcrExportStats {
    pub available: bool,
    pub language: String,
    pub scanned_pages: u32,
    pub sparse_supplements: u32,
    pub embedded_image_blocks: u32,
    pub embedded_form_blocks: u32,
    pub text_blocks: u32,
    pub missing_install_hints: u32,
}

pub fn ocr_missing_hint(context: &str) -> String {
    format!("_{context} - install Tesseract OCR (`tesseract` on PATH or `TESSERACT_CMD`) and language data (`KANOPRII_OCR_LANG`, default `eng`)._\n\n")
}

enum TesseractOutput {
    Text,
    Tsv,
}

fn run_tesseract_on_png(png: &[u8], output: TesseractOutput, fail_hard: bool) -> Result<Option<String>, String> {
    let tesseract = match resolve_tesseract() {
        Some(path) => path,
        None => return Ok(None),
    };

    let tmp_dir = std::env::temp_dir().join(format!(
        "kanoprii_ocr_{}_{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
    ));
    fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let image_path = tmp_dir.join("page.png");
    fs::write(&image_path, png).map_err(|e| e.to_string())?;

    let mut command = Command::new(&tesseract);
    command.arg(&image_path).arg("stdout");
    match output {
        TesseractOutput::Text => {
            command
                .arg("-l")
                .arg(ocr_language())
                .arg("--oem")
                .arg("1")
                .arg("--psm")
                .arg(ocr_page_segmentation_mode().to_string());
        }
        TesseractOutput::Tsv => {
            command.arg("tsv").arg("-l").arg(ocr_language());
        }
    }
    if let Some(prefix) = tessdata_prefix() {
        command.env("TESSDATA_PREFIX", prefix);
    }

    let output = command.output().map_err(|e| format!("failed to run {}: {e}", tesseract.display()))?;
    let _ = fs::remove_dir_all(&tmp_dir);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if fail_hard {
            return Err(format!("tesseract failed: {stderr}"));
        }
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

pub fn ocr_png_bytes(png: &[u8]) -> Result<Option<String>, String> {
    run_tesseract_on_png(png, TesseractOutput::Text, true)
}

pub fn try_ocr_png_bytes(png: &[u8]) -> Result<Option<String>, String> {
    run_tesseract_on_png(png, TesseractOutput::Text, false)
}

pub fn ocr_png_to_tsv(png: &[u8]) -> Result<String, String> {
    match run_tesseract_on_png(png, TesseractOutput::Tsv, true)? {
        Some(tsv) => Ok(tsv),
        None => Err(ocr_missing_hint("Make Searchable").trim().to_string()),
    }
}

pub fn ocr_status() -> OcrStatus {
    OcrStatus {
        available: resolve_tesseract().is_some(),
        binary: resolve_tesseract().map(|path| path.to_string_lossy().into_owned()),
        language: ocr_language(),
        tessdata_prefix: tessdata_prefix(),
        page_segmentation_mode: ocr_page_segmentation_mode(),
    }
}

pub fn ocr_available() -> bool {
    resolve_tesseract().is_some()
}

pub fn ocr_pdf_page_from_png(png: &[u8]) -> Result<String, String> {
    match ocr_png_bytes(png)? {
        Some(text) => Ok(text),
        None => Err("Tesseract OCR is not installed (set TESSERACT_CMD or add tesseract to PATH)".into()),
    }
}
