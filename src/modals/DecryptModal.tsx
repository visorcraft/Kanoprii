import { Modal } from '../ui/Modal';

type DecryptModalProps = {
  password: string;
  onPasswordChange: (value: string) => void;
  onClose: () => void;
  onDecrypt: () => void;
};

export function DecryptModal({
  password,
  onPasswordChange,
  onClose,
  onDecrypt,
}: DecryptModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Decrypt PDF</h3>
      <p className="modal-help">Writes an unencrypted copy as <code>&lt;name&gt;_decrypted.pdf</code> beside the encrypted source (uses the original file path when available).</p>
      <label>Password:</label>
      <input
        type="password"
        value={password}
        onChange={(e) => onPasswordChange(e.target.value)}
        className="modal-input"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onDecrypt()} className="btn" disabled={!password}>Decrypt</button>
      </div>
    </Modal>
  );
}
