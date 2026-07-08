import { PageRangePairModal } from './PageRangePairModal';

type DeleteRangeModalProps = {
  startPage: number;
  endPage: number;
  pageCount: number | null;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onClose: () => void;
  onDelete: () => void;
};

export function DeleteRangeModal({
  startPage,
  endPage,
  pageCount,
  onStartChange,
  onEndChange,
  onClose,
  onDelete,
}: DeleteRangeModalProps) {
  return (
    <PageRangePairModal
      title="Delete Page Range"
      help="Remove multiple pages from the working copy. At least one page must remain."
      startPage={startPage}
      endPage={endPage}
      pageCount={pageCount}
      onStartChange={onStartChange}
      onEndChange={onEndChange}
      onClose={onClose}
      actions={<button onClick={() => void onDelete()} className="btn btn-danger">Delete range</button>}
    />
  );
}
