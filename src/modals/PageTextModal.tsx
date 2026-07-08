import { Modal } from '../ui/Modal';

type PageTextModalProps = {
  editing: boolean;
  text: string;
  fontSize: string;
  onTextChange: (value: string) => void;
  onFontSizeChange: (value: string) => void;
  onClose: () => void;
  onSave: () => void;
};

export function PageTextModal({
  editing,
  text,
  fontSize,
  onTextChange,
  onFontSizeChange,
  onClose,
  onSave,
}: PageTextModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>{editing ? 'Edit Page Text' : 'Add Page Text'}</h3>
      <label>Text:</label>
      <input
        type="text"
        value={text}
        onChange={(e) => onTextChange(e.target.value)}
        className="modal-input"
        autoFocus
      />
      <label>Font size (8–72):</label>
      <input
        type="number"
        min="8"
        max="72"
        value={fontSize}
        onChange={(e) => onFontSizeChange(e.target.value)}
        className="modal-input"
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onSave()} className="btn" disabled={!text.trim()}>Save</button>
      </div>
    </Modal>
  );
}
