import { Modal } from '../ui/Modal';

type DeleteNthModalProps = {
  nth: number;
  onNthChange: (nth: number) => void;
  onClose: () => void;
  onDelete: () => void;
};

export function DeleteNthModal({
  nth,
  onNthChange,
  onClose,
  onDelete,
}: DeleteNthModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Delete Every Nth Page</h3>
      <p className="modal-help">Delete pages n, 2n, 3n, … (1-based). At least one page is always kept.</p>
      <label>N (≥ 2):</label>
      <input
        type="number"
        value={nth}
        onChange={(e) => onNthChange(Math.max(2, parseInt(e.target.value, 10) || 2))}
        min="2"
        className="modal-input"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onDelete()} className="btn btn-danger">Delete</button>
      </div>
    </Modal>
  );
}
