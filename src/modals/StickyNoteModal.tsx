import { Modal } from '../ui/Modal';

type StickyNoteModalProps = {
  noteDraft: string;
  onNoteDraftChange: (value: string) => void;
  onClose: () => void;
  onSubmit: () => void;
};

export function StickyNoteModal({
  noteDraft,
  onNoteDraftChange,
  onClose,
  onSubmit,
}: StickyNoteModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Add Sticky Note</h3>
      <label>Note text:</label>
      <textarea
        value={noteDraft}
        onChange={(e) => onNoteDraftChange(e.target.value)}
        className="modal-input note-textarea"
        rows={4}
        autoFocus
      />
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={onSubmit} className="btn" disabled={!noteDraft.trim()}>Add note</button>
      </div>
    </Modal>
  );
}
