import { useId } from 'react';
import { ScopedPageActionModal } from './ScopedPageActionModal';
import type { PageRangeController } from '../pageRange/usePageRange';

type ExportPagesPdfModalProps = {
  range: PageRangeController;
  pageCount: number | null;
  outputDir: string;
  onOutputDirChange: (value: string) => void;
  onClose: () => void;
  onExport: () => void;
  onExportOdd: () => void;
  onExportEven: () => void;
};

export function ExportPagesPdfModal({
  range,
  pageCount,
  outputDir,
  onOutputDirChange,
  onClose,
  onExport,
  onExportOdd,
  onExportEven,
}: ExportPagesPdfModalProps) {
  const outputId = useId();
  const disabled = !outputDir.trim();

  return (
    <ScopedPageActionModal
      title="Export Pages as PDF"
      help="Write each page as a separate single-page PDF. The open document is not modified."
      range={range}
      pageCount={pageCount}
      onClose={onClose}
      onApply={onExport}
      onApplyOdd={onExportOdd}
      onApplyEven={onExportEven}
      applyLabel="Export"
      oddLabel="Export Odd"
      evenLabel="Export Even"
      rangeApplyLabel="Pages to export:"
      applyDisabled={disabled}
      rangeFirst
    >
      <label htmlFor={outputId}>Output directory:</label>
      <input
        id={outputId}
        type="text"
        value={outputDir}
        onChange={(e) => onOutputDirChange(e.target.value)}
        className="modal-input"
        placeholder="/path/to/output_dir"
      />
      <p className="modal-help">Files are written as page-001.pdf, page-002.pdf, … inside the directory.</p>
    </ScopedPageActionModal>
  );
}
