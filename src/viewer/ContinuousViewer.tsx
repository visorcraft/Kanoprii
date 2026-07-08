import { useEffect, type ComponentProps } from 'react';
import type { PdfPageSize } from '../app/types';
import { PdfPageView } from './PdfPageView';

type PdfPageProps = ComponentProps<typeof PdfPageView>;

const EMPTY_ANNOTATIONS: PdfPageProps['annotations'] = [];
const EMPTY_TEXT_RUNS: PdfPageProps['textRuns'] = [];
const EMPTY_TEXT_EDITS: PdfPageProps['pageTextEdits'] = [];
const EMPTY_VECTOR_EDITS: PdfPageProps['pageVectorEdits'] = [];
const EMPTY_INK: PdfPageProps['inkDraft'] = [];
const EMPTY_FORMS: PdfPageProps['formFields'] = [];
const NOOP_REMOVE_HIGHLIGHT: PdfPageProps['onRemoveHighlight'] = () => {};
const NOOP_REMOVE_REDACTION: PdfPageProps['onRemoveRedaction'] = () => {};
const NOOP_REMOVE_STAMP: PdfPageProps['onRemoveStamp'] = () => {};
const NOOP_REMOVE_SHAPE: PdfPageProps['onRemoveShape'] = () => {};
const NOOP_REMOVE_INK: PdfPageProps['onRemoveInkStroke'] = () => {};
const NOOP_REMOVE_NOTE: PdfPageProps['onRemoveTextNote'] = () => {};

type ContinuousViewerProps = {
  pageCount: number;
  currentPage: number;
  placeholderHeight: (page: number) => number;
  registerPageRef: (el: HTMLDivElement | null) => void;
  getPageUrl: (page: number) => string | null;
  requestPage: (page: number) => void;
  renderPages: Set<number>;
  pdfPage: Omit<PdfPageProps, 'imageSrc' | 'currentPage'>;
  pageImageSrc: string | null;
  pageSizes: PdfPageSize[];
};

export function ContinuousViewer({
  pageCount,
  currentPage,
  placeholderHeight,
  registerPageRef,
  getPageUrl,
  requestPage,
  renderPages,
  pdfPage,
}: ContinuousViewerProps) {
  useEffect(() => {
    for (const page of renderPages) {
      requestPage(page);
    }
  }, [renderPages, requestPage]);

  return (
    <div className="continuous-viewer">
      {Array.from({ length: pageCount }, (_, page) => {
        const height = placeholderHeight(page);
        const showPage = renderPages.has(page);
        const imageSrc = getPageUrl(page);
        const isActive = page === currentPage;

        return (
          <div
            key={page}
            className="continuous-page-slot"
            data-page-index={page}
            data-testid={showPage ? `continuous-page-${page + 1}` : undefined}
            ref={registerPageRef}
            style={{ minHeight: height }}
          >
            {showPage && imageSrc ? (
              <PdfPageView
                {...pdfPage}
                currentPage={page}
                imageSrc={imageSrc}
                pageContainerRef={isActive ? pdfPage.pageContainerRef : undefined}
                imgRef={isActive ? pdfPage.imgRef : undefined}
                onImageLoad={isActive ? pdfPage.onImageLoad : undefined}
                onPageClick={isActive ? pdfPage.onPageClick : undefined}
                onMouseDown={isActive ? pdfPage.onMouseDown : undefined}
                onMouseMove={isActive ? pdfPage.onMouseMove : undefined}
                onMouseUp={isActive ? pdfPage.onMouseUp : undefined}
                highlightMode={isActive ? pdfPage.highlightMode : false}
                noteMode={isActive ? pdfPage.noteMode : false}
                drawMode={isActive ? pdfPage.drawMode : false}
                shapeMode={isActive ? pdfPage.shapeMode : false}
                stampMode={isActive ? pdfPage.stampMode : false}
                redactMode={isActive ? pdfPage.redactMode : false}
                imageInsertMode={isActive ? pdfPage.imageInsertMode : false}
                textEditMode={isActive ? pdfPage.textEditMode : false}
                vectorEditMode={isActive ? pdfPage.vectorEditMode : false}
                formAddMode={isActive ? pdfPage.formAddMode : false}
                annotations={isActive ? pdfPage.annotations : EMPTY_ANNOTATIONS}
                activeSearchRect={isActive ? pdfPage.activeSearchRect : null}
                textRuns={isActive ? pdfPage.textRuns : EMPTY_TEXT_RUNS}
                textLayerInteractive={isActive ? pdfPage.textLayerInteractive : false}
                textEditActiveRun={isActive ? pdfPage.textEditActiveRun : null}
                textEditActiveLine={isActive ? pdfPage.textEditActiveLine : null}
                pageTextEdits={isActive ? pdfPage.pageTextEdits : EMPTY_TEXT_EDITS}
                pageVectorEdits={isActive ? pdfPage.pageVectorEdits : EMPTY_VECTOR_EDITS}
                drawing={isActive ? pdfPage.drawing : false}
                highlightStart={isActive ? pdfPage.highlightStart : null}
                highlightRect={isActive ? pdfPage.highlightRect : null}
                shapeLineEnd={isActive ? pdfPage.shapeLineEnd : null}
                inkDraft={isActive ? pdfPage.inkDraft : EMPTY_INK}
                showFormsPanel={isActive ? pdfPage.showFormsPanel : false}
                formFields={isActive ? pdfPage.formFields : EMPTY_FORMS}
                onRemoveHighlight={isActive ? pdfPage.onRemoveHighlight : NOOP_REMOVE_HIGHLIGHT}
                onRemoveRedaction={isActive ? pdfPage.onRemoveRedaction : NOOP_REMOVE_REDACTION}
                onRemoveStamp={isActive ? pdfPage.onRemoveStamp : NOOP_REMOVE_STAMP}
                onRemoveShape={isActive ? pdfPage.onRemoveShape : NOOP_REMOVE_SHAPE}
                onRemoveInkStroke={isActive ? pdfPage.onRemoveInkStroke : NOOP_REMOVE_INK}
                onRemoveTextNote={isActive ? pdfPage.onRemoveTextNote : NOOP_REMOVE_NOTE}
              />
            ) : showPage ? (
              <p className="muted page-loading">Loading page {page + 1}…</p>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}
