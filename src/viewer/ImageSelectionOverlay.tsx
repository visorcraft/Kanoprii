import { useCallback, useEffect, useRef, useState } from 'react';
import type { ImageEditDraft, Rect } from '../app/usePdfEditState';
import { VIEWER_PAGE_H, VIEWER_PAGE_W } from '../app/constants';
import './ImageSelectionOverlay.css';

type ImageSelectionOverlayProps = {
  draft: ImageEditDraft;
  zoom: number;
  onUpdate: (payload: { rect: Rect; rotation: number }) => void;
  onApply: () => void;
  onDelete: () => void;
  onCancel: () => void;
};

const MIN_SIZE = 20;
const RESIZE_HANDLES = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'] as const;
type ResizeHandle = (typeof RESIZE_HANDLES)[number];
type DragKind = 'move' | 'rotate' | ResizeHandle;

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function clampRect(rect: Rect): Rect {
  const x = clamp(rect.x, 0, VIEWER_PAGE_W - MIN_SIZE);
  const y = clamp(rect.y, 0, VIEWER_PAGE_H - MIN_SIZE);
  const maxW = VIEWER_PAGE_W - x;
  const maxH = VIEWER_PAGE_H - y;
  const w = clamp(rect.w, MIN_SIZE, maxW);
  const h = clamp(rect.h, MIN_SIZE, maxH);
  return { x, y, w, h };
}

function resizeRect(start: Rect, kind: ResizeHandle, dx: number, dy: number, preserveRatio: boolean): Rect {
  let left = start.x;
  let top = start.y;
  let right = start.x + start.w;
  let bottom = start.y + start.h;
  if (kind.includes('w')) left = clamp(left + dx, 0, right - MIN_SIZE);
  if (kind.includes('e')) right = clamp(right + dx, left + MIN_SIZE, VIEWER_PAGE_W);
  if (kind.includes('n')) top = clamp(top + dy, 0, bottom - MIN_SIZE);
  if (kind.includes('s')) bottom = clamp(bottom + dy, top + MIN_SIZE, VIEWER_PAGE_H);

  if (preserveRatio && kind.length === 2 && start.h > 0) {
    const ratio = start.w / start.h;
    let w = right - left;
    let h = bottom - top;
    if (Math.abs(w - start.w) >= Math.abs(h - start.h) * ratio) h = w / ratio;
    else w = h * ratio;
    const maxW = kind.includes('w') ? start.x + start.w : VIEWER_PAGE_W - start.x;
    const maxH = kind.includes('n') ? start.y + start.h : VIEWER_PAGE_H - start.y;
    const scale = Math.min(1, maxW / w, maxH / h);
    w *= scale;
    h *= scale;
    left = kind.includes('w') ? start.x + start.w - w : start.x;
    top = kind.includes('n') ? start.y + start.h - h : start.y;
    right = left + w;
    bottom = top + h;
  }
  return { x: left, y: top, w: right - left, h: bottom - top };
}

