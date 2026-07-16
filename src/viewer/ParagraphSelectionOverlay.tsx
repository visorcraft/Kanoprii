import { useEffect, useRef, useState } from 'react';
import type { Rect } from '../app/usePdfEditState';
import { VIEWER_PAGE_H, VIEWER_PAGE_W } from '../app/constants';
import './ParagraphSelectionOverlay.css';

type HandleDir = 'nw' | 'n' | 'ne' | 'w' | 'e' | 'sw' | 's' | 'se';

type ParagraphSelectionOverlayProps = {
  draft: { pageRect: Rect };
  zoom: number;
  onUpdate: (patch: { pageRect: Rect }) => void;
  onEnterEdit?: () => void;
  onDelete: () => void;
  onCancel: () => void;
  ariaLabel?: string;
};

const MIN_SIZE = 20;

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export function ParagraphSelectionOverlay({ draft, zoom, onUpdate, onEnterEdit, onDelete, onCancel, ariaLabel = 'Paragraph selection' }: ParagraphSelectionOverlayProps) {
  const { pageRect } = draft;
  const [dragging, setDragging] = useState<{ kind: 'move' | HandleDir; start: Rect; pointerX: number; pointerY: number } | null>(null);
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    overlayRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!dragging) return;

    const onMove = (e: MouseEvent) => {
      const dx = (e.clientX - dragging.pointerX) / zoom;
      const dy = (e.clientY - dragging.pointerY) / zoom;
      const start = dragging.start;

      if (dragging.kind === 'move') {
        onUpdate({
          pageRect: {
            ...start,
            x: clamp(start.x + dx, 0, VIEWER_PAGE_W - start.w),
            y: clamp(start.y + dy, 0, VIEWER_PAGE_H - start.h),
          },
        });
        return;
      }

      const next: Rect = { ...start };
      const k = dragging.kind;
      if (k.includes('e')) {
        next.w = clamp(start.w + dx, MIN_SIZE, VIEWER_PAGE_W - start.x);
      }
      if (k.includes('s')) {
        next.h = clamp(start.h + dy, MIN_SIZE, VIEWER_PAGE_H - start.y);
      }
      if (k.includes('w')) {
        const newW = clamp(start.w - dx, MIN_SIZE, start.x + start.w);
        next.x = start.x + start.w - newW;
        next.w = newW;
      }
      if (k.includes('n')) {
        const newH = clamp(start.h - dy, MIN_SIZE, start.y + start.h);
        next.y = start.y + start.h - newH;
        next.h = newH;
      }
      onUpdate({ pageRect: next });
    };

    const onUp = () => setDragging(null);

    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
    return () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
  }, [dragging, onUpdate, zoom]);

  const startDrag = (kind: 'move' | HandleDir) => (e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    setDragging({
      kind,
      start: { ...pageRect },
      pointerX: e.clientX,
      pointerY: e.clientY,
    });
  };

  return (
    <div
      ref={overlayRef}
      className="paragraph-selection-overlay"
      style={{ left: pageRect.x, top: pageRect.y, width: pageRect.w, height: pageRect.h }}
      onDoubleClick={onEnterEdit}
      onMouseDown={startDrag('move')}
      onClick={(e) => e.stopPropagation()}
      onKeyDown={(e) => {
        if (e.key === 'Enter' && onEnterEdit) {
          e.preventDefault();
          onEnterEdit();
        } else if (e.key === 'Delete' || e.key === 'Backspace') {
          e.preventDefault();
          onDelete();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          onCancel();
        }
      }}
      tabIndex={0}
      role="region"
      aria-label={ariaLabel}
    >
      <div className="paragraph-selection-frame" />
      {(['nw', 'n', 'ne', 'w', 'e', 'sw', 's', 'se'] as HandleDir[]).map((handle) => (
        <div
          key={handle}
          className={`paragraph-handle paragraph-handle-${handle}`}
          onMouseDown={startDrag(handle)}
          onDoubleClick={(e) => e.stopPropagation()}
        />
      ))}
    </div>
  );
}
