import type { ReactNode } from 'react';
import { Modal } from '../ui/Modal';
import { PageRangeFields } from '../pageRange/PageRangeFields';
import type { PageRangeController } from '../pageRange/usePageRange';

type ScopedPageActionModalProps = {
  title: string;
  help: string;
  range: PageRangeController;
  pageCount: number | null;
  onClose: () => void;
  onApply: () => void;
  onApplyOdd?: () => void;
  onApplyEven?: () => void;
  applyLabel?: string;
  oddLabel?: string;
  evenLabel?: string;
  rangeApplyLabel?: string;
  applyDisabled?: boolean;
  rangeFirst?: boolean;
  children?: ReactNode;
};

export function ScopedPageActionModal({
  title,
  help,
  range,
  pageCount,
  onClose,
  onApply,
  onApplyOdd,
  onApplyEven,
  applyLabel = 'Apply',
  oddLabel = 'Apply Odd',
  evenLabel = 'Apply Even',
  rangeApplyLabel,
  applyDisabled = false,
  rangeFirst = false,
  children,
}: ScopedPageActionModalProps) {
  const hasParity = onApplyOdd !== undefined && onApplyEven !== undefined;
  const rangeFields = (
    <PageRangeFields range={range} pageCount={pageCount} applyLabel={rangeApplyLabel} />
  );

  return (
    <Modal onClose={onClose}>
      <h3>{title}</h3>
      <p className="modal-help">{help}</p>
      {rangeFirst ? rangeFields : children}
      {!rangeFirst && rangeFields}
      {rangeFirst && children}
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        {hasParity && (
          <>
            <button onClick={() => void onApplyOdd()} className="btn" disabled={applyDisabled}>{oddLabel}</button>
            <button onClick={() => void onApplyEven()} className="btn" disabled={applyDisabled}>{evenLabel}</button>
          </>
        )}
        <button onClick={() => void onApply()} className="btn" disabled={applyDisabled}>{applyLabel}</button>
      </div>
    </Modal>
  );
}
