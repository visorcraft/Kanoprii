import type { ReactNode } from 'react';
import { Modal } from '../ui/Modal';
import { PageRangePairInputs } from '../pageRange/PageRangeFields';

type PageRangePairModalProps = {
  title: string;
  help: string;
  startPage: number;
  endPage: number;
  pageCount: number | null;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onClose: () => void;
  children?: ReactNode;
  actions: ReactNode;
};

export function PageRangePairModal({
  title,
  help,
  startPage,
  endPage,
  pageCount,
  onStartChange,
  onEndChange,
  onClose,
  children,
  actions,
}: PageRangePairModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>{title}</h3>
      <p className="modal-help">{help}</p>
      <PageRangePairInputs
        startPage={startPage}
        endPage={endPage}
        onStartChange={onStartChange}
        onEndChange={onEndChange}
        maxPage={pageCount ?? undefined}
      />
      {children}
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        {actions}
      </div>
    </Modal>
  );
}
