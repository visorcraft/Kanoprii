import { ScopedPageActionModal } from './ScopedPageActionModal';
import type { PageRangeController } from '../pageRange/usePageRange';

type WatermarkModalProps = {
  range: PageRangeController;
  pageCount: number | null;
  text: string;
  onTextChange: (value: string) => void;
  onClose: () => void;
  onApply: () => void;
  onApplyOdd: () => void;
  onApplyEven: () => void;
};

export function WatermarkModal({
  range,
  pageCount,
  text,
  onTextChange,
  onClose,
  onApply,
  onApplyOdd,
  onApplyEven,
}: WatermarkModalProps) {
  const disabled = !text.trim();

  return (
    <ScopedPageActionModal
      title="Text Watermark"
      help="Add a diagonal watermark to the working copy."
      range={range}
      pageCount={pageCount}
      onClose={onClose}
      onApply={onApply}
      onApplyOdd={onApplyOdd}
      onApplyEven={onApplyEven}
      applyDisabled={disabled}
    >
      <label>Watermark text:</label>
      <input
        type="text"
        value={text}
        onChange={(e) => onTextChange(e.target.value)}
        className="modal-input"
      />
    </ScopedPageActionModal>
  );
}
