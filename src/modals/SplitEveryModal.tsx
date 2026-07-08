import { Modal } from '../ui/Modal';

type SplitEveryModalProps = {
  everyN: number;
  onEveryNChange: (n: number) => void;
  onClose: () => void;
  onSplit: () => void;
};

export function SplitEveryModal({
  everyN,
  onEveryNChange,
  onClose,
  onSplit,
}: SplitEveryModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Split Every N Pages</h3>
      <p className="modal-help">Write consecutive chunk files beside the open PDF. The working copy is not modified.</p>
      <label>Pages per file:</label>
      <input
        type="number"
        value={everyN}
        onChange={(e) => onEveryNChange(Math.max(1, parseInt(e.target.value, 10) || 1))}
        min="1"
        className="modal-input"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onSplit()} className="btn">Split</button>
      </div>
    </Modal>
  );
}
