import type { KeyboardEvent } from 'react';
import { Modal } from '../ui/Modal';

type DeletePageModalProps = {
  deletePageInput: string;
  currentPage: number;
  pageCount: number;
  onDeletePageInputChange: (value: string) => void;
  onClose: () => void;
  onDelete: () => void;
};

export function DeletePageModal({
  deletePageInput,
  currentPage,
  pageCount,
  onDeletePageInputChange,
  onClose,
  onDelete,
}: DeletePageModalProps) {
  const onFieldKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      onDelete();
      e.currentTarget.blur();
    }
  };

  return (
    <Modal onClose={onClose}>
      <h3>Delete Page</h3>
      <p className="modal-help">
        Choose the page to remove. This edits the open PDF file on disk.
      </p>
      <label>Page to delete:</label>
      <input
        type="number"
        value={deletePageInput}
        onChange={(e) => onDeletePageInputChange(e.target.value)}
        onKeyDown={onFieldKeyDown}
        className="modal-input"
        min="1"
        max={pageCount}
        autoFocus
      />
      <p className="muted">Current page: {currentPage + 1} / {pageCount}</p>
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={onDelete} className="btn btn-danger">Delete page</button>
      </div>
    </Modal>
  );
}
