import { PageRangePairModal } from './PageRangePairModal';

type ReverseRangeModalProps = {
  startPage: number;
  endPage: number;
  pageCount: number | null;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onClose: () => void;
  onReverse: () => void;
};

export function ReverseRangeModal({
  startPage,
  endPage,
  pageCount,
  onStartChange,
  onEndChange,
  onClose,
  onReverse,
}: ReverseRangeModalProps) {
  return (
    <PageRangePairModal
      title="Reverse Page Range"
      help="Reverse order within the selected page range only."
      startPage={startPage}
      endPage={endPage}
      pageCount={pageCount}
      onStartChange={onStartChange}
      onEndChange={onEndChange}
      onClose={onClose}
      actions={<button onClick={() => void onReverse()} className="btn">Reverse</button>}
    />
  );
}
