import { useCallback, useEffect, useRef, useState } from 'react';
import type { ImageEditDraft, Rect } from '../app/usePdfEditState';
import { VIEWER_PAGE_H, VIEWER_PAGE_W } from '../app/constants';
import './ImageSelectionOverlay.css';

type ImageSelectionOverlayProps = {
  draft: ImageEditDraft;
  zoom: number;
  onUpdate: (rect: Rect) => void;
  onApply: () => void;
  onDelete: () => void;
  onCancel: () => void;
};

const MIN_SIZE = 20;

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

export function ImageSelectionOverlay({
  draft,
  zoom,
  onUpdate,
  onApply,
  onDelete,
  onCancel,
}: ImageSelectionOverlayProps) {
  const [rect, setRect] = useState<Rect>(draft.pageRect);
  const [dragging, setDragging] = useState<'move' | 'resize' | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const dragStartPosRef = useRef<{ x: number; y: number } | null>(null);
  const dragStartRectRef = useRef<Rect | null>(null);
  const rectRef = useRef(rect);

  const updateRect = useCallback((next: Rect) => {
    rectRef.current = next;
    setRect(next);
  }, []);

  useEffect(() => {
    updateRect(draft.pageRect);
  }, [draft.pageRect, updateRect]);

  useEffect(() => {
    containerRef.current?.focus();
  }, []);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent, kind: 'move' | 'resize') => {
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

      const ratio = startRect.h > 0 ? startRect.w / startRect.h : 0;
      let newW = Math.max(MIN_SIZE, startRect.w + dx);
      let newH = Math.max(MIN_SIZE, startRect.h + dy);

      if (e.shiftKey && ratio > 0) {
        if (Math.abs(dx) >= Math.abs(dy)) {
          newH = newW / ratio;
        } else {
          newW = newH * ratio;
        }
        if (newW < MIN_SIZE) {
          newW = MIN_SIZE;
          newH = newW / ratio;
        }
        if (newH < MIN_SIZE) {
          newH = MIN_SIZE;
          newW = newH * ratio;
        }
      }

      updateRect(
        clampRect({
          x: startRect.x,
          y: startRect.y,
          w: newW,
          h: newH,
        })
      );
    };

    const onMouseUp = () => {
      setDragging(null);
      onUpdate(rectRef.current);
      dragStartPosRef.current = null;
      dragStartRectRef.current = null;
    };

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp, { once: true });
    return () => {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };
  }, [dragging, onUpdate, updateRect, zoom]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Delete' || e.key === 'Backspace') {
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
    [onApply, onCancel, onDelete]
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
        className="image-selection-handle-br"
        onMouseDown={(e) => handleMouseDown(e, 'resize')}
        aria-label="Resize image"
      />
    </div>
  );
}
