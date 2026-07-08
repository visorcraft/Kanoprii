import { useId } from 'react';
import { ScopedPageActionModal } from './ScopedPageActionModal';
import type { PageRangeController } from '../pageRange/usePageRange';

type PageNumbersModalProps = {
  range: PageRangeController;
  pageCount: number | null;
  prefix: string;
  onPrefixChange: (value: string) => void;
  onClose: () => void;
  onApply: () => void;
  onApplyOdd: () => void;
  onApplyEven: () => void;
};

export function PageNumbersModal({
  range,
  pageCount,
  prefix,
  onPrefixChange,
  onClose,
  onApply,
  onApplyOdd,
  onApplyEven,
}: PageNumbersModalProps) {
  const prefixId = useId();

  return (
    <ScopedPageActionModal
      title="Page Numbers"
      help="Stamp footer page numbers onto the working copy."
      range={range}
      pageCount={pageCount}
      rangeFirst
      onClose={onClose}
      onApply={onApply}
      onApplyOdd={onApplyOdd}
      onApplyEven={onApplyEven}
    >
      <label htmlFor={prefixId}>Prefix (e.g. &quot;Page &quot;):</label>
      <input
        id={prefixId}
        type="text"
        value={prefix}
        onChange={(e) => onPrefixChange(e.target.value)}
        className="modal-input"
      />
    </ScopedPageActionModal>
  );
}
