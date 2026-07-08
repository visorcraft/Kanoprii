use serde::{Deserialize, Serialize};
use std::process::Command;

const GPL_LICENSE_TEXT: &str = include_str!("../../LICENSE");
const THIRD_PARTY_LICENSES_TEXT: &str = include_str!("../../docs/credits-third-party.md");
const NPM_CREDITS_JSON: &str = include_str!("../../docs/credits-npm.json");
const CREDITS_TEXT: &str = include_str!("../../CREDITS.md");

const APACHE_2_0_TEXT: &str = include_str!("../../LICENSES/Apache-2.0.txt");
const BSD_3_CLAUSE_TEXT: &str = include_str!("../../LICENSES/BSD-3-Clause.txt");
const GPL_2_0_TEXT: &str = include_str!("../../LICENSES/GPL-2.0-or-later.txt");
const LGPL_2_1_TEXT: &str = include_str!("../../LICENSES/LGPL-2.1-or-later.txt");
const LGPL_3_0_TEXT: &str = include_str!("../../LICENSES/LGPL-3.0-only.txt");

struct RuntimeComponent {
    name: &'static str,
    url: &'static str,
    license_display: &'static str,
    spdx_ids: &'static [&'static str],
}

const RUNTIME_COMPONENTS: &[RuntimeComponent] = &[
    RuntimeComponent {
        name: "PDFium",
        url: "https://pdfium.googlesource.com/pdfium/",
        license_display: "BSD-3-Clause",
        spdx_ids: &["BSD-3-Clause"],
    },
    RuntimeComponent {
        name: "WebKitGTK",
        url: "https://webkitgtk.org/",
        license_display: "LGPL-2.1-or-later",
        spdx_ids: &["LGPL-2.1-or-later"],
    },
    RuntimeComponent {
        name: "GTK 3",
        url: "https://www.gtk.org/",
        license_display: "LGPL-2.1-or-later",
        spdx_ids: &["LGPL-2.1-or-later"],
    },
    RuntimeComponent {
        name: "CUPS",
        url: "https://openprinting.github.io/cups/",
        license_display: "Apache-2.0",
        spdx_ids: &["Apache-2.0"],
    },
    RuntimeComponent {
        name: "Tesseract OCR (optional)",
        url: "https://github.com/tesseract-ocr/tesseract",
        license_display: "Apache-2.0",
        spdx_ids: &["Apache-2.0"],
    },
];

#[derive(Debug, Eq, PartialEq)]
struct ThirdPartyCredit {
    name: String,
    version: String,
    license: String,
    url: String,
}

fn runtime_license_body(spdx: &str) -> &'static str {
    match spdx {
        "Apache-2.0" => APACHE_2_0_TEXT,
        "BSD-3-Clause" => BSD_3_CLAUSE_TEXT,
        "GPL-2.0-or-later" => GPL_2_0_TEXT,
        "LGPL-2.1-or-later" => LGPL_2_1_TEXT,
        "LGPL-3.0-only" => LGPL_3_0_TEXT,
        _ => "",
    }
}

fn build_runtime_licenses_text() -> String {
    const RULE: &str = "================================================================";

    let mut out = String::from(
        "These are the full license texts for the system and runtime components \
that Kanoprii links against or bundles in packaged builds.\n\n",
    );

    for spdx in ["BSD-3-Clause", "LGPL-2.1-or-later", "Apache-2.0", "GPL-2.0-or-later", "LGPL-3.0-only"] {
        let body = runtime_license_body(spdx);
        if body.is_empty() {
            continue;
        }

        let applies_to: Vec<&str> = RUNTIME_COMPONENTS
            .iter()
            .filter(|component| component.spdx_ids.contains(&spdx))
            .map(|component| component.name)
            .collect();
        if applies_to.is_empty() {
            continue;
        }

        out.push_str(RULE);
        out.push('\n');
        out.push_str(spdx);
        out.push_str(" - applies to: ");
        out.push_str(&applies_to.join(", "));
        out.push('\n');
        out.push_str(RULE);
        out.push_str("\n\n");
        out.push_str(body);
        out.push_str("\n\n");
    }

    out.push_str(RULE);
    out.push('\n');
    out.push_str("Runtime component index\n");
    out.push_str(RULE);
    out.push('\n');
    for component in RUNTIME_COMPONENTS {
        out.push_str(&format!("- {} - {} - {}\n", component.name, component.license_display, component.url));
    }

    out
}

