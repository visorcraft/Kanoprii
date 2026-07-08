import { Modal } from '../ui/Modal';

type SignPdfModalProps = {
  certPath: string;
  certPassword: string;
  reason: string;
  location: string;
  nativeDialogs: boolean;
  onCertPathChange: (path: string) => void;
  onCertPasswordChange: (value: string) => void;
  onReasonChange: (value: string) => void;
  onLocationChange: (value: string) => void;
  onClose: () => void;
  onChooseCertNative: () => void;
  onSign: () => void;
};

export function SignPdfModal({
  certPath,
  certPassword,
  reason,
  location,
  nativeDialogs,
  onCertPathChange,
  onCertPasswordChange,
  onReasonChange,
  onLocationChange,
  onClose,
  onChooseCertNative,
  onSign,
}: SignPdfModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Digital signature</h3>
      <p className="modal-help">
        Sign the open document with a PKCS#12 identity (.p12/.pfx). The signature is embedded in the working copy; use Save to write it to your file.
      </p>
      <label>Certificate (.p12 / .pfx):</label>
      <div className="modal-path-row">
        <input
          type="text"
          value={certPath}
          onChange={(e) => onCertPathChange(e.target.value)}
          className="modal-input"
          placeholder="/path/to/identity.p12"
        />
        {nativeDialogs && (
          <button type="button" onClick={() => void onChooseCertNative()} className="btn">Choose file…</button>
        )}
      </div>
      <label>Certificate password:</label>
      <input
        type="password"
        value={certPassword}
        onChange={(e) => onCertPasswordChange(e.target.value)}
        className="modal-input"
      />
      <label>Reason (optional):</label>
      <input
        type="text"
        value={reason}
        onChange={(e) => onReasonChange(e.target.value)}
        className="modal-input"
        placeholder="Approved"
      />
      <label>Location (optional):</label>
      <input
        type="text"
        value={location}
        onChange={(e) => onLocationChange(e.target.value)}
        className="modal-input"
        placeholder="Office"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button
          onClick={() => void onSign()}
          className="btn"
          disabled={!certPath.trim() || !certPassword}
        >
          Sign PDF
        </button>
      </div>
    </Modal>
  );
}
