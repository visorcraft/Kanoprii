import { PageRangePairInputs } from '../pageRange/PageRangeFields';
import { Modal } from '../ui/Modal';

type MergePdfModalProps = {
  sourcePath: string;
  sourcePageCount: number | null;
  pageCount: number | null;
  startPage: number;
  endPage: number;
  nativeDialogs: boolean;
  onSourcePathChange: (path: string) => void;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onClose: () => void;
  onChooseNative: () => void;
  onBrowse: () => void;
  onMerge: () => void;
};

export function MergePdfModal({
  sourcePath,
  sourcePageCount,
  pageCount,
  startPage,
  endPage,
  nativeDialogs,
  onSourcePathChange,
  onStartChange,
  onEndChange,
  onClose,
  onChooseNative,
  onBrowse,
  onMerge,
}: MergePdfModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Merge PDF</h3>
      <p className="modal-help">Append pages from another PDF to the end of this document.</p>
      <div className="insert-grid">
        <div className="insert-source">
          <label>Source PDF to merge:</label>
          <div className="modal-path-row">
            <input
              type="text"
              value={sourcePath}
              onChange={(e) => onSourcePathChange(e.target.value)}
              className="modal-input"
              placeholder="/path/to/source.pdf"
            />
            {nativeDialogs && (
              <button onClick={() => void onChooseNative()} className="btn">Choose file…</button>
            )}
            <button onClick={onBrowse} className="btn">Browse…</button>
          </div>
        </div>
        <PageRangePairInputs
          startPage={startPage}
          endPage={endPage}
          onStartChange={onStartChange}
          onEndChange={onEndChange}
          maxPage={sourcePageCount ?? undefined}
        />
      </div>
      {sourcePageCount ? (
        <p className="modal-help">
          Appends page{startPage === endPage ? '' : 's'} {startPage + 1}
          {startPage === endPage ? '' : `–${endPage + 1}`} of the source ({sourcePageCount} pages) after page {pageCount ?? 0} of this document.
        </p>
      ) : null}
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onMerge()} className="btn" disabled={!sourcePath.trim()}>Merge</button>
      </div>
    </Modal>
  );
}
