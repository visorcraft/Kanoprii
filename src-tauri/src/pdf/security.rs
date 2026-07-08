use lopdf::{Document, EncryptionState, EncryptionVersion, Object, Permissions};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use underskrift::inspect::signatures::inspect_signatures;
use underskrift::trust::{TrustStore, TrustStoreSet};
use underskrift::verify::report::SignatureStatus;
use underskrift::verify::SignatureVerifier;
use underskrift::{PdfSigner, SigningOptions, SoftwareSigner, SubFilter};

pub fn is_encrypted(path: &Path) -> Result<bool, String> {
    let path = path.to_path_buf();
    match Document::load(&path) {
        Ok(doc) => Ok(doc.is_encrypted()),
        Err(lopdf::Error::InvalidPassword) => Ok(true),
        Err(lopdf::Error::Unimplemented(_)) => Ok(true),
        Err(e) => Err(e.to_string()),
    }
}

/// Verify that `password` unlocks an encrypted PDF.
pub fn pdf_is_encrypted(path: String) -> Result<bool, String> {
    is_encrypted(&PathBuf::from(path))
}

/// Verify that `password` unlocks an encrypted PDF.
pub fn verify_pdf_password(path: String, password: String) -> Result<(), String> {
    Document::load_with_password(PathBuf::from(&path), &password).map_err(|_| "Incorrect password".to_string())?;
    Ok(())
}

/// Copy an encrypted PDF into a decrypted working copy for editing.
pub fn open_working_copy_with_password(original: String, password: String) -> Result<String, String> {
    let original = PathBuf::from(&original);
    let mut doc = Document::load_with_password(&original, &password).map_err(|e| e.to_string())?;
    if doc.is_encrypted() {
        doc.decrypt(&password).map_err(|e| e.to_string())?;
    }
    let stem = original.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
    let working = std::env::temp_dir().join(format!("kanoprii_work_{}_{}.pdf", std::process::id(), stem));
    crate::pdf::io::save_atomic(&mut doc, &working)?;
    Ok(working.to_string_lossy().into_owned())
}

pub fn ensure_pdf_file_id(doc: &mut Document) {
    if doc.trailer.get(b"ID").is_ok() {
        return;
    }
    let id = vec![0xA1u8; 16];
    doc.trailer.set(
        b"ID",
        Object::Array(vec![
            Object::String(id.clone(), lopdf::StringFormat::Hexadecimal),
            Object::String(id, lopdf::StringFormat::Hexadecimal),
        ]),
    );
}

/// Write a password-protected sibling `<stem>_protected.pdf` next to `path`.
pub fn protect_pdf(path: String, user_password: String, owner_password: Option<String>) -> Result<String, String> {
    if user_password.is_empty() {
        return Err("User password is required".to_string());
    }
    let path = PathBuf::from(&path);
    let mut doc = Document::load(&path).map_err(|e| e.to_string())?;
    if doc.is_encrypted() {
        return Err("PDF is already encrypted".to_string());
    }
    ensure_pdf_file_id(&mut doc);

    let owner = owner_password.filter(|value| !value.is_empty()).unwrap_or_else(|| user_password.clone());
    let version = EncryptionVersion::V2 {
        document: &doc,
        owner_password: &owner,
        user_password: &user_password,
        key_length: 128,
        permissions: Permissions::all(),
    };
    let state = EncryptionState::try_from(version).map_err(|e| e.to_string())?;
    doc.encrypt(&state).map_err(|e| e.to_string())?;

    let output_path = path.with_file_name(format!(
        "{}_protected.pdf",
        path.file_stem().unwrap_or_else(|| std::ffi::OsStr::new("document")).to_string_lossy()
    ));
    crate::pdf::io::save_atomic(&mut doc, &output_path)?;

    Ok(format!(
        "Saved encrypted PDF to {}. Open it with the user password you set.",
        output_path.file_name().unwrap().to_string_lossy()
    ))
}

