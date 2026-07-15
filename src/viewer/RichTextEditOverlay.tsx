import { useCallback, useEffect, useRef, useState } from 'react';
import type { ParagraphEditDraft, Rect, TextEditDraft, TextStyle } from '../app/usePdfEditState';
import { VIEWER_PAGE_H, VIEWER_PAGE_W } from '../app/constants';
import { EditToolbar } from './EditToolbar';
import './RichTextEditOverlay.css';

type HandleKind = 'tl' | 'tr' | 'bl' | 'br';

type RichTextEditOverlayProps = {
  draft: TextEditDraft | ParagraphEditDraft;
  zoom?: number;
  onUpdate: (patch: { text?: string; style?: TextStyle; pageRect?: Rect }) => void;
  onApply: () => void;
  onCancel: () => void;
};

const MIN_W = 40;
const MIN_H = 20;

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function getFixedCorner(rect: Rect, kind: HandleKind): { x: number; y: number } {
  switch (kind) {
    case 'br':
      return { x: rect.x, y: rect.y };
    case 'tr':
      return { x: rect.x, y: rect.y + rect.h };
    case 'bl':
      return { x: rect.x + rect.w, y: rect.y };
    case 'tl':
      return { x: rect.x + rect.w, y: rect.y + rect.h };
  }
}

function clampResize(rect: Rect, fixed: { x: number; y: number }, kind: HandleKind): Rect {
  switch (kind) {
    case 'br': {
      const x = fixed.x;
      const y = fixed.y;
      const w = clamp(rect.w, MIN_W, VIEWER_PAGE_W - x);
      const h = clamp(rect.h, MIN_H, VIEWER_PAGE_H - y);
      return { x, y, w, h };
    }
    case 'tr': {
      const x = fixed.x;
      const y = clamp(rect.y, 0, fixed.y - MIN_H);
      const w = clamp(rect.w, MIN_W, VIEWER_PAGE_W - x);
      const h = fixed.y - y;
      return { x, y, w, h };
    }
    case 'bl': {
      const x = clamp(rect.x, 0, fixed.x - MIN_W);
      const y = fixed.y;
      const w = fixed.x - x;
      const h = clamp(rect.h, MIN_H, VIEWER_PAGE_H - y);
      return { x, y, w, h };
    }
    case 'tl': {
      const x = clamp(rect.x, 0, fixed.x - MIN_W);
      const y = clamp(rect.y, 0, fixed.y - MIN_H);
      const w = fixed.x - x;
      const h = fixed.y - y;
      return { x, y, w, h };
    }
  }
}

