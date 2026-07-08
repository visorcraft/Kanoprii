import { Modal } from '../ui/Modal';

type MarkdownSaveAsModalProps = {
  outputPath: string;
  nativeDialogs: boolean;
  onOutputPathChange: (path: string) => void;
  onClose: () => void;
  onChooseNative: () => void;
  onSave: () => void;
};

export function MarkdownSaveAsModal({
  outputPath,
  nativeDialogs,
  onOutputPathChange,
  onClose,
  onChooseNative,
  onSave,
}: MarkdownSaveAsModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Save Markdown As</h3>
      <label>Save to path:</label>
      <div className="modal-path-row">
        <input
          type="text"
          value={outputPath}
          onChange={(e) => onOutputPathChange(e.target.value)}
          className="modal-input"
          placeholder="/path/to/output.md"
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
