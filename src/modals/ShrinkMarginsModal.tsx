import { ScopedPageActionModal } from './ScopedPageActionModal';
import { MarginQuadInputs, type MarginValues } from './MarginQuadInputs';
import type { PageRangeController } from '../pageRange/usePageRange';

type ShrinkMarginsModalProps = {
  range: PageRangeController;
  pageCount: number | null;
  margins: MarginValues;
  onMarginsChange: (margins: MarginValues) => void;
  onClose: () => void;
  onShrink: () => void;
  onShrinkOdd: () => void;
  onShrinkEven: () => void;
};

export function ShrinkMarginsModal({
  range,
  pageCount,
  margins,
  onMarginsChange,
  onClose,
  onShrink,
  onShrinkOdd,
  onShrinkEven,
}: ShrinkMarginsModalProps) {
  return (
    <ScopedPageActionModal
      title="Shrink Margins"
      help="Shrink MediaBox inward (clips page edges; does not scale content)."
      range={range}
      pageCount={pageCount}
      rangeFirst
      onClose={onClose}
      onApply={onShrink}
      onApplyOdd={onShrinkOdd}
      onApplyEven={onShrinkEven}
      applyLabel="Shrink"
      oddLabel="Shrink Odd"
      evenLabel="Shrink Even"
    >
      <MarginQuadInputs margins={margins} onChange={onMarginsChange} />
    </ScopedPageActionModal>
  );
}
