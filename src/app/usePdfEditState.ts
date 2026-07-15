import { useCallback, useState } from 'react';

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

export interface TextEditDraft {
  pageIndex: number;
  point: Point;
  text: string;
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
    (pageIndex: number, point: Point) => {
      setTextDraft({ pageIndex, point, text: '', style });
      setImageDraft(null);
      setMode('text');
    },
    [style],
  );

  const startEditingImage = useCallback((draft: ImageEditDraft) => {
    setImageDraft(draft);
    setTextDraft(null);
    setMode('image');
  }, []);

  const updateTextDraft = useCallback((patch: Partial<TextEditDraft>) => {
    setTextDraft((prev) => (prev ? { ...prev, ...patch } : null));
  }, []);

  const updateStyle = useCallback((patch: Partial<TextStyle>) => {
    setStyle((prev) => ({ ...prev, ...patch }));
  }, []);

  const clearDraft = useCallback(() => {
    setMode('idle');
    setTextDraft(null);
    setImageDraft(null);
    setStyle(DEFAULT_TEXT_STYLE);
  }, []);

  return {
    mode,
    textDraft,
    imageDraft,
    style,
    startEditingText,
    startInsertingText,
    startEditingImage,
    updateTextDraft,
    updateStyle,
    clearDraft,
  };
}

/** Canonical alias for this hook's state shape. */
export type PdfEditState = ReturnType<typeof usePdfEditState>;
