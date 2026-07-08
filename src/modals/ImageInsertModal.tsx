import { useId } from 'react';
import { Modal } from '../ui/Modal';

type ImageInsertModalProps = {
  imagePath: string;
  onImagePathChange: (path: string) => void;
  onClose: () => void;
  onConfirm: () => void;
};

export function ImageInsertModal({
  imagePath,
  onImagePathChange,
  onClose,
  onConfirm,
}: ImageInsertModalProps) {
  const imageId = useId();

  return (
    <Modal onClose={onClose}>
      <h3>Insert Image</h3>
      <p className="modal-help">Choose a PNG or JPEG file, then click twice on the page to size and place it.</p>
      <label htmlFor={imageId}>Image path:</label>
      <input
        id={imageId}
        type="text"
        value={imagePath}
        onChange={(e) => onImagePathChange(e.target.value)}
        className="modal-input"
        placeholder="/path/to/image.png"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onConfirm()} className="btn" disabled={!imagePath.trim()}>Place on page</button>
      </div>
    </Modal>
  );
}
