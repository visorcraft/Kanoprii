import { MAX_ZOOM, MIN_ZOOM } from '../app/constants';
import type { PdfPageSize } from '../app/types';
import { onFieldKeyDown } from './fieldInput';

type PageControlsProps = {
  pageCount: number;
  currentPage: number;
  pageInput: string;
  pageSizes: PdfPageSize[];
  onPageInputChange: (value: string) => void;
  onCommitPage: () => void;
  onGoToPage: (index: number) => void;
  zoom: number;
  zoomInput: string;
  onZoomInputChange: (value: string) => void;
  onCommitZoom: () => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onResetZoom: () => void;
};

export function PageControls({
  pageCount,
  currentPage,
  pageInput,
  pageSizes,
  onPageInputChange,
  onCommitPage,
  onGoToPage,
  zoom,
  zoomInput,
  onZoomInputChange,
  onCommitZoom,
  onZoomIn,
  onZoomOut,
  onResetZoom,
}: PageControlsProps) {
  return (
    <div className="page-controls">
      <button onClick={() => onGoToPage(currentPage - 1)} disabled={currentPage === 0} className="btn">Prev</button>
      <span className="field-group">
        <input
          className="num-input"
          type="text"
          inputMode="numeric"
          value={pageInput}
          onChange={(e) => onPageInputChange(e.target.value)}
          onKeyDown={(e) => onFieldKeyDown(e, onCommitPage)}
          onBlur={onCommitPage}
          aria-label="Current page"
        />
        <span className="muted" data-testid="page-count">/ {pageCount}</span>
        {pageSizes[currentPage] && (
          <span className="muted" title="Page size in PDF points">
            {' '}· {Math.round(pageSizes[currentPage].width)}×{Math.round(pageSizes[currentPage].height)}pt
            {pageSizes[currentPage].rotation !== 0 ? ` · ${pageSizes[currentPage].rotation}°` : ''}
          </span>
        )}
      </span>
      <button onClick={() => onGoToPage(currentPage + 1)} disabled={currentPage === pageCount - 1} className="btn">Next</button>

      <span className="zoom-divider" />

      <button onClick={onZoomOut} disabled={zoom <= MIN_ZOOM} className="btn">−</button>
      <span className="field-group">
        <input
          className="num-input"
          type="text"
          inputMode="numeric"
          value={zoomInput}
          onChange={(e) => onZoomInputChange(e.target.value)}
          onKeyDown={(e) => onFieldKeyDown(e, onCommitZoom)}
          onBlur={onCommitZoom}
          aria-label="Zoom percent"
          data-testid="zoom-input"
        />
        <span className="muted">%</span>
      </span>
      <button onClick={onZoomIn} disabled={zoom >= MAX_ZOOM} className="btn">+</button>
      <button onClick={onResetZoom} className="btn btn-secondary">Reset</button>
    </div>
  );
}
