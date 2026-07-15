import { useEffect, useRef, useState } from 'react';
import type { ParagraphEditDraft, Rect } from '../app/usePdfEditState';
import './ParagraphSelectionOverlay.css';

type HandleDir = 'nw' | 'n' | 'ne' | 'w' | 'e' | 'sw' | 's' | 'se';

type ParagraphSelectionOverlayProps = {
  draft: ParagraphEditDraft;
  onUpdate: (patch: Partial<ParagraphEditDraft>) => void;
  onEnterEdit: () => void;
  onDelete: () => void;
};

const MIN_SIZE = 20;

export function ParagraphSelectionOverlay({ draft, onUpdate, onEnterEdit, onDelete }: ParagraphSelectionOverlayProps) {
  const { pageRect } = draft;
  const [dragging, setDragging] = useState<{ kind: 'move' | HandleDir; start: Rect; pointerX: number; pointerY: number } | null>(null);
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    overlayRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!dragging) return;

    const onMove = (e: MouseEvent) => {
      const dx = e.clientX - dragging.pointerX;
      const dy = e.clientY - dragging.pointerY;
      const start = dragging.start;

      if (dragging.kind === 'move') {
        onUpdate({
          pageRect: {
            ...start,
            x: start.x + dx,
            y: start.y + dy,
          },
        });
        return;
      }

      const next: Rect = { ...start };
      const k = dragging.kind;
      if (k.includes('e')) {
        next.w = Math.max(MIN_SIZE, start.w + dx);
      }
      if (k.includes('s')) {
        next.h = Math.max(MIN_SIZE, start.h + dy);
      }
      if (k.includes('w')) {
        const newW = Math.max(MIN_SIZE, start.w - dx);
        next.x = start.x + start.w - newW;
        next.w = newW;
      }
      if (k.includes('n')) {
        const newH = Math.max(MIN_SIZE, start.h - dy);
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
  }, [dragging, onUpdate]);

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
        if (e.key === 'Enter') {
          e.preventDefault();
          onEnterEdit();
        } else if (e.key === 'Delete' || e.key === 'Backspace') {
          e.preventDefault();
          onDelete();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          // Let the consumer decide what Escape does; we just stop propagation.
        }
      }}
      tabIndex={0}
      role="region"
      aria-label="Paragraph selection"
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
