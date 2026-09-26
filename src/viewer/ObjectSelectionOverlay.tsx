import { useCallback, useEffect, useRef, useState } from 'react';
import type { Rect, RgbColor, ShapeLineGeometry } from '../app/usePdfEditState';
import { VIEWER_PAGE_H, VIEWER_PAGE_W } from '../app/constants';
import {
  moveLineEndpoint,
  moveLineWithinPage,
  moveRectWithinPage,
  resizeRectWithinPage,
  type ResizeHandle,
} from './selectionGeometry';
import './ObjectSelectionOverlay.css';

type SelectionActions = {
  zoom: number;
  ariaLabel: string;
  onApply: () => void;
  onDelete: () => void;
  onCancel: () => void;
};

type BoxSelectionProps = SelectionActions & {
  kind: 'box';
  boxShape?: 'rectangle' | 'ellipse';
  rect: Rect;
  strokeColor?: RgbColor;
  strokeWidth?: number;
  onUpdate: (rect: Rect) => void;
};

type LineSelectionProps = SelectionActions & {
  kind: 'line';
  line: ShapeLineGeometry;
  arrowEnd?: boolean;
  strokeColor?: RgbColor;
  strokeWidth?: number;
  onUpdate: (line: ShapeLineGeometry) => void;
};

type ObjectSelectionOverlayProps = BoxSelectionProps | LineSelectionProps;
type BoxDragKind = 'move' | ResizeHandle;
type LineDragKind = 'move' | 'start' | 'end';

const MIN_BOX_SIZE = 8;
const BOX_HANDLES: ResizeHandle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

function rgbCss(color: RgbColor | undefined): string {
  return color ? `rgb(${color.r}, ${color.g}, ${color.b})` : '#2563eb';
}

function keyDelta(e: React.KeyboardEvent): { dx: number; dy: number } | null {
  if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) return null;
  const step = e.shiftKey ? 10 : 1;
  return {
    dx: e.key === 'ArrowLeft' ? -step : e.key === 'ArrowRight' ? step : 0,
    dy: e.key === 'ArrowUp' ? -step : e.key === 'ArrowDown' ? step : 0,
  };
}

function handleActionKey(e: React.KeyboardEvent, actions: SelectionActions, nudge: (dx: number, dy: number) => void) {
  const delta = keyDelta(e);
  if (delta) nudge(delta.dx, delta.dy);
  else if (e.key === 'Escape') actions.onCancel();
  else if (e.key === 'Enter') actions.onApply();
  else if (e.key === 'Delete' || e.key === 'Backspace') actions.onDelete();
  else return;
  e.preventDefault();
  e.stopPropagation();
}

function BoxSelectionOverlay(props: BoxSelectionProps) {
  const { onUpdate } = props;
  const rootRef = useRef<HTMLDivElement>(null);
  const rectRef = useRef(props.rect);
  const [rect, setRect] = useState(props.rect);
  const [dragging, setDragging] = useState<BoxDragKind | null>(null);
  const startPointerRef = useRef<{ x: number; y: number } | null>(null);
  const startRectRef = useRef<Rect | null>(null);

  const updateRect = useCallback((next: Rect) => {
    rectRef.current = next;
    setRect(next);
  }, []);

  useEffect(() => {
    if (!dragging) updateRect(props.rect);
  }, [dragging, props.rect, updateRect]);

  useEffect(() => {
    rootRef.current?.focus();
  }, []);

  const startDrag = useCallback((e: React.MouseEvent, kind: BoxDragKind) => {
    e.preventDefault();
    e.stopPropagation();
    rootRef.current?.focus();
    setDragging(kind);
    startPointerRef.current = { x: e.clientX, y: e.clientY };
    startRectRef.current = rectRef.current;
  }, []);

  useEffect(() => {
    if (!dragging || !startPointerRef.current || !startRectRef.current) return;
    const startPointer = startPointerRef.current;
    const startRect = startRectRef.current;
    const onMouseMove = (e: MouseEvent) => {
      const dx = (e.clientX - startPointer.x) / props.zoom;
      const dy = (e.clientY - startPointer.y) / props.zoom;
      updateRect(
        dragging === 'move'
          ? moveRectWithinPage(startRect, startRect.x + dx, startRect.y + dy)
          : resizeRectWithinPage(startRect, dragging, dx, dy, MIN_BOX_SIZE),
      );
    };
    const onMouseUp = () => {
      onUpdate(rectRef.current);
      setDragging(null);
      startPointerRef.current = null;
      startRectRef.current = null;
    };
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp, { once: true });
    return () => {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };
  }, [dragging, onUpdate, props.zoom, updateRect]);

  const nudge = useCallback((dx: number, dy: number) => {
    const next = moveRectWithinPage(rectRef.current, rectRef.current.x + dx, rectRef.current.y + dy);
    updateRect(next);
    onUpdate(next);
  }, [onUpdate, updateRect]);

  const resizeWithKeyboard = useCallback((e: React.KeyboardEvent, handle: ResizeHandle) => {
    const delta = keyDelta(e);
    if (!delta) return;
    const next = resizeRectWithinPage(rectRef.current, handle, delta.dx, delta.dy, MIN_BOX_SIZE);
    updateRect(next);
    onUpdate(next);
    e.preventDefault();
    e.stopPropagation();
  }, [onUpdate, updateRect]);

  return (
    <div
      ref={rootRef}
      tabIndex={0}
      role="region"
      aria-label={props.ariaLabel}
      className={`object-selection-overlay${dragging === 'move' ? ' moving' : ''}`}
      style={{ left: rect.x, top: rect.y, width: rect.w, height: rect.h }}
      onMouseDown={(e) => startDrag(e, 'move')}
      onClick={(e) => e.stopPropagation()}
      onKeyDown={(e) => handleActionKey(e, props, nudge)}
    >
      <div
        className="object-selection-frame"
        style={{
          borderColor: rgbCss(props.strokeColor),
          borderWidth: props.strokeWidth ?? 2,
          borderRadius: props.boxShape === 'ellipse' ? '50%' : undefined,
        }}
      />
      {BOX_HANDLES.map((handle) => (
        <button
          key={handle}
          type="button"
          aria-label={`Resize ${handle}`}
          className={`object-selection-handle object-selection-handle-${handle}`}
          onMouseDown={(e) => startDrag(e, handle)}
          onKeyDown={(e) => resizeWithKeyboard(e, handle)}
        />
      ))}
    </div>
  );
}

