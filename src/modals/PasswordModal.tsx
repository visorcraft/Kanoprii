import type { KeyboardEvent } from 'react';
import { Modal } from '../ui/Modal';

type PasswordModalProps = {
  password: string;
  onPasswordChange: (value: string) => void;
  onClose: () => void;
  onOpen: () => void;
};

export function PasswordModal({
  password,
  onPasswordChange,
  onClose,
  onOpen,
}: PasswordModalProps) {
  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') void onOpen();
  };

  return (
    <Modal onClose={onClose}>
      <h3>Password required</h3>
      <p className="modal-help">This PDF is encrypted. Enter the user password to open it.</p>
      <label>Password:</label>
      <input
        type="password"
        value={password}
        onChange={(e) => onPasswordChange(e.target.value)}
        className="modal-input"
        onKeyDown={onKeyDown}
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onOpen()} className="btn" disabled={!password}>Open</button>
      </div>
    </Modal>
  );
}
