import { Modal } from '../ui/Modal';
import { PageRangePairInputs } from '../pageRange/PageRangeFields';
import { MarginQuadInputs, type MarginValues } from './MarginQuadInputs';

type CropRangeModalProps = {
  startPage: number;
  endPage: number;
  pageCount: number | null;
  margins: MarginValues;
  onStartChange: (page: number) => void;
  onEndChange: (page: number) => void;
  onMarginsChange: (margins: MarginValues) => void;
  onClose: () => void;
  onCropOdd: () => void;
  onCropEven: () => void;
  onCrop: () => void;
};

export function CropRangeModal({
  startPage,
  endPage,
  pageCount,
  margins,
  onStartChange,
  onEndChange,
  onMarginsChange,
  onClose,
  onCropOdd,
  onCropEven,
  onCrop,
}: CropRangeModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Crop Page Range</h3>
      <p className="modal-help">Apply the same margins to every page in the range.</p>
      <PageRangePairInputs
        startPage={startPage}
        endPage={endPage}
        onStartChange={onStartChange}
        onEndChange={onEndChange}
        maxPage={pageCount ?? undefined}
      />
      <MarginQuadInputs margins={margins} onChange={onMarginsChange} />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onCropOdd()} className="btn">Crop Odd</button>
        <button onClick={() => void onCropEven()} className="btn">Crop Even</button>
        <button onClick={() => void onCrop()} className="btn">Crop</button>
      </div>
    </Modal>
  );
}