function LineSelectionOverlay(props: LineSelectionProps) {
  const { onUpdate } = props;
  const rootRef = useRef<SVGSVGElement>(null);
  const lineRef = useRef(props.line);
  const [line, setLine] = useState(props.line);
  const [dragging, setDragging] = useState<LineDragKind | null>(null);
  const startPointerRef = useRef<{ x: number; y: number } | null>(null);
  const startLineRef = useRef<ShapeLineGeometry | null>(null);

  const updateLine = useCallback((next: ShapeLineGeometry) => {
    lineRef.current = next;
    setLine(next);
  }, []);

  useEffect(() => {
    if (!dragging) updateLine(props.line);
  }, [dragging, props.line, updateLine]);

  useEffect(() => {
    rootRef.current?.focus();
  }, []);

  const startDrag = useCallback((e: React.MouseEvent, kind: LineDragKind) => {
    e.preventDefault();
    e.stopPropagation();
    rootRef.current?.focus();
    setDragging(kind);
    startPointerRef.current = { x: e.clientX, y: e.clientY };
    startLineRef.current = lineRef.current;
  }, []);

  useEffect(() => {
    if (!dragging || !startPointerRef.current || !startLineRef.current) return;
    const startPointer = startPointerRef.current;
    const startLine = startLineRef.current;
    const onMouseMove = (e: MouseEvent) => {
      const dx = (e.clientX - startPointer.x) / props.zoom;
      const dy = (e.clientY - startPointer.y) / props.zoom;
      updateLine(
        dragging === 'move'
          ? moveLineWithinPage(startLine, dx, dy)
          : moveLineEndpoint(startLine, dragging, dx, dy),
      );
    };
    const onMouseUp = () => {
      onUpdate(lineRef.current);
      setDragging(null);
      startPointerRef.current = null;
      startLineRef.current = null;
    };
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp, { once: true });
    return () => {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };
  }, [dragging, onUpdate, props.zoom, updateLine]);

  const nudge = useCallback((dx: number, dy: number) => {
    const next = moveLineWithinPage(lineRef.current, dx, dy);
    updateLine(next);
    onUpdate(next);
  }, [onUpdate, updateLine]);

  const nudgeEndpoint = useCallback((e: React.KeyboardEvent<SVGCircleElement>, endpoint: 'start' | 'end') => {
    const delta = keyDelta(e);
    if (!delta) return;
    const next = moveLineEndpoint(lineRef.current, endpoint, delta.dx, delta.dy);
    updateLine(next);
    onUpdate(next);
    e.preventDefault();
    e.stopPropagation();
  }, [onUpdate, updateLine]);

  return (
    <svg
      ref={rootRef}
      tabIndex={0}
      role="region"
      aria-label={props.ariaLabel}
      className="object-selection-line-layer"
      viewBox={`0 0 ${VIEWER_PAGE_W} ${VIEWER_PAGE_H}`}
      style={{ color: rgbCss(props.strokeColor) }}
      onKeyDown={(e) => handleActionKey(e, props, nudge)}
      onClick={(e) => e.stopPropagation()}
    >
      <defs>
        <marker id="object-selection-arrow-end" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto">
          <path d="M 0 0 L 10 5 L 0 10" className="object-selection-arrow-head" />
        </marker>
      </defs>
      <line
        x1={line.x1}
        y1={line.y1}
        x2={line.x2}
        y2={line.y2}
        className="object-selection-line-hit"
        onMouseDown={(e) => startDrag(e, 'move')}
      />
      <line
        x1={line.x1}
        y1={line.y1}
        x2={line.x2}
        y2={line.y2}
        className="object-selection-line"
        markerEnd={props.arrowEnd ? 'url(#object-selection-arrow-end)' : undefined}
        strokeWidth={props.strokeWidth ?? 2}
      />
      <circle
        cx={line.x1}
        cy={line.y1}
        r={6}
        tabIndex={0}
        role="button"
        aria-label="Move line start"
        className="object-selection-line-handle"
        onMouseDown={(e) => startDrag(e, 'start')}
        onKeyDown={(e) => nudgeEndpoint(e, 'start')}
      />
      <circle
        cx={line.x2}
        cy={line.y2}
        r={6}
        tabIndex={0}
        role="button"
        aria-label="Move line end"
        className="object-selection-line-handle"
        onMouseDown={(e) => startDrag(e, 'end')}
        onKeyDown={(e) => nudgeEndpoint(e, 'end')}
      />
    </svg>
  );
}

export function ObjectSelectionOverlay(props: ObjectSelectionOverlayProps) {
  return props.kind === 'box' ? <BoxSelectionOverlay {...props} /> : <LineSelectionOverlay {...props} />;
}
