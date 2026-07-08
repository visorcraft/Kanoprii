import type React from 'react';
import type {
  FormFieldData,
  PdfBookmarkEntry,
  PdfSignatureInfo,
  PdfSignatureVerificationSummary,
} from '../app/types';
import { signatureStatusLabel } from '../app/utils';
import { AnnotationsPanel } from './AnnotationsPanel';
import { PdfUaPanel } from './PdfUaPanel';

type PdfSidebarProps = {
  filePath: string;
  thumbnails: string[];
  currentPage: number;
  draggedIndex: number | null;
  onDragStart: (index: number) => void;
  onDragOver: (e: React.DragEvent) => void;
  onDrop: (e: React.DragEvent, index: number) => void;
  onGoToPage: (index: number) => void;
  showAnnotationsPanel: boolean;
  pdfRevision: number;
  onRemoveHighlightOnPage: (page: number, index: number) => void;
  onRemoveTextNoteOnPage: (page: number, index: number) => void;
  onRemoveRedactionOnPage: (page: number, index: number) => void;
  showBookmarksPanel: boolean;
  pdfBookmarks: PdfBookmarkEntry[];
  onOpenAddBookmarkModal: () => void;
  onOpenBookmarkAllModal: () => void;
  onClearAllBookmarks: () => void | Promise<void>;
  onReloadBookmarks: (path: string) => void | Promise<void>;
  onOpenRenameBookmarkModal: (index: number, title: string) => void;
  onRemoveBookmark: (index: number) => void | Promise<void>;
  showSignaturesPanel: boolean;
  pdfSignatures: PdfSignatureInfo[];
  signatureVerification: PdfSignatureVerificationSummary | null;
  onReloadSignatures: (path: string) => void | Promise<void>;
  showFormsPanel: boolean;
  showPdfUaPanel: boolean;
  showHiddenLayers: boolean;
  formFields: FormFieldData[];
  formDrafts: Record<string, string>;
  onFormDraftsChange: React.Dispatch<
    React.SetStateAction<Record<string, string>>
  >;
  onOpenAddFormFieldModal: () => void;
  onApplyFormField: (name: string) => void;
};

