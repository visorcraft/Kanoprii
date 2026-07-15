import { useCallback, useMemo, useState } from 'react';

export type EditMode = 'idle' | 'text' | 'image';

export type TextAlignment = 'left' | 'center' | 'right';

/** Base font families. Bold/italic variants are selected at invoke time. */
export type FontFamily = 'Helvetica' | 'LiberationSans' | 'Courier';

export interface RgbColor {
  r: number;
  g: number;
  b: number;
}

export interface TextStyle {
  fontFamily: FontFamily;
  fontSize: number;
  color: RgbColor;
  align: TextAlignment;
  bold: boolean;
  italic: boolean;
  underline: boolean;
}

export interface Point {
  x: number;
  y: number;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface TextEditDraft {
  pageIndex: number;
  point: Point;
  text: string;
  /** Rectangle in natural page coordinates (800x1132 viewer space) where the overlay is placed. */
  pageRect: Rect;
  /** Source of truth for the active text edit. New edits start from the top-level style. */
  style: TextStyle;
}

export interface ImageEditDraft {
  pageIndex: number;
  point: Point;
  path: string;
  width?: number;
  height?: number;
}

export const DEFAULT_TEXT_STYLE: TextStyle = {
  fontFamily: 'Helvetica',
  fontSize: 12,
  color: { r: 0, g: 0, b: 0 },
  align: 'left',
  bold: false,
  italic: false,
  underline: false,
};

export function usePdfEditState() {
  const [mode, setMode] = useState<EditMode>('idle');
  const [textDraft, setTextDraft] = useState<TextEditDraft | null>(null);
  const [imageDraft, setImageDraft] = useState<ImageEditDraft | null>(null);
  /** Default style applied to newly created text edits. */
  const [style, setStyle] = useState<TextStyle>(DEFAULT_TEXT_STYLE);

  const startEditingText = useCallback((draft: TextEditDraft) => {
    setTextDraft(draft);
    setImageDraft(null);
    setMode('text');
  }, []);

  const startInsertingText = useCallback(
    (pageIndex: number, point: Point, pageRect: Rect) => {
      setTextDraft({ pageIndex, point, text: '', pageRect, style });
      setImageDraft(null);
      setMode('text');
    },
    [style]
  );

  const startEditingImage = useCallback((draft: ImageEditDraft) => {
    setImageDraft(draft);
    setTextDraft(null);
    setMode('image');
  }, []);

  const onUpdate = useCallback((patch: Partial<TextEditDraft>) => {
    setTextDraft((prev) => (prev ? { ...prev, ...patch } : null));
  }, []);

  const onCancel = useCallback(() => {
    setMode('idle');
    setTextDraft(null);
    setImageDraft(null);
    setStyle(DEFAULT_TEXT_STYLE);
  }, []);

  const onApply = useCallback(() => {
    // TODO: wire backend text insertion command (Task 9+).
    onCancel();
  }, [onCancel]);

  const updateStyle = useCallback((patch: Partial<TextStyle>) => {
    setStyle((prev) => ({ ...prev, ...patch }));
  }, []);

  return useMemo(
    () => ({
      mode,
      textDraft,
      imageDraft,
      style,
      startEditingText,
      startInsertingText,
      startEditingImage,
      updateStyle,
      onUpdate,
      onApply,
      onCancel,
    }),
    [
      mode,
      textDraft,
      imageDraft,
      style,
      startEditingText,
      startInsertingText,
      startEditingImage,
      updateStyle,
      onUpdate,
      onApply,
      onCancel,
    ]
  );
}

/** Canonical alias for this hook's state shape. */
export type PdfEditState = ReturnType<typeof usePdfEditState>;
