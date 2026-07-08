import { Modal } from '../ui/Modal';
import { PageRangePairInputs } from '../pageRange/PageRangeFields';

type InterleaveModalProps = {
  sourcePath: string;
  sourcePageCount: number | null;
  startPage: number;
  endPage: number;
  onSourcePathChange: (path: string) => void;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onBrowse: () => void;
  onClose: () => void;
  onInterleave: () => void;
};

export function InterleaveModal({
  sourcePath,
  sourcePageCount,
  startPage,
  endPage,
  onSourcePathChange,
  onStartChange,
  onEndChange,
  onBrowse,
  onClose,
  onInterleave,
}: InterleaveModalProps) {
  const disabled = !sourcePath.trim();

  return (
    <Modal onClose={onClose}>
      <h3>Interleave PDF</h3>
      <p className="modal-help">Alternate pages: A0, B0, A1, B1, … from the source range.</p>
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
        <button onClick={() => void onInterleave()} className="btn" disabled={disabled}>Interleave</button>
      </div>
    </Modal>
  );
}
