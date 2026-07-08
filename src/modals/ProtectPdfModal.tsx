import { useId } from 'react';
import { Modal } from '../ui/Modal';

type ProtectPdfModalProps = {
  userPassword: string;
  userPasswordConfirm: string;
  ownerPassword: string;
  onUserPasswordChange: (value: string) => void;
  onUserPasswordConfirmChange: (value: string) => void;
  onOwnerPasswordChange: (value: string) => void;
  onClose: () => void;
  onProtect: () => void;
};

export function ProtectPdfModal({
  userPassword,
  userPasswordConfirm,
  ownerPassword,
  onUserPasswordChange,
  onUserPasswordConfirmChange,
  onOwnerPasswordChange,
  onClose,
  onProtect,
}: ProtectPdfModalProps) {
  const baseId = useId();
  const userId = `${baseId}-user`;
  const confirmId = `${baseId}-confirm`;
  const ownerId = `${baseId}-owner`;

  return (
    <Modal onClose={onClose}>
      <h3>Password protect</h3>
      <p className="modal-help">Writes an encrypted copy as <code>&lt;name&gt;_protected.pdf</code> beside the working file. The open document stays editable.</p>
      <label htmlFor={userId}>User password:</label>
      <input
        id={userId}
        type="password"
        value={userPassword}
        onChange={(e) => onUserPasswordChange(e.target.value)}
        className="modal-input"
      />
      <label htmlFor={confirmId}>Confirm user password:</label>
      <input
        id={confirmId}
        type="password"
        value={userPasswordConfirm}
        onChange={(e) => onUserPasswordConfirmChange(e.target.value)}
        className="modal-input"
      />
      <label htmlFor={ownerId}>Owner password (optional):</label>
      <input
        id={ownerId}
        type="password"
        value={ownerPassword}
        onChange={(e) => onOwnerPasswordChange(e.target.value)}
        className="modal-input"
        placeholder="Defaults to user password"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button
          onClick={() => void onProtect()}
          className="btn"
          disabled={!userPassword || !userPasswordConfirm}
        >
          Protect
        </button>
      </div>
    </Modal>
  );
}
