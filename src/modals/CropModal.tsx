import { Modal } from '../ui/Modal';
import { MarginQuadInputs, type MarginValues } from './MarginQuadInputs';

type CropModalProps = {
  currentPage: number;
  applyAll: boolean;
  pageWidth?: number;
  pageHeight?: number;
  margins: MarginValues;
  onApplyAllChange: (applyAll: boolean) => void;
  onMarginsChange: (margins: MarginValues) => void;
  onClose: () => void;
  onClearPageCrop: () => void;
  onClearAllCrops: () => void;
  onClearOddCrops: () => void;
  onClearEvenCrops: () => void;
  onCrop: () => void;
};

export function CropModal({
  currentPage,
  applyAll,
  pageWidth,
  pageHeight,
  margins,
  onApplyAllChange,
  onMarginsChange,
  onClose,
  onClearPageCrop,
  onClearAllCrops,
  onClearOddCrops,
  onClearEvenCrops,
  onCrop,
}: CropModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Crop {applyAll ? 'All Pages' : `Page ${currentPage + 1}`}</h3>
      <p className="modal-help">Trim margins (viewer pixels, max ~800×1132).</p>
      {pageWidth !== undefined && pageHeight !== undefined && !applyAll && (
        <p className="muted">MediaBox: {Math.round(pageWidth)}×{Math.round(pageHeight)} pt</p>
      )}
      <label>
        <input type="checkbox" checked={applyAll} onChange={(e) => onApplyAllChange(e.target.checked)} />
        {' '}
        Apply to all pages
      </label>
      <MarginQuadInputs margins={margins} onChange={onMarginsChange} labelStyle="crop" />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        {!applyAll && (
          <button onClick={() => void onClearPageCrop()} className="btn btn-secondary">Clear crop</button>
        )}
        <button onClick={() => void onClearAllCrops()} className="btn btn-secondary">Clear all crops</button>
        <button onClick={() => void onClearOddCrops()} className="btn btn-secondary">Clear odd crops</button>
        <button onClick={() => void onClearEvenCrops()} className="btn btn-secondary">Clear even crops</button>
        <button onClick={() => void onCrop()} className="btn">Crop</button>
      </div>
    </Modal>
  );
}
