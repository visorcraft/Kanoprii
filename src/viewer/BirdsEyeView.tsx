import type { CSSProperties, DragEvent } from 'react';
import type { BirdsEyeWorkspace } from '../app/useBirdsEyeWorkspace';

type BirdsEyeViewProps = BirdsEyeWorkspace;

function pageLabel(pageCount: number) {
  return `${pageCount} page${pageCount === 1 ? '' : 's'}`;
}

export function BirdsEyeView({
  documents,
  totalPages,
  zoom,
  onZoomIn,
  onZoomOut,
  onOpenDocument,
  onSelectDocument,
  onSelectPage,
  onOpenPage,
  onAddPages,
  onPageDragStart,
  onPageDragEnd,
  onPageDragOver,
  onPageDrop,
}: BirdsEyeViewProps) {
  const handleDragStart = (
    event: DragEvent<HTMLButtonElement>,
    sessionId: string,
    pageIndex: number,
  ) => {
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.setData('text/plain', `${sessionId}:${pageIndex}`);
    onPageDragStart(sessionId, pageIndex);
  };

  return (
    <div className="birdseye-workspace">
      <div className="birdseye-toolbar">
        <div className="birdseye-counts">
          <strong>{documents.length}</strong> document{documents.length === 1 ? '' : 's'}
          <span> - </span>
          <strong>{totalPages}</strong> page{totalPages === 1 ? '' : 's'}
        </div>
        <div className="birdseye-toolbar-spacer" />
        <div className="birdseye-zoom" role="group" aria-label="Bird's Eye zoom">
          <button type="button" className="birdseye-icon-button" onClick={onZoomOut} aria-label="Zoom out">
            -
          </button>
          <span>{Math.round(zoom * 100)}%</span>
          <button type="button" className="birdseye-icon-button" onClick={onZoomIn} aria-label="Zoom in">
            +
          </button>
        </div>
        <button type="button" className="btn birdseye-open" onClick={onOpenDocument}>
          Open
        </button>
        <span className="birdseye-autosave">Auto-save on</span>
      </div>

      <div className="birdseye-scroll" aria-label="Bird's Eye document arrangement">
        {documents.length === 0 ? (
          <button type="button" className="birdseye-empty" onClick={onOpenDocument}>
            <span>+</span>
            Add document
          </button>
        ) : (
          <>
            {documents.map((document, docIndex) => (
              <section
                key={document.id}
                className={`birdseye-document${document.active ? ' active' : ''}`}
                aria-label={`${document.label}, ${pageLabel(document.pageCount)}`}
                onClick={() => onSelectDocument(document.id)}
              >
                <div className="birdseye-document-title">
                  <span>{String(docIndex + 1).padStart(2, '0')}</span>
                  <strong>{document.label}</strong>
                  <em>{pageLabel(document.pageCount)}</em>
                </div>
                <div
                  className="birdseye-page-row"
                  style={{ '--bird-page-width': `${Math.round(160 * zoom)}px` } as CSSProperties}
                  onDragOver={onPageDragOver}
                  onDrop={() => onPageDrop(document.id, document.pageCount)}
                >
                  {document.thumbnails.map((src, pageIndex) => (
                    <button
                      type="button"
                      key={`${document.id}-${pageIndex}-${src}`}
                      className={`birdseye-page${document.currentPage === pageIndex ? ' selected' : ''}`}
                      draggable
                      onClick={() => onSelectPage(document.id, pageIndex)}
                      onDoubleClick={() => onOpenPage(document.id, pageIndex)}
                      onDragStart={(event) => handleDragStart(event, document.id, pageIndex)}
                      onDragEnd={onPageDragEnd}
                      onDragOver={onPageDragOver}
                      onDrop={(event) => {
                        event.stopPropagation();
                        onPageDrop(document.id, pageIndex);
                      }}
                      aria-label={`${document.label}, page ${pageIndex + 1}`}
                    >
                      <img src={src} alt="" draggable={false} />
                      <span>{pageIndex + 1}</span>
                    </button>
                  ))}
                  <button
                    type="button"
                    className="birdseye-add-page"
                    onClick={() => onAddPages(document.id)}
                    onDragOver={onPageDragOver}
                    onDrop={(event) => {
                      event.stopPropagation();
                      onPageDrop(document.id, document.pageCount);
                    }}
                  >
                    <span>+</span>
                    Add page
                  </button>
                </div>
              </section>
            ))}
            <button type="button" className="birdseye-add-document" onClick={onOpenDocument}>
              <span>+</span>
              Add document
            </button>
          </>
        )}
      </div>
    </div>
  );
}
