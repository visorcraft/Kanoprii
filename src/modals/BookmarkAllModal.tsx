import { Modal } from '../ui/Modal';

type BookmarkAllModalProps = {
  prefix: string;
  onPrefixChange: (value: string) => void;
  onClose: () => void;
  onBookmarkOdd: () => void;
  onBookmarkEven: () => void;
  onBookmarkAll: () => void;
};

export function BookmarkAllModal({
  prefix,
  onPrefixChange,
  onClose,
  onBookmarkOdd,
  onBookmarkEven,
  onBookmarkAll,
}: BookmarkAllModalProps) {
  const disabled = !prefix.trim();

  return (
    <Modal onClose={onClose}>
      <h3>Bookmark All Pages</h3>
      <p className="modal-help">Append an outline entry for every page.</p>
      <label>Title prefix:</label>
      <input
        type="text"
        value={prefix}
        onChange={(e) => onPrefixChange(e.target.value)}
        className="modal-input"
        placeholder="Page "
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onBookmarkOdd()} className="btn" disabled={disabled}>Bookmark Odd</button>
        <button onClick={() => void onBookmarkEven()} className="btn" disabled={disabled}>Bookmark Even</button>
        <button onClick={() => void onBookmarkAll()} className="btn" disabled={disabled}>Add all</button>
      </div>
    </Modal>
  );
}
