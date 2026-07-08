import { Modal } from '../ui/Modal';

export type PageTextEditItem = {
  index: number;
  x: number;
  y: number;
  font_size: number;
  text: string;
};

type PageVectorEdit = {
  index: number;
  kind: string;
  width: number;
  height: number;
};

type PageEditsModalProps = {
  currentPage: number;
  textEdits: PageTextEditItem[];
  vectorEdits: PageVectorEdit[];
  onClose: () => void;
  onEditText: (edit: PageTextEditItem) => void;
  onRemoveText: (index: number) => void;
  onRemoveVector: (index: number) => void;
};

export function PageEditsModal({
  currentPage,
  textEdits,
  vectorEdits,
  onClose,
  onEditText,
  onRemoveText,
  onRemoveVector,
}: PageEditsModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Page Edits - page {currentPage + 1}</h3>
      <p className="modal-help">Text and vector shapes embedded in the PDF content stream for this page.</p>
      <h4>Text blocks</h4>
      {textEdits.length === 0 ? (
        <p className="muted">No page text on this page.</p>
      ) : (
        <ul className="summary-list">
          {textEdits.map((edit) => (
            <li key={`manage-text-${edit.index}`} className="page-edit-row">
              <span>{edit.text}</span>
              <span className="page-edit-actions">
                <button type="button" className="btn btn-secondary" onClick={() => onEditText(edit)}>Edit</button>
                <button type="button" className="btn btn-secondary" onClick={() => void onRemoveText(edit.index)}>Delete</button>
              </span>
            </li>
          ))}
        </ul>
      )}
      <h4>Vector shapes</h4>
      {vectorEdits.length === 0 ? (
        <p className="muted">No vector shapes on this page.</p>
      ) : (
        <ul className="summary-list">
          {vectorEdits.map((edit) => (
            <li key={`manage-vector-${edit.index}`} className="page-edit-row">
              <span>{edit.kind} {Math.round(edit.width)}×{Math.round(edit.height)}</span>
              <button type="button" className="btn btn-secondary" onClick={() => void onRemoveVector(edit.index)}>Delete</button>
            </li>
          ))}
        </ul>
      )}
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Close</button>
      </div>
    </Modal>
  );
}
