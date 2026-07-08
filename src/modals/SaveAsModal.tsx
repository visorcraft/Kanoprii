import { useId } from 'react';
import { Modal } from '../ui/Modal';

type SaveAsModalProps = {
  outputPath: string;
  nativeDialogs: boolean;
  onOutputPathChange: (path: string) => void;
  onClose: () => void;
  onChooseNative: () => void;
  onSave: () => void;
};

export function SaveAsModal({
  outputPath,
  nativeDialogs,
  onOutputPathChange,
  onClose,
  onChooseNative,
  onSave,
}: SaveAsModalProps) {
  const outputId = useId();

  return (
    <Modal onClose={onClose}>
      <h3>Save As</h3>
      <label htmlFor={outputId}>Save to path:</label>
      <div className="modal-path-row">
        <input
          id={outputId}
          type="text"
          value={outputPath}
          onChange={(e) => onOutputPathChange(e.target.value)}
          className="modal-input"
          placeholder="/path/to/output.pdf"
        />
        {nativeDialogs && (
          <button onClick={() => void onChooseNative()} className="btn">Choose location…</button>
        )}
      </div>
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={onSave} className="btn" disabled={!outputPath.trim()}>Save</button>
      </div>
    </Modal>
  );
}
