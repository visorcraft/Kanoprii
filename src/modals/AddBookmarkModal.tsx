import { Modal } from '../ui/Modal';

type AddBookmarkModalProps = {
  currentPage: number;
  title: string;
  onTitleChange: (value: string) => void;
  onClose: () => void;
  onAdd: () => void;
};

export function AddBookmarkModal({
  currentPage,
  title,
  onTitleChange,
  onClose,
  onAdd,
}: AddBookmarkModalProps) {
  const disabled = !title.trim();

  return (
    <Modal onClose={onClose}>
      <h3>Add Bookmark</h3>
      <p className="modal-help">Create an outline entry pointing at page {currentPage + 1}.</p>
      <label>Title:</label>
      <input
        type="text"
        value={title}
        onChange={(e) => onTitleChange(e.target.value)}
        className="modal-input"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onAdd()} className="btn" disabled={disabled}>Add</button>
      </div>
    </Modal>
  );
}
