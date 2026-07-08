import { Modal } from '../ui/Modal';
import { PageRangePairInputs } from '../pageRange/PageRangeFields';

type PrependModalProps = {
  sourcePath: string;
  sourcePageCount: number | null;
  startPage: number;
  endPage: number;
  onSourcePathChange: (path: string) => void;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onBrowse: () => void;
  onClose: () => void;
  onPrepend: () => void;
};

export function PrependModal({
  sourcePath,
  sourcePageCount,
  startPage,
  endPage,
  onSourcePathChange,
  onStartChange,
  onEndChange,
  onBrowse,
  onClose,
  onPrepend,
}: PrependModalProps) {
  const disabled = !sourcePath.trim();

  return (
    <Modal onClose={onClose}>
      <h3>Prepend PDF</h3>
      <p className="modal-help">Insert pages from another PDF at the beginning of the document.</p>
      <label>Source PDF path:</label>
      <div className="modal-path-row">
        <input
          type="text"
          value={sourcePath}
          onChange={(e) => onSourcePathChange(e.target.value)}
          className="modal-input"
          placeholder="/path/to/source.pdf"
        />
        <button onClick={onBrowse} className="btn">Browse…</button>
      </div>
      {sourcePageCount !== null && (
        <PageRangePairInputs
          startPage={startPage}
          endPage={endPage}
          onStartChange={onStartChange}
          onEndChange={onEndChange}
          maxPage={sourcePageCount}
        />
      )}
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onPrepend()} className="btn" disabled={disabled}>Prepend</button>
      </div>
    </Modal>
  );
}
