import { useId } from 'react';
import type { ShapeKind } from '../app/constants';
import type { ShapeStyle } from '../app/usePdfEditState';
import type { AnnotationData, FormFieldData, PageTextEdit, PageVectorEdit } from '../app/types';
import { inkPointsToPolyline, shapeStrokeColor, stampPresetMeta } from '../app/utils';

type Point = { x: number; y: number };
type DragRect = { x: number; y: number; w: number; h: number };

function lineEndingMarker(ending: string | undefined, openArrowId: string, closedArrowId: string): string | undefined {
  if (ending === 'OpenArrow') return `url(#${openArrowId})`;
  if (ending === 'ClosedArrow') return `url(#${closedArrowId})`;
  return undefined;
}

export type PdfPageOverlaysProps = {
  activeSearchRect: [number, number, number, number] | null;
  annotations: AnnotationData[];
  highlightMode: boolean;
  noteMode: boolean;
  drawMode: boolean;
  shapeMode: boolean;
  stampMode: boolean;
  redactMode: boolean;
  imageInsertMode: boolean;
  vectorEditMode: boolean;
  editShapeStyle?: ShapeStyle;
  formAddMode: boolean;
  shapeKind: ShapeKind;
  drawing: boolean;
  highlightStart: Point | null;
  highlightRect: DragRect | null;
  shapeLineEnd: Point | null;
  inkDraft: number[];
  pageTextEdits: PageTextEdit[];
  pageVectorEdits: PageVectorEdit[];
  showFormsPanel: boolean;
  formFields: FormFieldData[];
  currentPage: number;
  viewerWidth: number;
  viewerHeight: number;
  onRemoveHighlight: (index: number) => void;
  onRemoveRedaction: (index: number) => void;
  onRemoveStamp: (kind: 'text' | 'image', index: number) => void;
  onRemoveShape: (kind: 'Square' | 'Circle' | 'Line', index: number) => void;
  onRemoveInkStroke: (index: number) => void;
  onRemoveTextNote: (index: number) => void;
};

