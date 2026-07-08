import { useId } from 'react';
import { Modal } from '../ui/Modal';

type ReplacePageModalProps = {
  currentPage: number;
  sourcePath: string;
  sourcePage: number;
  sourcePageCount: number | null;
  onSourcePathChange: (path: string) => void;
  onSourcePageChange: (page: number) => void;
  onBrowse: () => void;
  onClose: () => void;
  onReplace: () => void;
};

export function ReplacePageModal({
  currentPage,
  sourcePath,
  sourcePage,
  sourcePageCount,
  onSourcePathChange,
  onSourcePageChange,
  onBrowse,
  onClose,
  onReplace,
}: ReplacePageModalProps) {
  const baseId = useId();
  const sourcePathId = `${baseId}-source-path`;
  const sourcePageId = `${baseId}-source-page`;
  const disabled = !sourcePath.trim();

  return (
    <Modal onClose={onClose}>
      <h3>Replace Page {currentPage + 1}</h3>
      <p className="modal-help">Replace the current page with a deep-copied page from another PDF.</p>
      <label htmlFor={sourcePathId}>Source PDF path:</label>
      <div className="modal-path-row">
        <input
          id={sourcePathId}
          type="text"
          value={sourcePath}
          onChange={(e) => onSourcePathChange(e.target.value)}
          className="modal-input"
          placeholder="/path/to/source.pdf"
        />
        <button onClick={onBrowse} className="btn">Browse…</button>
      </div>
      {sourcePageCount !== null && (
        <label htmlFor={sourcePageId}>
          Source page (1-{sourcePageCount}):
          {' '}
          <input
            id={sourcePageId}
            type="number"
            value={sourcePage + 1}
            onChange={(e) => onSourcePageChange(Math.max(0, parseInt(e.target.value, 10) - 1))}
            min="1"
            max={sourcePageCount}
            className="modal-input"
          />
        </label>
      )}
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onReplace()} className="btn" disabled={disabled}>Replace</button>
      </div>
    </Modal>
  );
}
