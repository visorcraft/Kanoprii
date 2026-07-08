import { Modal } from '../ui/Modal';

type MetadataModalProps = {
  title: string;
  author: string;
  subject: string;
  keywords: string;
  creator: string;
  producer: string;
  creationDate: string | null;
  modDate: string | null;
  onTitleChange: (value: string) => void;
  onAuthorChange: (value: string) => void;
  onSubjectChange: (value: string) => void;
  onKeywordsChange: (value: string) => void;
  onCreatorChange: (value: string) => void;
  onProducerChange: (value: string) => void;
  onClose: () => void;
  onClear: () => void;
  onApply: () => void;
};

export function MetadataModal({
  title,
  author,
  subject,
  keywords,
  creator,
  producer,
  creationDate,
  modDate,
  onTitleChange,
  onAuthorChange,
  onSubjectChange,
  onKeywordsChange,
  onCreatorChange,
  onProducerChange,
  onClose,
  onClear,
  onApply,
}: MetadataModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Document metadata</h3>
      <p className="modal-help">Edits the PDF Info dictionary in the working copy. Save the document to write changes to your file.</p>
      <label>Title:</label>
      <input type="text" value={title} onChange={(e) => onTitleChange(e.target.value)} className="modal-input" />
      <label>Author:</label>
      <input type="text" value={author} onChange={(e) => onAuthorChange(e.target.value)} className="modal-input" />
      <label>Subject:</label>
      <input type="text" value={subject} onChange={(e) => onSubjectChange(e.target.value)} className="modal-input" />
      <label>Keywords:</label>
      <input type="text" value={keywords} onChange={(e) => onKeywordsChange(e.target.value)} className="modal-input" />
      <label>Creator:</label>
      <input type="text" value={creator} onChange={(e) => onCreatorChange(e.target.value)} className="modal-input" />
      <label>Producer:</label>
      <input type="text" value={producer} onChange={(e) => onProducerChange(e.target.value)} className="modal-input" />
      {creationDate && (
        <p className="modal-help">Creation date: <code>{creationDate}</code></p>
      )}
      {modDate && (
        <p className="modal-help">Modified date: <code>{modDate}</code></p>
      )}
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button onClick={() => void onClear()} className="btn btn-secondary">Clear all</button>
        <button onClick={() => void onApply()} className="btn">Apply</button>
      </div>
    </Modal>
  );
}
