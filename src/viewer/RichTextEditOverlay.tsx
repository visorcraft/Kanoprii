import { useCallback, useEffect, useRef, useState } from 'react';
import type { ParagraphEditDraft, Rect, TextEditDraft, TextStyle } from '../app/usePdfEditState';
import { VIEWER_PAGE_H, VIEWER_PAGE_W } from '../app/constants';
import './RichTextEditOverlay.css';

type ResizeHandleKind = 'tl' | 'tr' | 'bl' | 'br';
type HandleKind = ResizeHandleKind | 'move';

type RichTextEditOverlayProps = {
  draft: TextEditDraft | ParagraphEditDraft;
  zoom?: number;
  /** Height of the edited page in PDF points (viewer vertical axis). When
   *  omitted the font is rendered 1:1 (legacy behaviour). */
  pageHeightPt?: number;
  onUpdate: (patch: { text?: string; style?: TextStyle; pageRect?: Rect }) => void;
  onApply: () => void;
  onCancel: () => void;
};

const MIN_W = 40;
const MIN_H = 20;

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function getFixedCorner(rect: Rect, kind: ResizeHandleKind): { x: number; y: number } {
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

function clampMove(rect: Rect): Rect {
  return {
    ...rect,
    x: clamp(rect.x, 0, VIEWER_PAGE_W - rect.w),
    y: clamp(rect.y, 0, VIEWER_PAGE_H - rect.h),
  };
}

function clampResize(rect: Rect, fixed: { x: number; y: number }, kind: ResizeHandleKind): Rect {
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
  pageHeightPt,
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
    // Keep focus on the textarea so typed text isn't lost when the user grabs the move handle.
    textareaRef.current?.focus();
    setDragging(kind);
    dragStartPosRef.current = { x: e.clientX, y: e.clientY };
    dragStartRectRef.current = rectRef.current;
  }, []);

  const handleMoveKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (!['ArrowRight', 'ArrowLeft', 'ArrowUp', 'ArrowDown'].includes(e.key)) return;
      e.preventDefault();
      e.stopPropagation();
      const current = rectRef.current;
      let { x, y } = current;
      switch (e.key) {
        case 'ArrowLeft':
          x -= 5;
          break;
        case 'ArrowRight':
          x += 5;
          break;
        case 'ArrowUp':
          y -= 5;
          break;
        case 'ArrowDown':
          y += 5;
          break;
      }
      const clamped = clampMove({ ...current, x, y });
      updateRect(clamped);
      onUpdate({ pageRect: clamped });
    },
    [onUpdate, updateRect],
  );

  const handleHandleKeyDown = useCallback(
    (e: React.KeyboardEvent, kind: ResizeHandleKind) => {
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

    let lastEvent: MouseEvent | null = null;

    const computeNext = (e: MouseEvent): Rect | null => {
      const dx = (e.clientX - startPos.x) / zoom;
      const dy = (e.clientY - startPos.y) / zoom;

      if (dragging === 'move') {
        return clampMove({
          x: startRect.x + dx,
          y: startRect.y + dy,
          w: startRect.w,
          h: startRect.h,
        });
      }

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
          return null;
      }
      const fixed = getFixedCorner(startRect, dragging);
      return clampResize(next, fixed, dragging);
    };

    const onMouseMove = (e: MouseEvent) => {
      lastEvent = e;
      const next = computeNext(e);
      if (next) updateRect(next);
    };

    const onMouseUp = () => {
      // Compute the final rect from the last known mouse position so the drop
      // lands exactly where the cursor is, even if no final mousemove fired.
      const finalRect =
        (lastEvent ? computeNext(lastEvent) : null) ?? rectRef.current;
      setDragging(null);
      onUpdate({ pageRect: finalRect });
      dragStartPosRef.current = null;
      dragStartRectRef.current = null;
      // Restore focus + caret to the textarea so typing continues where it left off.
      textareaRef.current?.focus();
    };

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp, { once: true });
    return () => {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };
  }, [dragging, onUpdate, updateRect, zoom]);

  const { style } = draft;

  // `style.fontSize` is in PDF points (written straight to the PDF `Tf`
  // operator), but this textarea lives in 800x1132 viewer space. Scale points
  // to viewer px so the preview matches the committed glyph size and the
  // baseline lands where `render_wrapped_text_box` will draw it.
  const fontPx =
    pageHeightPt && pageHeightPt > 0
      ? style.fontSize * (VIEWER_PAGE_H / pageHeightPt)
      : style.fontSize;

  const handleLabels: Record<ResizeHandleKind, string> = {
    tl: 'Resize text box top-left',
    tr: 'Resize text box top-right',
    bl: 'Resize text box bottom-left',
    br: 'Resize text box bottom-right',
  };

  const renderHandle = (kind: ResizeHandleKind) => (
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
      className={`rich-text-edit-overlay${dragging === 'move' ? ' moving' : ''}`}
      style={{
        position: 'absolute',
        left: rect.x,
        top: rect.y,
        width: rect.w,
        height: rect.h,
      }}
      onClick={(e) => e.stopPropagation()}
    >
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
              : style.fontFamily === 'Times'
                ? 'Times New Roman, Times, serif'
                : style.fontFamily,
          fontSize: `${fontPx}px`,
          lineHeight: `${fontPx}px`,
          // PDF baseline sits at box.y + fontPx. Push the textarea text down by
          // the descent (~20% of em) so its baseline lands at the same y as the
          // applied PDF text.
          paddingTop: `${Math.round(fontPx * 0.2)}px`,
          paddingBottom: 0,
          paddingLeft: 4,
          paddingRight: 4,
          fontWeight: style.bold ? 'bold' : 'normal',
          fontStyle: style.italic ? 'italic' : 'normal',
          textDecoration: style.underline ? 'underline' : 'none',
          color: `rgb(${style.color.r}, ${style.color.g}, ${style.color.b})`,
          textAlign: style.align,
        }}
      />
      <div
        className="rich-text-move-handle"
        role="button"
        tabIndex={0}
        aria-label="Move text box"
        onMouseDown={(e) => handleMouseDown(e, 'move')}
        onKeyDown={handleMoveKeyDown}
      />
      {renderHandle('tl')}
      {renderHandle('tr')}
      {renderHandle('bl')}
      {renderHandle('br')}
    </div>
  );
}
