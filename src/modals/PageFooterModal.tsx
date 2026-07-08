import { ScopedPageActionModal } from './ScopedPageActionModal';
import type { PageRangeController } from '../pageRange/usePageRange';

type PageFooterModalProps = {
  range: PageRangeController;
  pageCount: number | null;
  text: string;
  onTextChange: (value: string) => void;
  onClose: () => void;
  onApply: () => void;
  onApplyOdd: () => void;
  onApplyEven: () => void;
};

export function PageFooterModal({
  range,
  pageCount,
  text,
  onTextChange,
  onClose,
  onApply,
  onApplyOdd,
  onApplyEven,
}: PageFooterModalProps) {
  const disabled = !text.trim();

  return (
    <ScopedPageActionModal
      title="Page Footer"
      help="Stamp footer text near the bottom of selected pages."
      range={range}
      pageCount={pageCount}
      onClose={onClose}
      onApply={onApply}
      onApplyOdd={onApplyOdd}
      onApplyEven={onApplyEven}
      applyDisabled={disabled}
    >
      <label>Footer text:</label>
      <input
        type="text"
        value={text}
        onChange={(e) => onTextChange(e.target.value)}
        className="modal-input"
      />
    </ScopedPageActionModal>
  );
}
