import { Modal } from '../ui/Modal';

type ExportPagePdfModalProps = {
  currentPage: number;
  outputPath: string;
  onOutputPathChange: (path: string) => void;
  onClose: () => void;
  onExport: () => void;
};

export function ExportPagePdfModal({
  currentPage,
  outputPath,
  onOutputPathChange,
  onClose,
  onExport,
}: ExportPagePdfModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Export Page {currentPage + 1} as PDF</h3>
      <p className="modal-help">Save only the current page to a new PDF. The open document is not modified.</p>
      <label>Output PDF path:</label>
      <input
        type="text"
        value={outputPath}
        onChange={(e) => onOutputPathChange(e.target.value)}
        className="modal-input"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onExport()} className="btn" disabled={!outputPath.trim()}>Export</button>
      </div>
    </Modal>
  );
}
