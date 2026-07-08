import { Modal } from '../ui/Modal';
import { PageRangeFields } from '../pageRange/PageRangeFields';
import type { PageRangeController } from '../pageRange/usePageRange';

type FlattenModalProps = {
  range: PageRangeController;
  pageCount: number | null;
  onClose: () => void;
  onFlatten: () => void;
};

export function FlattenModal({ range, pageCount, onClose, onFlatten }: FlattenModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Flatten Annotations</h3>
      <p className="modal-help">Remove highlight, note, and other annotation objects from selected pages.</p>
      <PageRangeFields range={range} pageCount={pageCount} />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onFlatten()} className="btn">Flatten</button>
      </div>
    </Modal>
  );
}
