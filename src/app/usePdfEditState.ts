import { useCallback, useMemo, useRef, useState } from 'react';

export type EditMode = 'idle' | 'text' | 'image' | 'paragraph' | 'vector';

export type TextAlignment = 'left' | 'center' | 'right';

export type PdfEditCallbacks = {
  onApplyText?: () => void | Promise<void>;
  onApplyParagraph?: () => void | Promise<void>;
  onDeleteText?: () => void | Promise<void>;
  onDeleteParagraph?: () => void | Promise<void>;
  onApplyImage?: () => void | Promise<void>;
  onDeleteImage?: () => void | Promise<void>;
  onReplaceImage?: () => void | Promise<void>;
  onApplyVector?: () => void | Promise<void>;
  onDeleteVector?: () => void | Promise<void>;
};

/** Base font families. Bold/italic variants are selected at invoke time. */
export type FontFamily = 'Helvetica' | 'LiberationSans' | 'Times' | 'Courier';

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
  /** Original viewer rectangle to white out when PDFium found text the content decoder could not address. */
  sourceRect?: Rect;
  /** Original values used to skip unchanged edits. */
  original?: { text: string; style: TextStyle };
  /** Set only by explicit move/resize input, not automatic comfortable sizing. */
  geometryModified?: boolean;
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
  /** Rotation angle in degrees, counter-clockwise from upright. */
  rotation?: number;
  /** Original geometry used to close unchanged selections without rewriting the PDF. */
  original?: { pageRect: Rect; rotation: number };
}

export interface ParagraphEditDraft {
  pageIndex: number;
  /** Indices of decoded text lines that form this paragraph. */
  lineIndices: number[];
  text: string;
  /** Rectangle in natural page coordinates (800x1132 viewer space) for the paragraph edit box. */
  pageRect: Rect;
  style: TextStyle;
  /** Original values used to skip unchanged paragraph edits. */
  original?: { text: string; pageRect: Rect; style: TextStyle };
  /** Set only by explicit move/resize input, not automatic comfortable sizing. */
  geometryModified?: boolean;
}

export interface VectorEditDraft {
  pageIndex: number;
  index: number;
  pageRect: Rect;
  /** Original geometry used to skip unchanged Apply. */
  original?: Rect;
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
  const [paragraphDraft, setParagraphDraft] = useState<ParagraphEditDraft | null>(null);
  const [vectorDraft, setVectorDraft] = useState<VectorEditDraft | null>(null);
  /** Whether the paragraph draft is in text-edit mode (textarea) vs selection/move/resize mode. */
  const [paragraphEditing, setParagraphEditing] = useState(false);
  /** Default style applied to newly created text edits. */
  const [style, setStyle] = useState<TextStyle>(DEFAULT_TEXT_STYLE);
  /** Whether the PDF edit tool is selected. Active independently of any current draft. */
  const [editMode, setEditMode] = useState(false);

  const startEditingText = useCallback((draft: TextEditDraft) => {
    setTextDraft(draft);
    setImageDraft(null);
    setParagraphDraft(null);
    setVectorDraft(null);
    setMode('text');
  }, []);

  const startInsertingText = useCallback(
    (pageIndex: number, point: Point, pageRect: Rect) => {
      setTextDraft({ pageIndex, point, text: '', pageRect, style });
      setImageDraft(null);
      setParagraphDraft(null);
      setVectorDraft(null);
      setMode('text');
    },
    [style]
  );

  const beginTextInsert = useCallback(() => {
    setTextDraft(null);
    setImageDraft(null);
    setParagraphDraft(null);
    setVectorDraft(null);
    setParagraphEditing(false);
    setMode('text');
  }, []);

  const startEditingImage = useCallback((draft: ImageEditDraft) => {
    setImageDraft({
      ...draft,
      original: draft.original ?? { pageRect: { ...draft.pageRect }, rotation: draft.rotation ?? 0 },
    });
    setTextDraft(null);
    setParagraphDraft(null);
    setVectorDraft(null);
    setMode('image');
  }, []);

  const startEditingParagraph = useCallback((draft: ParagraphEditDraft) => {
    setParagraphDraft({
      ...draft,
      original: draft.original ?? { text: draft.text, pageRect: { ...draft.pageRect }, style: draft.style },
    });
    setParagraphEditing(false);
    setTextDraft(null);
    setImageDraft(null);
    setVectorDraft(null);
    setMode('paragraph');
  }, []);

