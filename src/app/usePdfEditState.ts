import { useCallback, useMemo, useRef, useState } from 'react';

export type EditMode = 'idle' | 'text' | 'image';

export type TextAlignment = 'left' | 'center' | 'right';

export type PdfEditCallbacks = {
  onApplyText?: () => void | Promise<void>;
  onApplyImage?: () => void | Promise<void>;
  onDeleteImage?: () => void | Promise<void>;
};

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
  /** When present, the edit updates an existing text line instead of adding a new box. */
  lineIndex?: number;
}

export interface ImageEditDraft {
  pageIndex: number;
  point: Point;
  path: string;
  /** Index of the image object within the page, used for transform/remove commands. */
  index: number;
  width?: number;
  height?: number;
  /** Rectangle in natural page coordinates (800x1132 viewer space) for the selection frame. */
  pageRect: Rect;
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
  /** Whether the PDF edit tool is selected. Active independently of any current draft. */
  const [editMode, setEditMode] = useState(false);

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

  const clearEditMode = useCallback(() => {
    setEditMode(false);
    onCancel();
  }, [onCancel]);

  const callbacksRef = useRef<PdfEditCallbacks>({});

  const bindEditCallbacks = useCallback((callbacks: PdfEditCallbacks) => {
    callbacksRef.current = callbacks;
  }, []);

  const onApply = useCallback(async () => {
    if (mode === 'text' && callbacksRef.current.onApplyText) {
      await callbacksRef.current.onApplyText();
    } else if (mode === 'image' && callbacksRef.current.onApplyImage) {
      await callbacksRef.current.onApplyImage();
    } else {
      onCancel();
    }
  }, [mode, onCancel]);

  const onUpdateImageRect = useCallback((rect: Rect) => {
    setImageDraft((prev) => (prev ? { ...prev, pageRect: rect } : null));
  }, []);

  const onDeleteImage = useCallback(async () => {
    if (callbacksRef.current.onDeleteImage) {
      await callbacksRef.current.onDeleteImage();
    } else {
      onCancel();
    }
  }, [onCancel]);

  const updateStyle = useCallback((patch: Partial<TextStyle>) => {
    setStyle((prev) => ({ ...prev, ...patch }));
  }, []);

  return useMemo(
    () => ({
      mode,
      editMode,
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
      setEditMode,
      clearEditMode,
      onUpdateImageRect,
      onDeleteImage,
      bindEditCallbacks,
    }),
    [
      mode,
      editMode,
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
      setEditMode,
      clearEditMode,
      onUpdateImageRect,
      onDeleteImage,
      bindEditCallbacks,
    ]
  );
}

/** Canonical alias for this hook's state shape. */
export type PdfEditState = ReturnType<typeof usePdfEditState>;
