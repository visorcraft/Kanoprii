import { useId } from 'react';
import { Modal } from '../ui/Modal';

type RenameBookmarkModalProps = {
  title: string;
  onTitleChange: (value: string) => void;
  onClose: () => void;
  onRename: () => void;
};

export function RenameBookmarkModal({
  title,
  onTitleChange,
  onClose,
  onRename,
}: RenameBookmarkModalProps) {
  const titleId = useId();

  return (
    <Modal onClose={onClose}>
      <h3>Rename Bookmark</h3>
      <label htmlFor={titleId}>Title:</label>
      <input
        id={titleId}
        type="text"
        value={title}
        onChange={(e) => onTitleChange(e.target.value)}
        className="modal-input"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onRename()} className="btn" disabled={!title.trim()}>Rename</button>
      </div>
    </Modal>
  );
}
