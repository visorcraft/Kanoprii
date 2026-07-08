import { useCallback, useState } from 'react';

export function useDrawingGesture() {
  const [highlightStart, setHighlightStart] = useState<{ x: number; y: number } | null>(null);
  const [highlightRect, setHighlightRect] = useState<{ x: number; y: number; w: number; h: number } | null>(null);
  const [inkDrawing, setInkDrawing] = useState(false);
  const [inkDraft, setInkDraft] = useState<number[]>([]);
  const [shapeLineEnd, setShapeLineEnd] = useState<{ x: number; y: number } | null>(null);
  const [drawing, setDrawing] = useState(false);

  const cancelDrawing = useCallback(() => {
    setDrawing(false);
    setHighlightStart(null);
    setHighlightRect(null);
    setInkDrawing(false);
    setInkDraft([]);
    setShapeLineEnd(null);
  }, []);

  return {
    highlightStart,
    setHighlightStart,
    highlightRect,
    setHighlightRect,
    inkDrawing,
    setInkDrawing,
    inkDraft,
    setInkDraft,
    shapeLineEnd,
    setShapeLineEnd,
    drawing,
    setDrawing,
    cancelDrawing,
  };
}
