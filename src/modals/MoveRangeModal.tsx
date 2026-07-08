import { PageRangePairModal } from './PageRangePairModal';

type MoveRangeModalProps = {
  startPage: number;
  endPage: number;
  targetIndex: number;
  pageCount: number | null;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onTargetChange: (index: number) => void;
  onClose: () => void;
  onMoveToStart: () => void;
  onMoveToEnd: () => void;
  onMove: () => void;
};

export function MoveRangeModal({
  startPage,
  endPage,
  targetIndex,
  pageCount,
  onStartChange,
  onEndChange,
  onTargetChange,
  onClose,
  onMoveToStart,
  onMoveToEnd,
  onMove,
}: MoveRangeModalProps) {
  return (
    <PageRangePairModal
      title="Move Page Range"
      help="Move a contiguous block so its first page lands at the target index (0 = first)."
      startPage={startPage}
      endPage={endPage}
      pageCount={pageCount}
      onStartChange={onStartChange}
      onEndChange={onEndChange}
      onClose={onClose}
      actions={(
        <>
          <button onClick={() => void onMoveToStart()} className="btn">To Start</button>
          <button onClick={() => void onMoveToEnd()} className="btn">To End</button>
          <button onClick={() => void onMove()} className="btn">Move</button>
        </>
      )}
    >
      <label>
        Target index (1-{((pageCount ?? 0) + 1)}):
        {' '}
        <input
          type="number"
          value={targetIndex + 1}
          onChange={(e) => onTargetChange(Math.max(0, (parseInt(e.target.value, 10) || 1) - 1))}
          min="1"
          max={(pageCount ?? 0) + 1}
          className="modal-input"
        />
      </label>
    </PageRangePairModal>
  );
}