export function PdfPageOverlays({
  activeSearchRect,
  annotations,
  highlightMode,
  noteMode,
  drawMode,
  shapeMode,
  stampMode,
  redactMode,
  imageInsertMode,
  vectorEditMode,
  editShapeStyle,
  formAddMode,
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
  viewerWidth,
  viewerHeight,
  onRemoveHighlight,
  onRemoveRedaction,
  onRemoveStamp,
  onRemoveShape,
  onRemoveInkStroke,
  onRemoveTextNote,
}: PdfPageOverlaysProps) {
  const markerScope = useId().replaceAll(':', '');
  const openArrowId = `annotation-open-arrow-${markerScope}`;
  const closedArrowId = `annotation-closed-arrow-${markerScope}`;
  const editStroke = editShapeStyle
    ? `rgb(${editShapeStyle.strokeColor.r},${editShapeStyle.strokeColor.g},${editShapeStyle.strokeColor.b})`
    : 'rgb(255,0,0)';
  const draftStroke = vectorEditMode ? editStroke : 'rgb(255,0,0)';
  const draftStrokeWidth = vectorEditMode ? editShapeStyle?.strokeWidth ?? 2 : 2;
  return (
    <>
{activeSearchRect && (
  <div
    className="search-highlight-overlay"
    style={{
      left: activeSearchRect[0],
      top: activeSearchRect[1],
      width: activeSearchRect[2] - activeSearchRect[0],
      height: activeSearchRect[3] - activeSearchRect[1],
    }}
  />
)}
{/* Existing highlights */}
{annotations.filter((a) => a.subtype === 'Highlight').map((a, i) => (
  <div
    key={i}
    className="highlight-overlay"
    title={highlightMode ? 'Click to remove' : undefined}
    onClick={highlightMode ? (e) => { e.stopPropagation(); onRemoveHighlight(i); } : undefined}
    style={{
      left: a.rect[0],
      top: a.rect[1],
      width: a.rect[2] - a.rect[0],
      height: a.rect[3] - a.rect[1],
      backgroundColor: a.color
        ? `rgba(${a.color[0] * 255},${a.color[1] * 255},${a.color[2] * 255},0.3)`
        : 'rgba(255,255,0,0.3)',
      pointerEvents: highlightMode ? 'auto' : 'none',
      cursor: highlightMode ? 'pointer' : 'default',
    }}
  />
))}
{/* Redaction boxes */}
{annotations.filter((a) => a.is_redaction).map((a, i) => (
  <div
    key={`redact-${i}`}
    className="redaction-overlay"
    title={redactMode ? 'Click to remove' : undefined}
    onClick={redactMode ? (e) => { e.stopPropagation(); onRemoveRedaction(i); } : undefined}
    style={{
      left: a.rect[0],
      top: a.rect[1],
      width: a.rect[2] - a.rect[0],
      height: a.rect[3] - a.rect[1],
      pointerEvents: redactMode ? 'auto' : 'none',
      cursor: redactMode ? 'pointer' : 'default',
    }}
  />
))}
{/* Text stamps */}
{annotations.filter((a) => a.stamp_kind === 'text').map((a, i) => {
  const meta = stampPresetMeta(a.stamp_preset);
  return (
    <div
      key={`text-stamp-${i}`}
      className="text-stamp-overlay"
      title={stampMode ? 'Click to remove' : undefined}
      onClick={stampMode ? (e) => { e.stopPropagation(); onRemoveStamp('text', i); } : undefined}
      style={{
        left: a.rect[0],
        top: a.rect[1],
        width: a.rect[2] - a.rect[0],
        height: a.rect[3] - a.rect[1],
        borderColor: meta?.color ?? '#333',
        color: meta?.color ?? '#333',
        pointerEvents: stampMode ? 'auto' : 'none',
        cursor: stampMode ? 'pointer' : 'default',
      }}
    >
      {a.contents ?? meta?.label}
    </div>
  );
})}
{/* Image stamps */}
{annotations.filter((a) => a.stamp_kind === 'image').map((a, i) => {
  const meta = stampPresetMeta(a.stamp_preset);
  return (
    <div
      key={`image-stamp-${i}`}
      className="image-stamp-overlay"
      title={stampMode ? 'Click to remove' : undefined}
      onClick={stampMode ? (e) => { e.stopPropagation(); onRemoveStamp('image', i); } : undefined}
      style={{
        left: a.rect[0],
        top: a.rect[1],
        width: a.rect[2] - a.rect[0],
        height: a.rect[3] - a.rect[1],
        backgroundColor: meta?.color ?? '#666',
        pointerEvents: stampMode ? 'auto' : 'none',
        cursor: stampMode ? 'pointer' : 'default',
      }}
    >
      {meta?.label}
    </div>
  );
})}
{/* Shape outlines */}
{annotations.filter((a) => a.subtype === 'Square' && !a.is_redaction).map((a, i) => (
  <div
    key={`square-${i}`}
    className="shape-overlay shape-square"
    title={shapeMode ? 'Click to remove' : undefined}
    onClick={shapeMode ? (e) => { e.stopPropagation(); onRemoveShape('Square', i); } : undefined}
    style={{
      left: a.rect[0],
      top: a.rect[1],
      width: a.rect[2] - a.rect[0],
      height: a.rect[3] - a.rect[1],
      borderColor: shapeStrokeColor(a.color),
      borderWidth: a.stroke_width ?? 2,
      pointerEvents: shapeMode ? 'auto' : 'none',
      cursor: shapeMode ? 'pointer' : 'default',
    }}
  />
))}
{annotations.filter((a) => a.subtype === 'Circle').map((a, i) => (
  <div
    key={`circle-${i}`}
    className="shape-overlay shape-circle"
    title={shapeMode ? 'Click to remove' : undefined}
    onClick={shapeMode ? (e) => { e.stopPropagation(); onRemoveShape('Circle', i); } : undefined}
    style={{
      left: a.rect[0],
      top: a.rect[1],
      width: a.rect[2] - a.rect[0],
      height: a.rect[3] - a.rect[1],
      borderColor: shapeStrokeColor(a.color),
      borderWidth: a.stroke_width ?? 2,
      pointerEvents: shapeMode ? 'auto' : 'none',
      cursor: shapeMode ? 'pointer' : 'default',
    }}
  />
))}
{/* Freehand ink strokes and line shapes */}
<svg
  className="ink-overlay"
  viewBox={`0 0 ${viewerWidth} ${viewerHeight}`}
  aria-hidden={!drawMode && !shapeMode && !vectorEditMode}
>
  <defs>
    <marker id={openArrowId} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10" fill="none" stroke="context-stroke" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </marker>
    <marker id={closedArrowId} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 Z" fill="context-stroke" stroke="context-stroke" strokeWidth="1" />
    </marker>
  </defs>
  {annotations.filter((a) => a.subtype === 'Line' && a.line_endpoints).map((a, i) => {
    const [x1, y1, x2, y2] = a.line_endpoints!;
    const stroke = shapeStrokeColor(a.color);
    const markerStart = lineEndingMarker(a.line_endings?.[0], openArrowId, closedArrowId);
    const markerEnd = lineEndingMarker(a.line_endings?.[1], openArrowId, closedArrowId);
    return (
      <g key={`line-${i}`}>
        {shapeMode && (
          <line
            x1={x1}
            y1={y1}
            x2={x2}
            y2={y2}
            stroke="transparent"
            strokeWidth={12}
            strokeLinecap="round"
            style={{ pointerEvents: 'stroke', cursor: 'pointer' }}
            onMouseDown={(e) => e.stopPropagation()}
            onClick={(e) => { e.stopPropagation(); onRemoveShape('Line', i); }}
          />
        )}
        <line
          x1={x1}
          y1={y1}
          x2={x2}
          y2={y2}
          stroke={stroke}
          strokeWidth={a.stroke_width ?? 2}
          strokeLinecap="round"
          markerStart={markerStart}
          markerEnd={markerEnd}
          style={{ pointerEvents: 'none' }}
        />
      </g>
    );
  })}
  {annotations.filter((a) => a.subtype === 'Ink').map((a, i) => {
    const points = inkPointsToPolyline(a.ink_points);
    const stroke = a.color
      ? `rgb(${a.color[0] * 255},${a.color[1] * 255},${a.color[2] * 255})`
      : 'rgb(0,0,255)';
    return (
      <g key={`ink-${i}`}>
        {drawMode && (
          <polyline
            points={points}
            fill="none"
            stroke="transparent"
            strokeWidth={12}
            strokeLinecap="round"
            strokeLinejoin="round"
            style={{ pointerEvents: 'stroke', cursor: 'pointer' }}
            onMouseDown={(e) => e.stopPropagation()}
            onClick={(e) => { e.stopPropagation(); onRemoveInkStroke(i); }}
          />
        )}
        <polyline
          points={points}
          fill="none"
          stroke={stroke}
          strokeWidth={2}
          strokeLinecap="round"
          strokeLinejoin="round"
          style={{ pointerEvents: 'none' }}
        />
      </g>
    );
  })}
  {inkDraft.length >= 2 && (
    <polyline
      points={inkPointsToPolyline(inkDraft)}
      fill="none"
      stroke="rgb(0,0,255)"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{ pointerEvents: 'none', opacity: 0.75 }}
    />
  )}
  {(shapeMode || vectorEditMode) && drawing && highlightStart && (shapeKind === 'line' || shapeKind === 'arrow') && shapeLineEnd && (
    <line
      x1={highlightStart.x}
      y1={highlightStart.y}
      x2={shapeLineEnd.x}
      y2={shapeLineEnd.y}
      stroke={draftStroke}
      strokeWidth={draftStrokeWidth}
      strokeLinecap="round"
      markerEnd={shapeKind === 'arrow' ? `url(#${openArrowId})` : undefined}
      style={{ pointerEvents: 'none', opacity: 0.75 }}
    />
  )}
</svg>
{/* Sticky text notes */}
{annotations.filter((a) => a.subtype === 'Text').map((a, i) => (
  <div
    key={`note-${i}`}
    className="text-note-overlay"
    title={noteMode ? 'Click to remove' : (a.contents ?? undefined)}
    onClick={noteMode ? (e) => { e.stopPropagation(); onRemoveTextNote(i); } : undefined}
    style={{
      left: a.rect[0],
      top: a.rect[1],
      width: a.rect[2] - a.rect[0],
      height: a.rect[3] - a.rect[1],
      pointerEvents: noteMode ? 'auto' : 'none',
      cursor: noteMode ? 'pointer' : 'default',
    }}
  >
    {a.contents}
  </div>
))}
{pageTextEdits.map((edit) => (
  <div
    key={`page-text-${edit.index}`}
    className="page-text-edit-overlay"
    style={{ left: edit.x, top: edit.y }}
    title={edit.text}
  >
    {edit.text}
  </div>
))}
{pageVectorEdits.map((edit) => (
  <div
    key={`page-vector-${edit.index}`}
    className="page-vector-edit-overlay"
    style={{
      left: edit.x,
      top: edit.y,
      width: edit.width,
      height: edit.height,
    }}
  />
))}
{/* Current highlight drag */}
{highlightRect && highlightRect.w > 0 && highlightRect.h > 0 && highlightMode && (
  <div
    className="highlight-draft"
    style={{
      left: highlightRect.x,
      top: highlightRect.y,
      width: highlightRect.w,
      height: highlightRect.h,
    }}
  />
)}
{/* Current shape drag */}
{(shapeMode || vectorEditMode) && highlightRect && highlightRect.w > 0 && highlightRect.h > 0 && shapeKind !== 'line' && shapeKind !== 'arrow' && (
  <div
    className={`shape-draft ${shapeKind === 'circle' ? 'shape-circle' : 'shape-square'}`}
    style={{
      left: highlightRect.x,
      top: highlightRect.y,
      width: highlightRect.w,
      height: highlightRect.h,
      borderColor: draftStroke,
      borderWidth: draftStrokeWidth,
    }}
  />
)}
{redactMode && highlightRect && highlightRect.w > 0 && highlightRect.h > 0 && (
  <div
    className="redaction-draft"
    style={{
      left: highlightRect.x,
      top: highlightRect.y,
      width: highlightRect.w,
      height: highlightRect.h,
    }}
  />
)}
{imageInsertMode && highlightRect && highlightRect.w > 0 && highlightRect.h > 0 && (
  <div
    className="image-insert-draft"
    style={{
      left: highlightRect.x,
      top: highlightRect.y,
      width: highlightRect.w,
      height: highlightRect.h,
    }}
  />
)}
{formAddMode && highlightRect && highlightRect.w > 0 && highlightRect.h > 0 && (
  <div
    className="form-field-draft"
    style={{
      left: highlightRect.x,
      top: highlightRect.y,
      width: highlightRect.w,
      height: highlightRect.h,
    }}
  />
)}
{showFormsPanel && formFields
  .filter((field) => field.page_index === currentPage && field.rect)
  .map((field) => {
    const rect = field.rect!;
    return (
      <div
        key={field.name}
        className="form-field-overlay"
        style={{
          left: rect[0],
          top: rect[1],
          width: Math.max(0, rect[2] - rect[0]),
          height: Math.max(0, rect[3] - rect[1]),
        }}
        title={field.name}
      />
    );
  })}
    </>
  );
}
