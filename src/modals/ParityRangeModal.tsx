import { useId } from 'react';
import { PageRangePairInputs } from '../pageRange/PageRangeFields';
import { parityBatchNeedsRange } from '../pdf/parityPayload';
import { Modal } from '../ui/Modal';

type ParityRangeModalProps = {
  commands: readonly string[];
  command: string;
  outputPath: string;
  startPage: number;
  endPage: number;
  pageCount: number | null;
  onCommandChange: (command: string) => void;
  onOutputPathChange: (path: string) => void;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onClose: () => void;
  onRun: () => void;
};

export function ParityRangeModal({
  commands,
  command,
  outputPath,
  startPage,
  endPage,
  pageCount,
  onCommandChange,
  onOutputPathChange,
  onStartChange,
  onEndChange,
  onClose,
  onRun,
}: ParityRangeModalProps) {
  const baseId = useId();
  const actionId = `${baseId}-action`;
  const outputId = `${baseId}-output`;
  const needsRange = parityBatchNeedsRange(command);
  const needsOutput = command.startsWith('export_') || command.startsWith('extract_');

  return (
    <Modal onClose={onClose}>
      <h3>Parity Range Tools</h3>
      <p className="modal-help">Run parity actions within a page range, or document-wide mod-3/mod-4 filters (no range). Export/extract use the output path below; margin/text stamps use values from their respective modals.</p>
      {needsRange && (
        <PageRangePairInputs
          startPage={startPage}
          endPage={endPage}
          onStartChange={onStartChange}
          onEndChange={onEndChange}
          maxPage={pageCount ?? undefined}
        />
      )}
      <label htmlFor={actionId}>Action:</label>
      <select id={actionId} className="modal-input" value={command} onChange={(e) => onCommandChange(e.target.value)}>
        {commands.map((cmd) => (
          <option key={cmd} value={cmd}>{cmd.replaceAll('_', ' ')}</option>
        ))}
      </select>
      {needsOutput && (
        <>
          <label htmlFor={outputId}>{command.startsWith('extract_') ? 'Output PDF path:' : 'Output directory:'}</label>
          <input
            id={outputId}
            type="text"
            value={outputPath}
            onChange={(e) => onOutputPathChange(e.target.value)}
            className="modal-input"
            placeholder={command.startsWith('extract_') ? '/path/to/output.pdf' : '/path/to/output_dir'}
          />
        </>
      )}
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onRun()} className="btn">Run</button>
      </div>
    </Modal>
  );
}