#[derive(Debug, Clone, Serialize)]
pub struct PdfSignatureInfo {
    pub field_name: String,
    pub signer_name: Option<String>,
    pub reason: Option<String>,
    pub location: Option<String>,
    pub signing_time: Option<String>,
    pub sub_filter: Option<String>,
    pub signed_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PdfSignatureVerificationEntry {
    pub field_name: String,
    pub status: String,
    pub signer_name: Option<String>,
    pub signing_time: Option<String>,
    pub integrity_ok: bool,
    pub modifications_after_signing: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PdfSignatureVerificationSummary {
    pub signature_count: usize,
    pub valid_count: usize,
    pub invalid_count: usize,
    pub document_modified: bool,
    pub overall_valid: bool,
    pub summary: String,
    pub signatures: Vec<PdfSignatureVerificationEntry>,
}

pub fn pdf_sign_runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Runtime::new().expect("tokio runtime for PDF signing"))
}

pub fn read_pdf_bytes_for_signing(path: &Path) -> Result<Vec<u8>, String> {
    if is_encrypted(path)? {
        return Err("Cannot sign an encrypted PDF. Save an unencrypted copy first.".to_string());
    }
    fs::read(path).map_err(|e| e.to_string())
}

pub fn signature_info_from_field(field: &underskrift::inspect::signatures::SignatureFieldInfo) -> PdfSignatureInfo {
    let field_name = field.field_name.clone().unwrap_or_else(|| format!("Signature{}", field.obj_num.unwrap_or(0)));
    PdfSignatureInfo {
        field_name,
        signer_name: field.name.clone(),
        reason: field.reason.clone(),
        location: field.location.clone(),
        signing_time: field.signing_time.clone(),
        sub_filter: field.sub_filter.clone(),
        signed_percent: field.coverage.as_ref().map(|coverage| coverage.percentage),
    }
}

pub fn next_signature_field_name(inspection: &underskrift::inspect::signatures::PdfSignatureInspection) -> String {
    let mut index = 1u32;
    loop {
        let candidate = format!("Signature{index}");
        let taken = inspection.signatures.iter().any(|field| field.field_name.as_deref() == Some(candidate.as_str()));
        if !taken {
            return candidate;
        }
        index += 1;
    }
}

pub fn signature_status_label(status: &SignatureStatus) -> &'static str {
    match status {
        SignatureStatus::Valid => "valid",
        SignatureStatus::ValidButUntrusted => "valid_untrusted",
        SignatureStatus::Invalid => "invalid",
        SignatureStatus::Indeterminate => "indeterminate",
    }
}

pub fn build_trust_store_set(trust_pem_path: Option<&Path>) -> Result<TrustStoreSet, String> {
    let mut trust_set = TrustStoreSet::new();
    if let Some(path) = trust_pem_path {
        let store = TrustStore::from_pem_file(path).map_err(|e| e.to_string())?;
        trust_set = trust_set.with_sig_store(store);
    }
    Ok(trust_set)
}

/// List digital signature fields embedded in a PDF.
pub fn list_pdf_signatures(path: String) -> Result<Vec<PdfSignatureInfo>, String> {
    let path = PathBuf::from(path);
    let bytes = read_pdf_bytes_for_signing(&path)?;
    let inspection = inspect_signatures(&bytes).map_err(|e| e.to_string())?;
    Ok(inspection.signatures.iter().map(signature_info_from_field).collect())
}

/// Verify cryptographic integrity and certificate chains for all PDF signatures.
pub fn verify_pdf_signatures(
    path: String,
    trust_pem_path: Option<String>,
) -> Result<PdfSignatureVerificationSummary, String> {
    let path = PathBuf::from(path);
    let bytes = read_pdf_bytes_for_signing(&path)?;
    let inspection = inspect_signatures(&bytes).map_err(|e| e.to_string())?;
    if !inspection.has_signatures {
        return Ok(PdfSignatureVerificationSummary {
            signature_count: 0,
            valid_count: 0,
            invalid_count: 0,
            document_modified: false,
            overall_valid: false,
            summary: "No digital signatures found.".to_string(),
            signatures: vec![],
        });
    }
    let trust_path = trust_pem_path.map(PathBuf::from);
    let trust_set = build_trust_store_set(trust_path.as_deref())?;
    let verifier = SignatureVerifier::new(&trust_set);
    let report = verifier.verify_pdf(&bytes).map_err(|e| e.to_string())?;
    let signatures = report
        .signatures
        .iter()
        .map(|sig| PdfSignatureVerificationEntry {
            field_name: sig.field_name.clone(),
            status: signature_status_label(&sig.status).to_string(),
            signer_name: sig.signer_name.clone(),
            signing_time: sig.signing_time.clone(),
            integrity_ok: sig.integrity_ok,
            modifications_after_signing: sig.modifications_after_signing,
            summary: sig.summary.clone(),
        })
        .collect();
    Ok(PdfSignatureVerificationSummary {
        signature_count: report.signatures.len(),
        valid_count: report.valid_count,
        invalid_count: report.invalid_count,
        document_modified: report.document_modified,
        overall_valid: report.all_valid(),
        summary: report.summary,
        signatures,
    })
}

