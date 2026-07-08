import { Modal } from '../ui/Modal';

type SplitAtModalProps = {
  splitAtPage: number;
  pageCount: number | null;
  onSplitAtPageChange: (page: number) => void;
  onClose: () => void;
  onSplit: () => void;
};

export function SplitAtModal({
  splitAtPage,
  pageCount,
  onSplitAtPageChange,
  onClose,
  onSplit,
}: SplitAtModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Split At Page</h3>
      <p className="modal-help">Write `_part1.pdf` (pages before the split) and `_part2.pdf` (from the split page onward). The open document is not modified.</p>
      <label>Start of second file (page 2–{pageCount ?? 0}):</label>
      <input
        type="number"
        value={splitAtPage}
        onChange={(e) => onSplitAtPageChange(Math.max(2, parseInt(e.target.value, 10) || 2))}
        min="2"
        max={pageCount ?? undefined}
        className="modal-input"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onSplit()} className="btn">Split</button>
      </div>
    </Modal>
  );
}