export function ImageSelectionOverlay({
  draft,
  zoom,
  onUpdate,
  onApply,
  onDelete,
  onCancel,
}: ImageSelectionOverlayProps) {
  const [rect, setRect] = useState<Rect>(draft.pageRect);
  const [rotation, setRotation] = useState<number>(draft.rotation ?? 0);
  const [dragging, setDragging] = useState<DragKind | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const dragStartPosRef = useRef<{ x: number; y: number } | null>(null);
  const dragStartRectRef = useRef<Rect | null>(null);
  const rectRef = useRef(rect);
  const rotationRef = useRef(rotation);

  const updateRect = useCallback((next: Rect) => {
    rectRef.current = next;
    setRect(next);
  }, []);

  const updateRotation = useCallback((next: number) => {
    rotationRef.current = next;
    setRotation(next);
  }, []);

  useEffect(() => {
    updateRect(draft.pageRect);
  }, [draft.pageRect, updateRect]);

  useEffect(() => {
    updateRotation(draft.rotation ?? 0);
  }, [draft.rotation, updateRotation]);

  useEffect(() => {
    containerRef.current?.focus();
  }, []);

  useEffect(() => {
    if (dragging === 'rotate') {
      document.body.style.cursor = 'grabbing';
      return () => {
        document.body.style.cursor = '';
      };
    }
  }, [dragging]);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent, kind: DragKind) => {
      e.stopPropagation();
      e.preventDefault();
      containerRef.current?.focus();
      setDragging(kind);
      dragStartPosRef.current = { x: e.clientX, y: e.clientY };
      dragStartRectRef.current = rectRef.current;
    },
    []
  );

  useEffect(() => {
    if (!dragging) return;

    const startPos = dragStartPosRef.current;
    const startRect = dragStartRectRef.current;
    if (!startPos || !startRect) return;

    const onMouseMove = (e: MouseEvent) => {
      if (dragging === 'rotate') {
        const box = containerRef.current?.getBoundingClientRect();
        if (!box) return;
        const cx = box.left + box.width / 2;
        const cy = box.top + box.height / 2;
        const angle = Math.atan2(e.clientY - cy, e.clientX - cx);
        const degrees = -((angle * 180) / Math.PI + 90);
        const normalized = ((degrees % 360) + 360) % 360;
        updateRotation(normalized);
        return;
      }

      const dx = (e.clientX - startPos.x) / zoom;
      const dy = (e.clientY - startPos.y) / zoom;

      if (dragging === 'move') {
        updateRect(
          clampRect({
            x: startRect.x + dx,
            y: startRect.y + dy,
            w: startRect.w,
            h: startRect.h,
          })
        );
        return;
      }

      updateRect(resizeRect(startRect, dragging, dx, dy, e.shiftKey));
    };

    const onMouseUp = () => {
      setDragging(null);
      onUpdate({ rect: rectRef.current, rotation: rotationRef.current });
      dragStartPosRef.current = null;
      dragStartRectRef.current = null;
    };

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp, { once: true });
    return () => {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };
  }, [dragging, onUpdate, updateRect, updateRotation, zoom]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) {
        e.preventDefault();
        e.stopPropagation();
        const step = e.shiftKey ? 10 : 1;
        const dx = e.key === 'ArrowLeft' ? -step : e.key === 'ArrowRight' ? step : 0;
        const dy = e.key === 'ArrowUp' ? -step : e.key === 'ArrowDown' ? step : 0;
        const next = clampRect({ ...rectRef.current, x: rectRef.current.x + dx, y: rectRef.current.y + dy });
        updateRect(next);
        onUpdate({ rect: next, rotation: rotationRef.current });
      } else if (e.key === 'Delete' || e.key === 'Backspace') {
        e.preventDefault();
        onDelete();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        onCancel();
      } else if (e.key === 'Enter') {
        e.preventDefault();
        onApply();
      }
    },
    [onApply, onCancel, onDelete, onUpdate, updateRect]
  );

  return (
    <div
      ref={containerRef}
      tabIndex={-1}
      className={`image-selection-overlay ${dragging === 'move' ? 'move' : ''}`}
      style={{
        left: rect.x,
        top: rect.y,
        width: rect.w,
        height: rect.h,
      }}
      onMouseDown={(e) => handleMouseDown(e, 'move')}
      onKeyDown={handleKeyDown}
      role="dialog"
      aria-label="Image selection"
    >
      <div
        className="image-selection-inner"
        style={{ transform: `rotate(${-rotation}deg)` }}
        aria-hidden="true"
      />
      {RESIZE_HANDLES.map((kind) => (
        <button
          key={kind}
          type="button"
          className={`image-selection-handle image-selection-handle-${kind}`}
          onMouseDown={(e) => handleMouseDown(e, kind)}
          aria-label={`Resize image ${kind}`}
        />
      ))}
      <button
        type="button"
        className="image-selection-handle-rotate"
        onMouseDown={(e) => handleMouseDown(e, 'rotate')}
        aria-label="Rotate image"
      />
    </div>
  );
}
