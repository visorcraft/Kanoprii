import { useId } from 'react';
import { Modal } from '../ui/Modal';

type ExtractEvenPagesModalProps = {
  outputPath: string;
  onOutputPathChange: (path: string) => void;
  onClose: () => void;
  onExtract: () => void;
};

export function ExtractEvenPagesModal({
  outputPath,
  onOutputPathChange,
  onClose,
  onExtract,
}: ExtractEvenPagesModalProps) {
  const outputId = useId();
  const disabled = !outputPath.trim();

  return (
    <Modal onClose={onClose}>
      <h3>Extract Even Pages</h3>
      <p className="modal-help">Save pages 2, 4, 6, … to a new PDF. The open document is not modified.</p>
      <label htmlFor={outputId}>Output path:</label>
      <input
        id={outputId}
        type="text"
        value={outputPath}
        onChange={(e) => onOutputPathChange(e.target.value)}
        className="modal-input"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onExtract()} className="btn" disabled={disabled}>Extract</button>
      </div>
    </Modal>
  );
}
