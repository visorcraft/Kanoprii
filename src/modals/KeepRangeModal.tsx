import { PageRangePairModal } from './PageRangePairModal';

type KeepRangeModalProps = {
  startPage: number;
  endPage: number;
  pageCount: number | null;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onClose: () => void;
  onKeep: () => void;
};

export function KeepRangeModal({
  startPage,
  endPage,
  pageCount,
  onStartChange,
  onEndChange,
  onClose,
  onKeep,
}: KeepRangeModalProps) {
  return (
    <PageRangePairModal
      title="Keep Page Range"
      help="Delete every page outside the selected range."
      startPage={startPage}
      endPage={endPage}
      pageCount={pageCount}
      onStartChange={onStartChange}
      onEndChange={onEndChange}
      onClose={onClose}
      actions={<button onClick={() => void onKeep()} className="btn btn-danger">Keep range</button>}
    />
  );
}
