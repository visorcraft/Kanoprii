import type React from 'react';
import kanopriiWelcome from '../assets/kanoprii.png';
import type { MarkdownOcrNotice, PdfPageSize, ScrollViewMode, ViewMode } from '../app/types';
import { ContinuousViewer } from './ContinuousViewer';
import { PdfPageView } from './PdfPageView';
import { PageControls } from './PageControls';

type ViewerMainProps = {
  filePath: string;
  viewMode: ViewMode;
  scrollViewMode: ScrollViewMode;
  pageCount: number | null;
  currentPage: number;
  pageSizes: PdfPageSize[];
  scrollRef: React.RefObject<HTMLDivElement | null>;
  onWheel: (e: React.WheelEvent) => void;
  onOpenPdf: () => void;
  markdownOcrNotice: MarkdownOcrNotice | null;
  markdownPath: string;
  markdownText: string;
  onOpenMarkdownSaveAs: () => void;
  continuous: React.ComponentProps<typeof ContinuousViewer> | null;
  pdfPage: React.ComponentProps<typeof PdfPageView>;
  pageControls: React.ComponentProps<typeof PageControls> | null;
};

export function ViewerMain({
  filePath,
  viewMode,
  scrollViewMode,
  pageCount,
  currentPage,
  scrollRef,
  onWheel,
  onOpenPdf,
  markdownOcrNotice,
  markdownPath,
  markdownText,
  onOpenMarkdownSaveAs,
  continuous,
  pdfPage,
  pageControls,
}: ViewerMainProps) {
  return (
    <main className="main">
      <div
        className={`page-scroll${!filePath ? ' welcome-scroll' : ''}${viewMode === 'markdown' ? ' markdown-scroll' : ''}${scrollViewMode === 'continuous' ? ' continuous-scroll' : ''}`}
        ref={scrollRef}
        onWheel={onWheel}
        tabIndex={0}
        aria-label="Document pages"
      >
        {!filePath ? (
          <button
            type="button"
            className="welcome-splash"
            onClick={onOpenPdf}
            data-testid="welcome-open-pdf"
            aria-label="Click to open a PDF"
          >
            <img src={kanopriiWelcome} alt="" className="welcome-kanoprii" aria-hidden="true" />
            <span className="welcome-hint">Click to open a PDF</span>
          </button>
        ) : viewMode === 'markdown' ? (
          <div className="markdown-viewer">
            <div className="markdown-header">
              <span>Markdown</span>
              {markdownOcrNotice && (
                <span className={`markdown-ocr-badge ${markdownOcrNotice.tone === 'success' ? 'ready' : 'missing'}`}>
                  {markdownOcrNotice.message}
                </span>
              )}
              {markdownPath && <span className="markdown-path">{markdownPath}</span>}
              <button type="button" onClick={onOpenMarkdownSaveAs} className="btn btn-secondary">Save As…</button>
            </div>
            <pre className="markdown-preview">{markdownText}</pre>
          </div>
        ) : scrollViewMode === 'continuous' && continuous && pageCount !== null ? (
          <ContinuousViewer {...continuous} pageCount={pageCount} currentPage={currentPage} />
        ) : (
          <PdfPageView {...pdfPage} />
        )}
      </div>
      {pageControls && <PageControls {...pageControls} />}
    </main>
  );
}
