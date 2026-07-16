import { memo } from 'react';
import type React from 'react';
import type { ShapeKind } from '../app/constants';
import { pageHeightPtFor } from '../app/constants';
import type {
  AnnotationData,
  FormFieldData,
  PageTextEdit,
  PageVectorEdit,
  PdfPageSize,
} from '../app/types';
import type { PageTextRun } from '../pdf/useTextLayerLoader';
import type { TextLineInfo } from './useTextEditRun';
import { PdfPageOverlays } from './PdfPageOverlays';
import { TextLayer } from './TextLayer';
import { TextEditOverlay } from './TextEditOverlay';
import { RichTextEditOverlay } from './RichTextEditOverlay';
import { ImageSelectionOverlay } from './ImageSelectionOverlay';
import { ParagraphSelectionOverlay } from './ParagraphSelectionOverlay';
import type { PdfEditState } from '../app/usePdfEditState';

type PdfPageViewProps = {
  zoom: number;
  imageSrc: string | null;
  pageContainerRef?: React.RefObject<HTMLDivElement | null>;
  textRuns?: PageTextRun[];
  textLayerInteractive?: boolean;
  imgRef?: React.RefObject<HTMLImageElement | null>;
  onImageLoad?: () => void;
  highlightMode: boolean;
  noteMode: boolean;
  drawMode: boolean;
  shapeMode: boolean;
  stampMode: boolean;
  redactMode: boolean;
  imageInsertMode: boolean;
  textEditMode: boolean;
  editTextRunMode: boolean;
  vectorEditMode: boolean;
  formAddMode: boolean;
  onPageClick?: (e: React.MouseEvent) => void;
  onMouseDown?: (e: React.MouseEvent) => void;
  onMouseMove?: (e: React.MouseEvent) => void;
  onMouseUp?: (e: React.MouseEvent) => void;
  activeSearchRect: [number, number, number, number] | null;
  annotations: AnnotationData[];
  shapeKind: ShapeKind;
  drawing: boolean;
  highlightStart: { x: number; y: number } | null;
  highlightRect: { x: number; y: number; w: number; h: number } | null;
  shapeLineEnd: { x: number; y: number } | null;
  inkDraft: number[];
  pageTextEdits: PageTextEdit[];
  pageVectorEdits: PageVectorEdit[];
  showFormsPanel: boolean;
  formFields: FormFieldData[];
  currentPage: number;
  pageSizes?: PdfPageSize[];
  onRemoveHighlight: (index: number) => void;
  onRemoveRedaction: (index: number) => void;
  onRemoveStamp: (kind: 'text' | 'image', index: number) => void;
  onRemoveShape: (kind: 'Square' | 'Circle' | 'Line', index: number) => void;
  onRemoveInkStroke: (index: number) => void;
  onRemoveTextNote: (index: number) => void;
  textEditActiveRun?: PageTextRun | null;
  textEditActiveLine?: TextLineInfo | null;
  textEditDraft?: string;
  onTextEditDraftChange?: (value: string) => void;
  onApplyTextEdit?: () => void;
  onCancelTextEdit?: () => void;
  pdfEdit?: PdfEditState | null;
};