/// Digitally sign a PDF with a PKCS#12 (.p12/.pfx) identity. Writes back to `path`
/// unless `output_path` is set.
pub fn sign_pdf(
    path: String,
    cert_path: String,
    cert_password: String,
    reason: Option<String>,
    location: Option<String>,
    field_name: Option<String>,
    output_path: Option<String>,
) -> Result<String, String> {
    if cert_password.is_empty() {
        return Err("Certificate password is required".to_string());
    }
    let path = PathBuf::from(path);
    let pdf_bytes = read_pdf_bytes_for_signing(&path)?;
    let cert_path = PathBuf::from(cert_path);
    if !cert_path.is_file() {
        return Err("Certificate file not found".to_string());
    }
    let cert_bytes = fs::read(&cert_path).map_err(|e| e.to_string())?;
    let signer = SoftwareSigner::from_pkcs12_data(&cert_bytes, &cert_password).map_err(|e| e.to_string())?;
    let inspection = inspect_signatures(&pdf_bytes).map_err(|e| e.to_string())?;
    let field =
        field_name.filter(|value| !value.trim().is_empty()).unwrap_or_else(|| next_signature_field_name(&inspection));
    let options = SigningOptions {
        sub_filter: SubFilter::Pades,
        field_name: field,
        reason: reason.filter(|value| !value.trim().is_empty()),
        location: location.filter(|value| !value.trim().is_empty()),
        ..Default::default()
    };
    let signed = pdf_sign_runtime()
        .block_on(PdfSigner::new().options(options).sign(&pdf_bytes, &signer))
        .map_err(|e| e.to_string())?;
    let output = output_path.map(PathBuf::from).unwrap_or(path);
    // ponytail: atomic write — a crash mid-fs::write would otherwise leave
    // the file truncated (and invalidate the signature). Write bytes to a
    // temp file then rename.
    let parent = output.parent().unwrap_or_else(|| std::path::Path::new("."));
    let temp = parent.join(format!(
        ".{}.{}.{}.tmp",
        output.file_name().and_then(|n| n.to_str()).unwrap_or("out"),
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0)
    ));
    std::fs::write(&temp, &signed).map_err(|e| e.to_string())?;
    if let Err(e) = crate::pdf::io::save_atomic_overwrite(&temp, &output) {
        let _ = std::fs::remove_file(&temp);
        return Err(format!("Failed to replace {}: {}", output.display(), e));
    }
    Ok(format!("Signed PDF saved to {}", output.file_name().unwrap_or_default().to_string_lossy()))
}

pub fn remove_pdf_password(path: String, password: String) -> Result<String, String> {
    if password.is_empty() {
        return Err("Password is required".to_string());
    }
    let path = PathBuf::from(&path);
    let mut doc = Document::load_with_password(&path, &password).map_err(|_| "Incorrect password".to_string())?;
    if doc.is_encrypted() {
        doc.decrypt(&password).map_err(|e| e.to_string())?;
    }
    let output_path = path.with_file_name(format!(
        "{}_decrypted.pdf",
        path.file_stem().unwrap_or_else(|| std::ffi::OsStr::new("document")).to_string_lossy()
    ));
    crate::pdf::io::save_atomic(&mut doc, &output_path)?;
    Ok(output_path.to_string_lossy().into_owned())
}