export function PdfSidebar({
  filePath,
  thumbnails,
  currentPage,
  draggedIndex,
  onDragStart,
  onDragOver,
  onDrop,
  onGoToPage,
  showAnnotationsPanel,
  pdfRevision,
  onRemoveHighlightOnPage,
  onRemoveTextNoteOnPage,
  onRemoveRedactionOnPage,
  showBookmarksPanel,
  pdfBookmarks,
  onOpenAddBookmarkModal,
  onOpenBookmarkAllModal,
  onClearAllBookmarks,
  onReloadBookmarks,
  onOpenRenameBookmarkModal,
  onRemoveBookmark,
  showSignaturesPanel,
  pdfSignatures,
  signatureVerification,
  onReloadSignatures,
  showFormsPanel,
  showPdfUaPanel,
  formFields,
  formDrafts,
  onFormDraftsChange,
  onOpenAddFormFieldModal,
  onApplyFormField,
}: PdfSidebarProps) {
  const handleThumbnailKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (thumbnails.length === 0) return;
    let nextIndex = currentPage;
    if (e.key === 'ArrowDown' || e.key === 'ArrowRight') {
      e.preventDefault();
      nextIndex = Math.min(currentPage + 1, thumbnails.length - 1);
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') {
      e.preventDefault();
      nextIndex = Math.max(currentPage - 1, 0);
    } else if (e.key === 'Home') {
      e.preventDefault();
      nextIndex = 0;
    } else if (e.key === 'End') {
      e.preventDefault();
      nextIndex = thumbnails.length - 1;
    }
    if (nextIndex !== currentPage) {
      onGoToPage(nextIndex);
      const list = e.currentTarget;
      requestAnimationFrame(() => {
        const thumbs = list.querySelectorAll<HTMLElement>('.thumbnail');
        thumbs[nextIndex]?.focus();
      });
    }
  };

  return (
    <aside className="sidebar">
      <h3>Thumbnails</h3>
      {thumbnails.length > 0 ? (
        <div className="thumbnail-list" onKeyDown={handleThumbnailKeyDown}>
          {thumbnails.map((src, idx) => (
            <img
              key={idx}
              src={src}
              draggable
              onDragStart={() => onDragStart(idx)}
              onDragOver={onDragOver}
              onDrop={(e) => onDrop(e, idx)}
              onClick={() => onGoToPage(idx)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  onGoToPage(idx);
                }
              }}
              tabIndex={currentPage === idx ? 0 : -1}
              role="button"
              aria-label={`Page ${idx + 1}`}
              className={`thumbnail ${currentPage === idx ? 'active' : ''} ${draggedIndex === idx ? 'dragging' : ''}`}
            />
          ))}
        </div>
      ) : (
        <p className="muted">No thumbnails loaded</p>
      )}
      {filePath && showAnnotationsPanel && (
        <AnnotationsPanel
          filePath={filePath}
          pdfRevision={pdfRevision}
          onGoToPage={onGoToPage}
          onRemoveHighlight={onRemoveHighlightOnPage}
          onRemoveTextNote={onRemoveTextNoteOnPage}
          onRemoveRedaction={onRemoveRedactionOnPage}
        />
      )}
      {filePath && showBookmarksPanel && (
        <div className="bookmarks-panel">
          <div className="forms-panel-header">
            <h3>Bookmarks</h3>
            <button
              type="button"
              onClick={onOpenAddBookmarkModal}
              className="btn"
              title="Add bookmark at current page"
            >
              Add
            </button>
            <button
              type="button"
              onClick={onOpenBookmarkAllModal}
              className="btn"
              title="Bookmark every page"
            >
              All
            </button>
            <button
              type="button"
              onClick={() => void onClearAllBookmarks()}
              className="btn"
              title="Remove all bookmarks"
            >
              Clear
            </button>
            <button
              type="button"
              onClick={() => void onReloadBookmarks(filePath)}
              className="btn"
              title="Reload bookmarks"
            >
              Refresh
            </button>
          </div>
          {pdfBookmarks.length === 0 ? (
            <p className="muted">No bookmarks in this PDF.</p>
          ) : (
            <div className="bookmark-list">
              {pdfBookmarks.map((bookmark, index) => (
                <div
                  key={`${bookmark.title}-${index}`}
                  className={`bookmark-row-wrap ${bookmark.page_index === currentPage ? 'active' : ''}`}
                  style={{ paddingLeft: `${12 + bookmark.depth * 14}px` }}
                >
                  <button
                    type="button"
                    className="bookmark-row"
                    disabled={bookmark.page_index === null}
                    onClick={() => {
                      if (bookmark.page_index !== null)
                        onGoToPage(bookmark.page_index);
                    }}
                  >
                    <span className="bookmark-title">{bookmark.title}</span>
                    {bookmark.page_index !== null && (
                      <span className="muted bookmark-page">
                        p.{bookmark.page_index + 1}
                      </span>
                    )}
                  </button>
                  <button
                    type="button"
                    className="btn btn-secondary"
                    title="Rename bookmark"
                    onClick={() =>
                      onOpenRenameBookmarkModal(index, bookmark.title)
                    }
                  >
                    ✎
                  </button>
                  <button
                    type="button"
                    className="btn btn-secondary"
                    title="Remove bookmark"
                    onClick={() => void onRemoveBookmark(index)}
                  >
                    ×
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
      {filePath && showSignaturesPanel && (
        <div className="signatures-panel">
          <div className="forms-panel-header">
            <h3>Digital Signatures</h3>
            <button
              type="button"
              onClick={() => void onReloadSignatures(filePath)}
              className="btn"
              title="Re-verify signatures"
            >
              Refresh
            </button>
          </div>
          {pdfSignatures.length === 0 ? (
            <p className="muted">No digital signatures in this PDF.</p>
          ) : (
            <div className="signature-list">
              {pdfSignatures.map((sig) => {
                const verified = signatureVerification?.signatures.find(
                  (entry) => entry.field_name === sig.field_name
                );
                const status = verified?.status ?? 'indeterminate';
                return (
                  <div
                    key={sig.field_name}
                    className={`signature-row signature-row--${status}`}
                  >
                    <div className="signature-row-header">
                      <strong>{sig.field_name}</strong>
                      <span
                        className={`signature-status signature-status--${status}`}
                      >
                        {signatureStatusLabel(status)}
                      </span>
                    </div>
                    {sig.signer_name && (
                      <div className="muted">Signer: {sig.signer_name}</div>
                    )}
                    {sig.reason && (
                      <div className="muted">Reason: {sig.reason}</div>
                    )}
                    {sig.location && (
                      <div className="muted">Location: {sig.location}</div>
                    )}
                    {sig.signing_time && (
                      <div className="muted">Signed: {sig.signing_time}</div>
                    )}
                    {sig.signed_percent !== null && (
                      <div className="muted">
                        Coverage: {sig.signed_percent.toFixed(1)}%
                      </div>
                    )}
                    {verified && (
                      <div className="muted signature-summary">
                        {verified.summary}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}
          {signatureVerification &&
            signatureVerification.signature_count > 0 && (
              <p className="muted signature-doc-summary">
                {signatureVerification.summary}
              </p>
            )}
        </div>
      )}
      {filePath && showFormsPanel && (
        <div className="forms-panel">
          <div className="forms-panel-header">
            <h3>Form Fields</h3>
            <button
              type="button"
              onClick={onOpenAddFormFieldModal}
              className="btn"
              title="Add text field"
            >
              + Field
            </button>
          </div>
          {formFields.length === 0 ? (
            <p className="muted">No fillable fields in this PDF.</p>
          ) : (
            <div className="form-field-list">
              {formFields.map((field) => (
                <div key={field.name} className="form-field-row">
                  <div className="form-field-meta">
                    <strong>{field.name}</strong>
                    <span className="muted">{field.field_type}</span>
                  </div>
                  {field.field_type === 'checkbox' ||
                  field.field_type === 'radio' ? (
                    <label className="form-checkbox-row">
                      <input
                        type="checkbox"
                        checked={formDrafts[field.name] === 'true'}
                        onChange={(e) =>
                          onFormDraftsChange((prev) => ({
                            ...prev,
                            [field.name]: e.target.checked ? 'true' : 'false',
                          }))
                        }
                      />
                      <span>Checked</span>
                    </label>
                  ) : field.field_type === 'choice' &&
                    field.options.length > 0 ? (
                    <select
                      className="form-field-input"
                      value={formDrafts[field.name] ?? ''}
                      onChange={(e) =>
                        onFormDraftsChange((prev) => ({
                          ...prev,
                          [field.name]: e.target.value,
                        }))
                      }
                    >
                      {field.options.map((option) => (
                        <option key={option} value={option}>
                          {option}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <input
                      type="text"
                      className="form-field-input"
                      value={formDrafts[field.name] ?? ''}
                      disabled={
                        field.field_type === 'button' ||
                        field.field_type === 'signature'
                      }
                      onChange={(e) =>
                        onFormDraftsChange((prev) => ({
                          ...prev,
                          [field.name]: e.target.value,
                        }))
                      }
                    />
                  )}
                  <button
                    type="button"
                    className="btn"
                    disabled={
                      field.field_type === 'button' ||
                      field.field_type === 'signature'
                    }
                    onClick={() => onApplyFormField(field.name)}
                  >
                    Apply
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
      {filePath && showPdfUaPanel && (
        <PdfUaPanel filePath={filePath} pdfRevision={pdfRevision} />
      )}
    </aside>
  );
}
