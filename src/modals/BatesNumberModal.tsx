import { PageRangePairModal } from './PageRangePairModal';

const POSITIONS = [
  { value: 'footer-left', label: 'Footer left' },
  { value: 'footer-center', label: 'Footer center' },
  { value: 'footer-right', label: 'Footer right' },
  { value: 'header-right', label: 'Header right' },
] as const;

type BatesNumberModalProps = {
  startPage: number;
  endPage: number;
  pageCount: number | null;
  prefix: string;
  startNumber: number;
  digits: number;
  position: string;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onPrefixChange: (value: string) => void;
  onStartNumberChange: (value: number) => void;
  onDigitsChange: (value: number) => void;
  onPositionChange: (value: string) => void;
  onClose: () => void;
  onApply: () => void;
};

export function BatesNumberModal({
  startPage,
  endPage,
  pageCount,
  prefix,
  startNumber,
  digits,
  position,
  onStartChange,
  onEndChange,
  onPrefixChange,
  onStartNumberChange,
  onDigitsChange,
  onPositionChange,
  onClose,
  onApply,
}: BatesNumberModalProps) {
  return (
    <PageRangePairModal
      title="Bates Numbering"
      help="Stamp legal-style Bates numbers (prefix + zero-padded counter) on the working copy."
      startPage={startPage}
      endPage={endPage}
      pageCount={pageCount}
      onStartChange={onStartChange}
      onEndChange={onEndChange}
      onClose={onClose}
      actions={(
        <button type="button" onClick={() => void onApply()} className="btn">Apply</button>
      )}
    >
      <label>Prefix:</label>
      <input
        type="text"
        value={prefix}
        onChange={(e) => onPrefixChange(e.target.value)}
        className="modal-input"
        placeholder="e.g. ACME-"
      />
      <label>Start number:</label>
      <input
        type="number"
        min={0}
        value={startNumber}
        onChange={(e) => onStartNumberChange(parseInt(e.target.value, 10) || 0)}
        className="modal-input"
      />
      <label>Digits (zero padding):</label>
      <input
        type="number"
        min={1}
        max={12}
        value={digits}
        onChange={(e) => onDigitsChange(parseInt(e.target.value, 10) || 6)}
        className="modal-input"
      />
      <label>Position:</label>
      <select
        value={position}
        onChange={(e) => onPositionChange(e.target.value)}
        className="modal-input"
      >
        {POSITIONS.map((opt) => (
          <option key={opt.value} value={opt.value}>{opt.label}</option>
        ))}
      </select>
    </PageRangePairModal>
  );
}
