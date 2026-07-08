import { Modal } from '../ui/Modal';

type InsertImagePageModalProps = {
  atIndex: number;
  imagePath: string;
  pageCount: number | null;
  onAtIndexChange: (index: number) => void;
  onImagePathChange: (path: string) => void;
  onClose: () => void;
  onInsert: () => void;
};

export function InsertImagePageModal({
  atIndex,
  imagePath,
  pageCount,
  onAtIndexChange,
  onImagePathChange,
  onClose,
  onInsert,
}: InsertImagePageModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Insert Image Page</h3>
      <p className="modal-help">Add a new page with a centered image (JPEG/PNG/WebP).</p>
      <label>Insert at position (1-{(pageCount ?? 0) + 1}):</label>
      <input
        type="number"
        value={atIndex + 1}
        onChange={(e) => onAtIndexChange(Math.max(0, parseInt(e.target.value, 10) - 1))}
        min="1"
        max={(pageCount ?? 0) + 1}
        className="modal-input"
      />
      <label>Image file path:</label>
      <input
        type="text"
        value={imagePath}
        onChange={(e) => onImagePathChange(e.target.value)}
        className="modal-input"
        placeholder="/path/to/image.jpg"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onInsert()} className="btn" disabled={!imagePath.trim()}>Insert</button>
      </div>
    </Modal>
  );
}
