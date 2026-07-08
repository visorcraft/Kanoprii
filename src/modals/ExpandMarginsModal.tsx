import { ScopedPageActionModal } from './ScopedPageActionModal';
import { MarginQuadInputs, type MarginValues } from './MarginQuadInputs';
import type { PageRangeController } from '../pageRange/usePageRange';

type ExpandMarginsModalProps = {
  range: PageRangeController;
  pageCount: number | null;
  margins: MarginValues;
  onMarginsChange: (margins: MarginValues) => void;
  onClose: () => void;
  onExpand: () => void;
  onExpandOdd: () => void;
  onExpandEven: () => void;
};

export function ExpandMarginsModal({
  range,
  pageCount,
  margins,
  onMarginsChange,
  onClose,
  onExpand,
  onExpandOdd,
  onExpandEven,
}: ExpandMarginsModalProps) {
  return (
    <ScopedPageActionModal
      title="Expand Margins"
      help="Grow MediaBox outward (adds white space; does not scale content)."
      range={range}
      pageCount={pageCount}
      rangeFirst
      onClose={onClose}
      onApply={onExpand}
      onApplyOdd={onExpandOdd}
      onApplyEven={onExpandEven}
      applyLabel="Expand"
      oddLabel="Expand Odd"
      evenLabel="Expand Even"
    >
      <MarginQuadInputs margins={margins} onChange={onMarginsChange} />
    </ScopedPageActionModal>
  );
}