function PdfPageViewInner({
  zoom,
  imageSrc,
  pageContainerRef,
  textRuns = [],
  textLayerInteractive = false,
  imgRef,
  onImageLoad,
  highlightMode,
  noteMode,
  drawMode,
  shapeMode,
  stampMode,
  redactMode,
  imageInsertMode,
  textEditMode,
  editTextRunMode,
  vectorEditMode,
  formAddMode,
  onPageClick,
  onMouseDown,
  onMouseMove,
  onMouseUp,
  activeSearchRect,
  annotations,
  shapeKind,
  drawing,
  highlightStart,
  highlightRect,
  shapeLineEnd,
  inkDraft,
  pageTextEdits,
  pageVectorEdits,
  showFormsPanel,
  formFields,
  currentPage,
  pageSizes,
  onRemoveHighlight,
  onRemoveRedaction,
  onRemoveStamp,
  onRemoveShape,
  onRemoveInkStroke,
  onRemoveTextNote,
  textEditActiveRun,
  textEditActiveLine,
  textEditDraft = '',
  onTextEditDraftChange,
  onApplyTextEdit,
  onCancelTextEdit,
  pdfEdit,
}: PdfPageViewProps) {
  const cursorClass = [
    highlightMode ? 'highlight-cursor' : '',
    noteMode ? 'note-cursor' : '',
    drawMode ? 'draw-cursor' : '',
    shapeMode ? 'shape-cursor' : '',
    stampMode ? 'stamp-cursor' : '',
    redactMode ? 'redact-cursor' : '',
    imageInsertMode ? 'image-insert-cursor' : '',
    textEditMode ? 'text-edit-cursor' : '',
    editTextRunMode ? 'text-edit-cursor' : '',
    textEditActiveRun ? 'text-edit-cursor' : '',
    vectorEditMode ? 'vector-edit-cursor' : '',
    formAddMode ? 'form-add-cursor' : '',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div
      className={`page-container ${cursorClass}`.trim()}
      data-testid="page-container"
      onClick={onPageClick}
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
      onMouseLeave={onMouseUp}
    >
      {imageSrc ? (
        <div
          className="page-scale"
          style={{ transform: `scale(${zoom})`, transformOrigin: 'top center' }}
        >
          <div
            ref={pageContainerRef}
            style={{ position: 'relative', display: 'inline-block' }}
          >
            <img
              ref={imgRef}
              src={imageSrc}
              alt="PDF Page"
              className="page-image"
              draggable={false}
              onLoad={onImageLoad}
            />
            <TextLayer runs={textRuns} interactive={textLayerInteractive} />
            {(textEditActiveRun || textEditActiveLine) &&
              onTextEditDraftChange &&
              onApplyTextEdit &&
              onCancelTextEdit && (
                <TextEditOverlay
                  target={textEditActiveLine ?? textEditActiveRun!}
                  zoom={zoom}
                  draft={textEditDraft}
                  onDraftChange={onTextEditDraftChange}
                  onApply={onApplyTextEdit}
                  onCancel={onCancelTextEdit}
                />
              )}
            {pdfEdit?.textDraft &&
              pdfEdit.textDraft.pageIndex === currentPage && (
                <RichTextEditOverlay
                  draft={pdfEdit.textDraft}
                  zoom={zoom}
                  pageHeightPt={pageHeightPtFor(pageSizes?.[currentPage])}
                  onUpdate={pdfEdit.onUpdate}
                  onApply={pdfEdit.onApply}
                  onCancel={pdfEdit.onCancel}
                />
              )}
            {pdfEdit?.paragraphDraft &&
              pdfEdit.paragraphDraft.pageIndex === currentPage &&
              !pdfEdit.paragraphEditing && (
                <ParagraphSelectionOverlay
                  draft={pdfEdit.paragraphDraft}
                  zoom={zoom}
                  onUpdate={pdfEdit.onUpdateParagraph}
                  onEnterEdit={pdfEdit.enterParagraphTextEdit}
                  onDelete={pdfEdit.onDeleteParagraph}
                  onCancel={pdfEdit.onCancel}
                />
              )}
            {pdfEdit?.paragraphDraft &&
              pdfEdit.paragraphDraft.pageIndex === currentPage &&
              pdfEdit.paragraphEditing && (
                <RichTextEditOverlay
                  draft={pdfEdit.paragraphDraft}
                  zoom={zoom}
                  pageHeightPt={pageHeightPtFor(pageSizes?.[currentPage])}
                  onUpdate={pdfEdit.onUpdateParagraph}
                  onApply={pdfEdit.onApply}
                  onCancel={pdfEdit.onCancel}
                />
              )}
            {pdfEdit?.imageDraft &&
              pdfEdit.imageDraft.pageIndex === currentPage && (
                <ImageSelectionOverlay
                  draft={pdfEdit.imageDraft}
                  zoom={zoom}
                  onUpdate={({ rect, rotation }) => {
                    pdfEdit.onUpdateImageRect(rect);
                    pdfEdit.onUpdateImageRotation(rotation);
                  }}
                  onApply={pdfEdit.onApply}
                  onDelete={pdfEdit.onDeleteImage}
                  onCancel={pdfEdit.onCancel}
                />
              )}
            <PdfPageOverlays
              activeSearchRect={activeSearchRect}
              annotations={annotations}
              highlightMode={highlightMode}
              noteMode={noteMode}
              drawMode={drawMode}
              shapeMode={shapeMode}
              stampMode={stampMode}
              redactMode={redactMode}
              imageInsertMode={imageInsertMode}
              vectorEditMode={vectorEditMode}
              formAddMode={formAddMode}
              shapeKind={shapeKind}
              drawing={drawing}
              highlightStart={highlightStart}
              highlightRect={highlightRect}
              shapeLineEnd={shapeLineEnd}
              inkDraft={inkDraft}
              pageTextEdits={pageTextEdits}
              pageVectorEdits={pageVectorEdits}
              showFormsPanel={showFormsPanel}
              formFields={formFields}
              currentPage={currentPage}
              onRemoveHighlight={onRemoveHighlight}
              onRemoveRedaction={onRemoveRedaction}
              onRemoveStamp={onRemoveStamp}
              onRemoveShape={onRemoveShape}
              onRemoveInkStroke={onRemoveInkStroke}
              onRemoveTextNote={onRemoveTextNote}
            />
          </div>
        </div>
      ) : (
        <p className="muted">No page rendered.</p>
      )}
    </div>
  );
}

export const PdfPageView = memo(PdfPageViewInner);
