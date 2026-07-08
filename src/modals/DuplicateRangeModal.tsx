import { PageRangePairModal } from './PageRangePairModal';

type DuplicateRangeModalProps = {
  startPage: number;
  endPage: number;
  pageCount: number | null;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onClose: () => void;
  onDuplicate: () => void;
  onDuplicateBefore: () => void;
  onDuplicateToStart: () => void;
  onDuplicateToEnd: () => void;
};

export function DuplicateRangeModal({
  startPage,
  endPage,
  pageCount,
  onStartChange,
  onEndChange,
  onClose,
  onDuplicate,
  onDuplicateBefore,
  onDuplicateToStart,
  onDuplicateToEnd,
}: DuplicateRangeModalProps) {
  return (
    <PageRangePairModal
      title="Duplicate Page Range"
      help="Deep-copy a page range and insert the copies immediately after the range."
      startPage={startPage}
      endPage={endPage}
      pageCount={pageCount}
      onStartChange={onStartChange}
      onEndChange={onEndChange}
      onClose={onClose}
      actions={(
        <>
          <button onClick={() => void onDuplicate()} className="btn">Duplicate</button>
          <button onClick={() => void onDuplicateBefore()} className="btn">Before</button>
          <button onClick={() => void onDuplicateToStart()} className="btn">To Start</button>
          <button onClick={() => void onDuplicateToEnd()} className="btn">To End</button>
        </>
      )}
    />
  );
}
