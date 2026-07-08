import { Modal } from '../ui/Modal';

type InsertBlankPagesModalProps = {
  atIndex: number;
  count: number;
  pageCount: number | null;
  onAtIndexChange: (index: number) => void;
  onCountChange: (count: number) => void;
  onClose: () => void;
  onInsert: () => void;
};

export function InsertBlankPagesModal({
  atIndex,
  count,
  pageCount,
  onAtIndexChange,
  onCountChange,
  onClose,
  onInsert,
}: InsertBlankPagesModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Insert Blank Pages</h3>
      <p className="modal-help">Insert multiple empty pages at once.</p>
      <label>Insert at position (1-{(pageCount ?? 0) + 1}):</label>
      <input
        type="number"
        value={atIndex + 1}
        onChange={(e) => onAtIndexChange(Math.max(0, parseInt(e.target.value, 10) - 1))}
        min="1"
        max={(pageCount ?? 0) + 1}
        className="modal-input"
      />
      <label>Number of pages:</label>
      <input
        type="number"
        value={count}
        onChange={(e) => onCountChange(Math.max(1, parseInt(e.target.value, 10) || 1))}
        min="1"
        className="modal-input"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onInsert()} className="btn">Insert</button>
      </div>
    </Modal>
  );
}
