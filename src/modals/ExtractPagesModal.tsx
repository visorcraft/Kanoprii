import { PageRangePairInputs } from '../pageRange/PageRangeFields';
import { Modal } from '../ui/Modal';

type ExtractPagesModalProps = {
  startPage: number;
  endPage: number;
  pageCount: number | null;
  outputPath: string;
  nativeDialogs: boolean;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onOutputPathChange: (path: string) => void;
  onClose: () => void;
  onChooseOutputNative: () => void;
  onExtract: () => void;
};

export function ExtractPagesModal({
  startPage,
  endPage,
  pageCount,
  outputPath,
  nativeDialogs,
  onStartChange,
  onEndChange,
  onOutputPathChange,
  onClose,
  onChooseOutputNative,
  onExtract,
}: ExtractPagesModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Extract Pages</h3>
      <p className="modal-help">Save a page range from this document into a new PDF. The open file is not modified.</p>
      <PageRangePairInputs
        startPage={startPage}
        endPage={endPage}
        onStartChange={onStartChange}
        onEndChange={onEndChange}
        maxPage={pageCount ?? undefined}
      />
      <label>Output PDF path:</label>
      <div className="modal-path-row">
        <input
          type="text"
          value={outputPath}
          onChange={(e) => onOutputPathChange(e.target.value)}
          className="modal-input"
          placeholder="/path/to/output.pdf"
        />
        {nativeDialogs && (
          <button onClick={() => void onChooseOutputNative()} className="btn">Choose file…</button>
        )}
      </div>
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onExtract()} className="btn" disabled={!outputPath.trim()}>Extract</button>
      </div>
    </Modal>
  );
}