fn license_section_to_spdx(title: &str) -> String {
    match title {
        "Apache License 2.0" => "Apache-2.0".to_owned(),
        "BSD 2-Clause \"Simplified\" License" => "BSD-2-Clause".to_owned(),
        "BSD 3-Clause \"New\" or \"Revised\" License" => "BSD-3-Clause".to_owned(),
        "BSD Zero Clause License" => "0BSD".to_owned(),
        "Community Data License Agreement Permissive 2.0" => "CDLA-Permissive-2.0".to_owned(),
        "GNU General Public License v3.0 only" => "GPL-3.0-only".to_owned(),
        "ISC License" => "ISC".to_owned(),
        "MIT License" => "MIT".to_owned(),
        "MIT OR Apache-2.0" => "MIT OR Apache-2.0".to_owned(),
        "Mozilla Public License 2.0" => "MPL-2.0".to_owned(),
        "Unicode License v3" => "Unicode-3.0".to_owned(),
        "University of Illinois/NCSA Open Source License" => "NCSA".to_owned(),
        "zlib License" => "Zlib".to_owned(),
        other => other.replace("&quot;", "\"").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">"),
    }
}

fn parse_used_by_line(line: &str, license: &str) -> Option<ThirdPartyCredit> {
    let body = line.strip_prefix("- [`")?;
    let (label, rest) = body.split_once("`](")?;
    let url = rest.strip_suffix(')')?;
    let (name, version) = label.rsplit_once(' ')?;

    Some(ThirdPartyCredit {
        name: name.to_owned(),
        version: version.to_owned(),
        license: license.to_owned(),
        url: url.to_owned(),
    })
}

fn rust_credit_entries(text: &str) -> Vec<ThirdPartyCredit> {
    let mut current_license = String::new();
    let mut in_license_texts = false;
    let mut in_used_by = false;
    let mut entries = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed == "## npm License Texts" || trimmed == "# Shipped npm packages" {
            break;
        }

        if trimmed == "## License Texts" {
            in_license_texts = true;
            continue;
        }
        if !in_license_texts {
            continue;
        }

        if let Some(title) = trimmed.strip_prefix("### ") {
            current_license = license_section_to_spdx(title);
            in_used_by = false;
            continue;
        }

        if trimmed == "Used by:" {
            in_used_by = !current_license.is_empty();
            continue;
        }

        if !in_used_by {
            continue;
        }

        if trimmed.starts_with("```") || trimmed == "---" {
            in_used_by = false;
            continue;
        }

        if let Some(entry) = parse_used_by_line(trimmed, &current_license) {
            entries.push(entry);
        }
    }

    entries.sort_by(|a, b| {
        a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)).then_with(|| a.license.cmp(&b.license))
    });
    entries
}

fn npm_credit_rows() -> Vec<ThirdPartyCreditRow> {
    serde_json::from_str(NPM_CREDITS_JSON).unwrap_or_default()
}

#[derive(Serialize)]
pub struct LicenseDocuments {
    pub gpl: String,
    pub third_party: String,
    pub credits: String,
    pub runtime: String,
}

