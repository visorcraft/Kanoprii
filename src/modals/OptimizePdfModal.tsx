import { Modal } from '../ui/Modal';

type OptimizePdfModalProps = {
  originalFileName: string;
  onClose: () => void;
  onReplaceOriginal: () => void;
  onSaveAsNew: () => void;
};

export function OptimizePdfModal({
  originalFileName,
  onClose,
  onReplaceOriginal,
  onSaveAsNew,
}: OptimizePdfModalProps) {
  const name = originalFileName || 'this PDF';
  const siblingName = name.toLowerCase().endsWith('.pdf')
    ? `${name.slice(0, -4)}_optimized.pdf`
    : `${name}_optimized.pdf`;

  return (
    <Modal onClose={onClose} aria-label="Optimize PDF" data-testid="optimize-pdf-modal">
      <h3>Optimize PDF</h3>
      <p className="modal-help">
        Strip metadata, recompress images, prune unused objects, and compress streams.
      </p>
      <p className="modal-help">
        Replace overwrites <code>{name}</code> in place. Save as new writes{' '}
        <code>{siblingName}</code> next to it and leaves the original alone.
      </p>
      <div className="modal-actions">
        <button type="button" onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button
          type="button"
          onClick={() => void onSaveAsNew()}
          className="btn btn-secondary"
          data-testid="optimize-save-as"
        >
          Save as new file
        </button>
        <button
          type="button"
          onClick={() => void onReplaceOriginal()}
          className="btn"
          data-testid="optimize-replace"
        >
          Replace original
        </button>
      </div>
    </Modal>
  );
}
