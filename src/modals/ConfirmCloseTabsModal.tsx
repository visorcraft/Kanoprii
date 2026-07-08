import { Modal } from '../ui/Modal';

type ConfirmCloseTabsModalProps = {
  count: number;
  onConfirm: () => void;
  onCancel: () => void;
};

/** Single confirmation for closing several tabs when some have unsaved edits. */
export function ConfirmCloseTabsModal({ count, onConfirm, onCancel }: ConfirmCloseTabsModalProps) {
  const noun = count === 1 ? 'tab has' : 'tabs have';
  const them = count === 1 ? 'it' : 'them';
  return (
    <Modal onClose={onCancel} aria-label="Close tabs with unsaved changes">
      <h3>Unsaved changes</h3>
      <p className="modal-help">
        {count} {noun} unsaved changes. Close and discard {them}?
      </p>
      <div className="modal-actions">
        <button type="button" className="btn btn-secondary" onClick={onCancel}>
          Cancel
        </button>
        <button type="button" className="btn btn-active" onClick={onConfirm}>
          Discard &amp; Close
        </button>
      </div>
    </Modal>
  );
}