#[derive(Serialize, Deserialize)]
pub struct ThirdPartyCreditRow {
    pub name: String,
    pub version: String,
    pub license: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct RuntimeComponentRow {
    pub name: String,
    pub licenses: String,
    pub url: String,
    pub spdx: Vec<String>,
}

#[derive(Serialize)]
pub struct CreditsCatalog {
    pub crates: Vec<ThirdPartyCreditRow>,
    pub npm_packages: Vec<ThirdPartyCreditRow>,
    pub runtime_components: Vec<RuntimeComponentRow>,
}

#[tauri::command]
pub fn license_documents() -> LicenseDocuments {
    LicenseDocuments {
        gpl: GPL_LICENSE_TEXT.to_string(),
        third_party: THIRD_PARTY_LICENSES_TEXT.to_string(),
        credits: CREDITS_TEXT.to_string(),
        runtime: build_runtime_licenses_text(),
    }
}

#[tauri::command]
pub fn credits_catalog() -> CreditsCatalog {
    let crates = rust_credit_entries(THIRD_PARTY_LICENSES_TEXT)
        .into_iter()
        .map(|entry| ThirdPartyCreditRow {
            name: entry.name,
            version: entry.version,
            license: entry.license,
            url: entry.url,
        })
        .collect();

    let runtime_components = RUNTIME_COMPONENTS
        .iter()
        .map(|component| RuntimeComponentRow {
            name: component.name.to_string(),
            licenses: component.license_display.to_string(),
            url: component.url.to_string(),
            spdx: component.spdx_ids.iter().map(|id| (*id).to_string()).collect(),
        })
        .collect();

    CreditsCatalog { crates, npm_packages: npm_credit_rows(), runtime_components }
}

#[tauri::command]
pub fn runtime_license_text(spdx_id: String) -> String {
    runtime_license_body(&spdx_id).to_string()
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Only http(s) URLs are supported.".into());
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(&url).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(&url).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", "", &url]).spawn().map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_license_texts_are_present() {
        assert!(GPL_LICENSE_TEXT.contains("GNU GENERAL PUBLIC LICENSE"));
        assert!(THIRD_PARTY_LICENSES_TEXT.contains("Third-Party Licenses"));
        assert!(CREDITS_TEXT.contains("Kanoprii"));
    }

    #[test]
    fn runtime_licenses_text_has_all_sections() {
        let text = build_runtime_licenses_text();
        assert!(text.contains("BSD-3-Clause"));
        assert!(text.contains("PDFium"));
        assert!(text.contains("WebKitGTK"));
        assert!(text.contains("Tesseract OCR"));
    }

    #[test]
    fn rust_credit_entries_parse_bundled_markdown() {
        let entries = rust_credit_entries(THIRD_PARTY_LICENSES_TEXT);
        assert!(entries.len() > 200);
        assert!(entries
            .iter()
            .any(|entry| { entry.name == "adler2" && entry.version == "2.0.1" && entry.license == "0BSD" }));
        assert!(entries
            .iter()
            .any(|entry| { entry.name == "lopdf" && entry.license == "MIT" && entry.url.contains("lopdf") }));
        assert!(!entries.iter().any(|entry| entry.name == "react"));
    }

    #[test]
    fn npm_credit_rows_include_shipped_frontend_packages() {
        let rows = npm_credit_rows();
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().any(|row| row.name == "react" && row.license == "MIT"));
        assert!(rows.iter().any(|row| row.name == "@tauri-apps/api"));
        assert!(rows.iter().any(|row| row.name == "@tauri-apps/plugin-updater"));
        assert!(rows.iter().any(|row| row.name == "@tauri-apps/plugin-process"));
    }

    #[test]
    fn third_party_bundle_includes_npm_license_texts() {
        assert!(THIRD_PARTY_LICENSES_TEXT.contains("## npm License Texts"));
        assert!(THIRD_PARTY_LICENSES_TEXT.contains("react 19."));
    }

    #[test]
    fn credits_catalog_is_frontend_ready() {
        let catalog = credits_catalog();
        assert!(catalog.crates.len() > 200);
        assert_eq!(catalog.npm_packages.len(), 6);
        assert_eq!(catalog.runtime_components.len(), RUNTIME_COMPONENTS.len());
        for row in &catalog.runtime_components {
            assert!(!row.name.is_empty());
            assert!(!row.licenses.is_empty());
            assert!(!row.url.is_empty());
            assert!(!row.spdx.is_empty());
        }
    }

    #[test]
    fn every_runtime_component_spdx_resolves() {
        for component in RUNTIME_COMPONENTS {
            for spdx in component.spdx_ids {
                assert!(
                    !runtime_license_body(spdx).is_empty(),
                    "{} references unresolved spdx id {spdx}",
                    component.name
                );
            }
        }
    }

    #[test]
    fn open_external_url_rejects_non_http() {
        assert!(open_external_url("file:///etc/passwd".into()).is_err());
    }
}
