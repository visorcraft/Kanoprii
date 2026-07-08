import { ScopedPageActionModal } from './ScopedPageActionModal';
import type { PageRangeController } from '../pageRange/usePageRange';

type PageBorderModalProps = {
  range: PageRangeController;
  pageCount: number | null;
  inset: number;
  onInsetChange: (value: number) => void;
  onClose: () => void;
  onApply: () => void;
  onApplyOdd: () => void;
  onApplyEven: () => void;
};

export function PageBorderModal({
  range,
  pageCount,
  inset,
  onInsetChange,
  onClose,
  onApply,
  onApplyOdd,
  onApplyEven,
}: PageBorderModalProps) {
  return (
    <ScopedPageActionModal
      title="Page Border"
      help="Draw a rectangular border inset from page edges (viewer pixels)."
      range={range}
      pageCount={pageCount}
      onClose={onClose}
      onApply={onApply}
      onApplyOdd={onApplyOdd}
      onApplyEven={onApplyEven}
    >
      <label>
        Inset (px):
        {' '}
        <input
          type="number"
          value={inset}
          onChange={(e) => onInsetChange(Math.max(0, parseInt(e.target.value, 10) || 0))}
          min="0"
          className="modal-input"
        />
      </label>
    </ScopedPageActionModal>
  );
}
