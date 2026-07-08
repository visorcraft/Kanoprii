import { Modal } from '../ui/Modal';

export type DocumentSummaryData = {
  pageCount: number;
  wordCount: number;
  titleGuess: string | null;
  overview: string;
  keyPoints: string[];
  scannedPages: number;
  extraction: {
    headings: string[];
    emails: string[];
    urls: string[];
    dates: string[];
  };
};

type DocumentSummaryModalProps = {
  summary: DocumentSummaryData;
  onClose: () => void;
  onCopy: () => void;
  onSave: () => void;
};

export function DocumentSummaryModal({
  summary,
  onClose,
  onCopy,
  onSave,
}: DocumentSummaryModalProps) {
  return (
    <Modal onClose={onClose}>
      <h3>Document Summary</h3>
      <p className="modal-help">
        {summary.titleGuess ? (
          <>
            <strong>{summary.titleGuess}</strong>
            {' · '}
          </>
        ) : null}
        {summary.pageCount} pages · {summary.wordCount} words
        {summary.scannedPages > 0 ? ` · ${summary.scannedPages} scanned/image-only` : ''}
      </p>
      <div className="summary-panel">
        <h4>Overview</h4>
        <p>{summary.overview}</p>
        {summary.keyPoints.length > 0 && (
          <>
            <h4>Key points</h4>
            <ul className="summary-list">
              {summary.keyPoints.map((point) => <li key={point}>{point}</li>)}
            </ul>
          </>
        )}
        {summary.extraction.headings.length > 0 && (
          <>
            <h4>Headings</h4>
            <ul className="summary-list">
              {summary.extraction.headings.map((heading) => <li key={heading}>{heading}</li>)}
            </ul>
          </>
        )}
        {(summary.extraction.emails.length > 0
          || summary.extraction.urls.length > 0
          || summary.extraction.dates.length > 0) && (
          <>
            <h4>Extracted contacts &amp; dates</h4>
            <ul className="summary-list">
              {summary.extraction.emails.map((email) => <li key={`email-${email}`}>{email}</li>)}
              {summary.extraction.urls.map((url) => <li key={`url-${url}`}>{url}</li>)}
              {summary.extraction.dates.map((date) => <li key={`date-${date}`}>{date}</li>)}
            </ul>
          </>
        )}
      </div>
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Close</button>
        <button onClick={() => void onCopy()} className="btn">Copy</button>
        <button onClick={() => void onSave()} className="btn btn-active">Save summary</button>
      </div>
    </Modal>
  );
}