export function RichTextEditOverlay({
  draft,
  zoom = 1,
  onUpdate,
  onApply,
  onCancel,
}: RichTextEditOverlayProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [rect, setRect] = useState<Rect>(draft.pageRect);
  const [dragging, setDragging] = useState<HandleKind | null>(null);
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
    textareaRef.current?.focus();
    textareaRef.current?.select();
  }, []);

  const handleMouseDown = useCallback((e: React.MouseEvent, kind: HandleKind) => {
    e.stopPropagation();
    e.preventDefault();
    setDragging(kind);
    dragStartPosRef.current = { x: e.clientX, y: e.clientY };
    dragStartRectRef.current = rectRef.current;
  }, []);

  const handleHandleKeyDown = useCallback(
    (e: React.KeyboardEvent, kind: HandleKind) => {
      if (!['ArrowRight', 'ArrowLeft', 'ArrowUp', 'ArrowDown'].includes(e.key)) return;
      e.preventDefault();
      e.stopPropagation();
      const current = rectRef.current;
      const next: Rect = { ...current };
      switch (kind) {
        case 'br':
          if (e.key === 'ArrowRight') next.w += 5;
          if (e.key === 'ArrowLeft') next.w -= 5;
          if (e.key === 'ArrowDown') next.h += 5;
          if (e.key === 'ArrowUp') next.h -= 5;
          break;
        case 'tr':
          if (e.key === 'ArrowRight') next.w += 5;
          if (e.key === 'ArrowLeft') next.w -= 5;
          if (e.key === 'ArrowDown') {
            next.y += 5;
            next.h -= 5;
          }
          if (e.key === 'ArrowUp') {
            next.y -= 5;
            next.h += 5;
          }
          break;
        case 'bl':
          if (e.key === 'ArrowRight') {
            next.x += 5;
            next.w -= 5;
          }
          if (e.key === 'ArrowLeft') {
            next.x -= 5;
            next.w += 5;
          }
          if (e.key === 'ArrowDown') next.h += 5;
          if (e.key === 'ArrowUp') next.h -= 5;
          break;
        case 'tl':
          if (e.key === 'ArrowRight') {
            next.x += 5;
            next.w -= 5;
          }
          if (e.key === 'ArrowLeft') {
            next.x -= 5;
            next.w += 5;
          }
          if (e.key === 'ArrowDown') {
            next.y += 5;
            next.h -= 5;
          }
          if (e.key === 'ArrowUp') {
            next.y -= 5;
            next.h += 5;
          }
          break;
      }
      const fixed = getFixedCorner(current, kind);
      const clamped = clampResize(next, fixed, kind);
      updateRect(clamped);
      onUpdate({ pageRect: clamped });
    },
    [onUpdate, updateRect],
  );

  useEffect(() => {
    if (!dragging) return;

    const startPos = dragStartPosRef.current;
    const startRect = dragStartRectRef.current;
    if (!startPos || !startRect) return;

    const onMouseMove = (e: MouseEvent) => {
      const dx = (e.clientX - startPos.x) / zoom;
      const dy = (e.clientY - startPos.y) / zoom;

      let next: Rect;
      switch (dragging) {
        case 'br':
          next = { x: startRect.x, y: startRect.y, w: startRect.w + dx, h: startRect.h + dy };
          break;
        case 'tr':
          next = { x: startRect.x, y: startRect.y + dy, w: startRect.w + dx, h: startRect.h - dy };
          break;
        case 'bl':
          next = { x: startRect.x + dx, y: startRect.y, w: startRect.w - dx, h: startRect.h + dy };
          break;
        case 'tl':
          next = { x: startRect.x + dx, y: startRect.y + dy, w: startRect.w - dx, h: startRect.h - dy };
          break;
        default:
          return;
      }
      const fixed = getFixedCorner(startRect, dragging);
      updateRect(clampResize(next, fixed, dragging));
    };

    const onMouseUp = () => {
      setDragging(null);
      onUpdate({ pageRect: rectRef.current });
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

  const { style } = draft;

  const handleLabels: Record<HandleKind, string> = {
    tl: 'Resize text box top-left',
    tr: 'Resize text box top-right',
    bl: 'Resize text box bottom-left',
    br: 'Resize text box bottom-right',
  };

  const renderHandle = (kind: HandleKind) => (
    <div
      key={kind}
      className={`rich-text-resize-handle-${kind}`}
      role="button"
      tabIndex={0}
      aria-label={handleLabels[kind]}
      onMouseDown={(e) => handleMouseDown(e, kind)}
      onKeyDown={(e) => handleHandleKeyDown(e, kind)}
    />
  );

  return (
    <div
      className="rich-text-edit-overlay"
      style={{
        position: 'absolute',
        left: rect.x,
        top: rect.y,
        width: rect.w,
        height: rect.h,
      }}
    >
      <EditToolbar
        style={style}
        onChange={(patch) => onUpdate({ style: { ...style, ...patch } })}
        onApply={onApply}
        onCancel={onCancel}
      />
      <textarea
        ref={textareaRef}
        className="rich-text-edit-textarea"
        value={draft.text}
        onChange={(e) => onUpdate({ text: e.target.value })}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            onApply();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            onCancel();
          }
        }}
        style={{
          fontFamily:
            style.fontFamily === 'LiberationSans'
              ? 'Liberation Sans, sans-serif'
              : style.fontFamily,
          fontSize: style.fontSize,
          fontWeight: style.bold ? 'bold' : 'normal',
          fontStyle: style.italic ? 'italic' : 'normal',
          textDecoration: style.underline ? 'underline' : 'none',
          color: `rgb(${style.color.r}, ${style.color.g}, ${style.color.b})`,
          textAlign: style.align,
        }}
      />
      {renderHandle('tl')}
      {renderHandle('tr')}
      {renderHandle('bl')}
      {renderHandle('br')}
    </div>
  );
}
