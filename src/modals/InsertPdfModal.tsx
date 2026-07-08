import { PageRangePairInputs } from '../pageRange/PageRangeFields';
import { Modal } from '../ui/Modal';

type InsertPdfModalProps = {
  sourcePath: string;
  sourcePageCount: number | null;
  pageCount: number | null;
  insertAtPage: number;
  startPage: number;
  endPage: number;
  nativeDialogs: boolean;
  onSourcePathChange: (path: string) => void;
  onInsertAtPageChange: (page: number) => void;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onClose: () => void;
  onChooseNative: () => void;
  onBrowse: () => void;
  onInsert: () => void;
};

export function InsertPdfModal({
  sourcePath,
  sourcePageCount,
  pageCount,
  insertAtPage,
  startPage,
  endPage,
  nativeDialogs,
  onSourcePathChange,
  onInsertAtPageChange,
  onStartChange,
  onEndChange,
  onClose,
  onChooseNative,
  onBrowse,
  onInsert,
}: InsertPdfModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Insert PDF</h3>
      <div className="insert-grid">
        <div className="insert-source">
          <label>Source PDF to insert:</label>
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
        <label>
          Insert at page (1-{(pageCount ?? 0) + 1}) of this document:
          <input
            type="number"
            value={insertAtPage + 1}
            onChange={(e) => onInsertAtPageChange(Math.max(0, parseInt(e.target.value, 10) - 1))}
            min="1"
            max={(pageCount ?? 0) + 1}
            className="modal-input"
          />
        </label>
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
          Inserts page{startPage === endPage ? '' : 's'} {startPage + 1}
          {startPage === endPage ? '' : `–${endPage + 1}`} of the source ({sourcePageCount} pages) at position {insertAtPage + 1} of this document.
        </p>
      ) : null}
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={onInsert} className="btn" disabled={!sourcePath.trim()}>Insert</button>
      </div>
    </Modal>
  );
}
