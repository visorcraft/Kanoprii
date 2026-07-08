import { useCallback, useEffect, useRef, useState, type RefObject } from 'react';
import { PDF_BASE_HEIGHT, PDF_BASE_WIDTH } from '../pdf/usePdfDocument';

export type NaturalRect = { x: number; y: number; w: number; h: number };

function mergeLineRects(rects: NaturalRect[]): NaturalRect[] {
  if (rects.length === 0) return [];
  const sorted = [...rects].sort((a, b) => a.y - b.y || a.x - b.x);
  const lines: NaturalRect[][] = [];
  for (const rect of sorted) {
    const line = lines.find((group) => Math.abs(group[0].y - rect.y) < Math.max(group[0].h, rect.h) * 0.5);
    if (line) line.push(rect);
    else lines.push([rect]);
  }
  return lines.map((group) => {
    const x = Math.min(...group.map((r) => r.x));
    const y = Math.min(...group.map((r) => r.y));
    const right = Math.max(...group.map((r) => r.x + r.w));
    const bottom = Math.max(...group.map((r) => r.y + r.h));
    return { x, y, w: right - x, h: bottom - y };
  });
}

export function useTextSelection(pageContainerRef: RefObject<HTMLElement | null>, zoom: number) {
  const [hasSelection, setHasSelection] = useState(false);
  const capturedRectsRef = useRef<NaturalRect[]>([]);

  const computeSelectionRects = useCallback((): NaturalRect[] => {
    const container = pageContainerRef.current;
    const selection = window.getSelection();
    if (!container || !selection || selection.isCollapsed || selection.rangeCount === 0) return [];

    const containerBox = container.getBoundingClientRect();
    const scale = zoom > 0 ? zoom : 1;
    const rects: NaturalRect[] = [];

    for (let i = 0; i < selection.rangeCount; i++) {
      const range = selection.getRangeAt(i);
      for (const clientRect of range.getClientRects()) {
        if (clientRect.width <= 0 || clientRect.height <= 0) continue;
        const x = (clientRect.left - containerBox.left) / scale;
        const y = (clientRect.top - containerBox.top) / scale;
        const w = clientRect.width / scale;
        const h = clientRect.height / scale;
        if (x + w < 0 || y + h < 0 || x > PDF_BASE_WIDTH || y > PDF_BASE_HEIGHT) continue;
        rects.push({ x, y, w, h });
      }
    }

    return mergeLineRects(rects);
  }, [pageContainerRef, zoom]);

  // WebKit collapses the selection as soon as the user clicks the menu bar, so
  // capture rects on every selection change and keep the last non-empty capture
  // for the menu action to consume; a collapsed selection does not erase it.
  useEffect(() => {
    const onSelectionChange = () => {
      const text = window.getSelection()?.toString().trim() ?? '';
      if (text.length === 0) return;
      const rects = computeSelectionRects();
      if (rects.length > 0) {
        capturedRectsRef.current = rects;
        setHasSelection(true);
      }
    };
    document.addEventListener('selectionchange', onSelectionChange);
    return () => document.removeEventListener('selectionchange', onSelectionChange);
  }, [computeSelectionRects]);

  const readSelectionRects = useCallback((): NaturalRect[] => {
    const live = computeSelectionRects();
    return live.length > 0 ? live : capturedRectsRef.current;
  }, [computeSelectionRects]);

  const clearSelection = useCallback(() => {
    capturedRectsRef.current = [];
    setHasSelection(false);
    window.getSelection()?.removeAllRanges();
  }, []);

  return { hasSelection, readSelectionRects, clearSelection };
}
