import { useId, type KeyboardEvent } from 'react';
import { Modal } from '../ui/Modal';

export type PdfBrowserEntry = {
  name: string;
  path: string;
  isDir: boolean;
};

export type PdfBrowserListing = {
  currentDir: string;
  parentDir: string | null;
  entries: PdfBrowserEntry[];
};

type PdfBrowserModalProps = {
  pathInput: string;
  listing: PdfBrowserListing | null;
  onPathInputChange: (path: string) => void;
  onClose: () => void;
  onCommitPath: () => void;
  onNavigateParent: (parentDir: string | undefined) => void;
  onEntryClick: (entry: PdfBrowserEntry) => void;
};

export function PdfBrowserModal({
  pathInput,
  listing,
  onPathInputChange,
  onClose,
  onCommitPath,
  onNavigateParent,
  onEntryClick,
}: PdfBrowserModalProps) {
  const folderId = useId();

  const onFieldKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      onCommitPath();
      e.currentTarget.blur();
    }
  };

  return (
    <Modal onClose={onClose}>
      <h3>Browse PDF</h3>
      <label htmlFor={folderId}>Folder:</label>
      <div className="modal-path-row">
        <input
          id={folderId}
          type="text"
          value={pathInput}
          onChange={(e) => onPathInputChange(e.target.value)}
          onKeyDown={onFieldKeyDown}
          className="modal-input"
        />
        <button onClick={onCommitPath} className="btn">Go</button>
      </div>
      <div className="file-browser-list">
        {listing?.parentDir && (
          <button className="file-browser-row" onClick={() => onNavigateParent(listing.parentDir ?? undefined)}>
            <span className="file-browser-kind">Folder</span>
            <span className="file-browser-name">..</span>
          </button>
        )}
        {listing?.entries.map((entry) => (
          <button key={entry.path} className="file-browser-row" onClick={() => onEntryClick(entry)}>
            <span className="file-browser-kind">{entry.isDir ? 'Folder' : 'PDF'}</span>
            <span className="file-browser-name">{entry.name}</span>
          </button>
        ))}
        {listing && listing.entries.length === 0 && (
          <p className="muted browser-empty">No folders or PDF files here</p>
        )}
      </div>
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
      </div>
    </Modal>
  );
}
