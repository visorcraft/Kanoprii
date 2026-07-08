import { useId } from 'react';
import { Modal } from '../ui/Modal';

type SwapPagesModalProps = {
  pageA: number;
  pageB: number;
  pageCount: number | null;
  onPageAChange: (page: number) => void;
  onPageBChange: (page: number) => void;
  onClose: () => void;
  onSwap: () => void;
};

export function SwapPagesModal({
  pageA,
  pageB,
  pageCount,
  onPageAChange,
  onPageBChange,
  onClose,
  onSwap,
}: SwapPagesModalProps) {
  const baseId = useId();
  const pageAId = `${baseId}-a`;
  const pageBId = `${baseId}-b`;
  const parsePage = (value: string) => Math.max(0, parseInt(value, 10) - 1);

  return (
    <Modal onClose={onClose}>
      <h3>Swap Pages</h3>
      <p className="modal-help">Exchange the positions of two pages in the working copy.</p>
      <label htmlFor={pageAId}>
        Page A (1-{pageCount ?? 0}):
        {' '}
        <input
          id={pageAId}
          type="number"
          value={pageA + 1}
          onChange={(e) => onPageAChange(parsePage(e.target.value))}
          min="1"
          max={pageCount ?? undefined}
          className="modal-input"
        />
      </label>
      <label htmlFor={pageBId}>
        Page B (1-{pageCount ?? 0}):
        {' '}
        <input
          id={pageBId}
          type="number"
          value={pageB + 1}
          onChange={(e) => onPageBChange(parsePage(e.target.value))}
          min="1"
          max={pageCount ?? undefined}
          className="modal-input"
        />
      </label>
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onSwap()} className="btn" disabled={pageA === pageB}>Swap</button>
      </div>
    </Modal>
  );
}