  const startEditingVector = useCallback((draft: VectorEditDraft) => {
    setVectorDraft({ ...draft, original: draft.original ?? { ...draft.pageRect } });
    setTextDraft(null);
    setImageDraft(null);
    setParagraphDraft(null);
    setMode('vector');
  }, []);

  const enterParagraphTextEdit = useCallback(() => {
    setParagraphEditing(true);
  }, []);

  const onUpdate = useCallback((patch: Partial<TextEditDraft>) => {
    setTextDraft((prev) => (prev ? { ...prev, ...patch } : null));
  }, []);

  const onUpdateParagraph = useCallback((patch: Partial<ParagraphEditDraft>) => {
    setParagraphDraft((prev) => (prev ? { ...prev, ...patch } : null));
  }, []);

  const onCancel = useCallback(() => {
    setMode('idle');
    setTextDraft(null);
    setImageDraft(null);
    setParagraphDraft(null);
    setVectorDraft(null);
    setParagraphEditing(false);
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
    } else if (mode === 'paragraph' && callbacksRef.current.onApplyParagraph) {
      await callbacksRef.current.onApplyParagraph();
    } else if (mode === 'image' && callbacksRef.current.onApplyImage) {
      await callbacksRef.current.onApplyImage();
    } else if (mode === 'vector' && callbacksRef.current.onApplyVector) {
      await callbacksRef.current.onApplyVector();
    } else {
      onCancel();
    }
  }, [mode, onCancel]);

  const onUpdateImageRect = useCallback((rect: Rect) => {
    setImageDraft((prev) => (prev ? { ...prev, pageRect: rect } : null));
  }, []);

  const onUpdateImageRotation = useCallback((rotation: number) => {
    setImageDraft((prev) => (prev ? { ...prev, rotation } : null));
  }, []);

  const onUpdateVectorRect = useCallback((pageRect: Rect) => {
    setVectorDraft((prev) => (prev ? { ...prev, pageRect } : null));
  }, []);

  const onDeleteParagraph = useCallback(async () => {
    if (callbacksRef.current.onDeleteParagraph) {
      await callbacksRef.current.onDeleteParagraph();
    } else {
      onCancel();
    }
  }, [onCancel]);

  const onDeleteText = useCallback(async () => {
    if (callbacksRef.current.onDeleteText) {
      await callbacksRef.current.onDeleteText();
    } else {
      onCancel();
    }
  }, [onCancel]);

  const onDeleteImage = useCallback(async () => {
    if (callbacksRef.current.onDeleteImage) {
      await callbacksRef.current.onDeleteImage();
    } else {
      onCancel();
    }
  }, [onCancel]);

  const onReplaceImage = useCallback(async () => {
    await callbacksRef.current.onReplaceImage?.();
  }, []);

  const onDeleteVector = useCallback(async () => {
    if (callbacksRef.current.onDeleteVector) await callbacksRef.current.onDeleteVector();
    else onCancel();
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
      paragraphDraft,
      vectorDraft,
      paragraphEditing,
      style,
      startEditingText,
      startInsertingText,
      beginTextInsert,
      startEditingParagraph,
      enterParagraphTextEdit,
      startEditingImage,
      startEditingVector,
      updateStyle,
      onUpdate,
      onUpdateParagraph,
      onApply,
      onCancel,
      setEditMode,
      clearEditMode,
      onUpdateImageRect,
      onUpdateImageRotation,
      onUpdateVectorRect,
      onDeleteText,
      onDeleteParagraph,
      onDeleteImage,
      onReplaceImage,
      onDeleteVector,
      bindEditCallbacks,
    }),
    [
      mode,
      editMode,
      textDraft,
      imageDraft,
      paragraphDraft,
      vectorDraft,
      paragraphEditing,
      style,
      startEditingText,
      startInsertingText,
      beginTextInsert,
      startEditingParagraph,
      enterParagraphTextEdit,
      startEditingImage,
      startEditingVector,
      updateStyle,
      onUpdate,
      onUpdateParagraph,
      onApply,
      onCancel,
      setEditMode,
      clearEditMode,
      onUpdateImageRect,
      onUpdateImageRotation,
      onUpdateVectorRect,
      onDeleteText,
      onDeleteParagraph,
      onDeleteImage,
      onReplaceImage,
      onDeleteVector,
      bindEditCallbacks,
    ]
  );
}

/** Canonical alias for this hook's state shape. */
export type PdfEditState = ReturnType<typeof usePdfEditState>;
