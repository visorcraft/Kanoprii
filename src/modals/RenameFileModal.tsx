import { useState } from 'react';
import { Modal } from '../ui/Modal';

type RenameFileModalProps = {
  /** Current base name without the `.pdf` extension. */
  currentName: string;
  onClose: () => void;
  onSubmit: (newName: string) => void;
};

export function RenameFileModal({ currentName, onClose, onSubmit }: RenameFileModalProps) {
  const [name, setName] = useState(currentName);
  const submit = () => {
    const trimmed = name.trim();
    if (trimmed) onSubmit(trimmed);
  };
  return (
    <Modal onClose={onClose} aria-label="Rename file" data-testid="rename-modal">
      <h3>Rename file</h3>
      <p className="modal-help">Renames the file on disk. The .pdf extension is kept.</p>
      <input
        className="modal-input"
        type="text"
        value={name}
        autoFocus
        onFocus={(e) => e.currentTarget.select()}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') submit();
        }}
        data-testid="rename-input"
      />
      <div className="modal-actions">
        <button type="button" className="btn btn-secondary" onClick={onClose}>
          Cancel
        </button>
        <button type="button" className="btn btn-active" onClick={submit} disabled={!name.trim()}>
          Rename
        </button>
      </div>
    </Modal>
  );
}
