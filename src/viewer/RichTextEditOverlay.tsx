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

function clampRect(rect: Rect): Rect {
  const x = clamp(rect.x, 0, VIEWER_PAGE_W - MIN_W);
  const y = clamp(rect.y, 0, VIEWER_PAGE_H - MIN_H);
  const maxW = VIEWER_PAGE_W - x;
  const maxH = VIEWER_PAGE_H - y;
  const w = clamp(rect.w, MIN_W, maxW);
  const h = clamp(rect.h, MIN_H, maxH);
  return { x, y, w, h };
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
      updateRect(clampRect(next));
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
      <div
        className="rich-text-resize-handle-tl"
        onMouseDown={(e) => handleMouseDown(e, 'tl')}
        aria-label="Resize text box top-left"
      />
      <div
        className="rich-text-resize-handle-tr"
        onMouseDown={(e) => handleMouseDown(e, 'tr')}
        aria-label="Resize text box top-right"
      />
      <div
        className="rich-text-resize-handle-bl"
        onMouseDown={(e) => handleMouseDown(e, 'bl')}
        aria-label="Resize text box bottom-left"
      />
      <div
        className="rich-text-resize-handle-br"
        onMouseDown={(e) => handleMouseDown(e, 'br')}
        aria-label="Resize text box bottom-right"
      />
    </div>
  );
}
