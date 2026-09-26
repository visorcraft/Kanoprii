import { useCallback, useMemo, useRef, useState } from 'react';
import type { ShapeKind } from './constants';

export type EditMode = 'idle' | 'text' | 'image' | 'paragraph' | 'vector' | 'shape';

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
  onApplyShape?: () => void | Promise<void>;
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
  /** Rectangle in natural page coordinates (viewer pixel space) where the overlay is placed. */
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
  /** Rectangle in natural page coordinates (viewer pixel space) for the selection frame. */
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
  /** Rectangle in natural page coordinates (viewer pixel space) for the paragraph edit box. */
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

export interface ShapeLineGeometry {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

type BoxShapeDraft = {
  pageIndex: number;
  kind: Extract<ShapeKind, 'square' | 'circle'>;
  geometry: { type: 'box'; rect: Rect };
  style: ShapeStyle;
};

type LineShapeDraft = {
  pageIndex: number;
  kind: Extract<ShapeKind, 'line' | 'arrow'>;
  geometry: { type: 'line'; line: ShapeLineGeometry };
  style: ShapeStyle;
};

export type ShapeEditDraft = BoxShapeDraft | LineShapeDraft;
export type NewShapeDraft = Omit<BoxShapeDraft, 'style'> | Omit<LineShapeDraft, 'style'>;

function isBoxShapeDraft(draft: ShapeEditDraft): draft is BoxShapeDraft {
  return draft.kind === 'square' || draft.kind === 'circle';
}

function isLineShapeDraft(draft: ShapeEditDraft): draft is LineShapeDraft {
  return draft.kind === 'line' || draft.kind === 'arrow';
}

export interface ShapeStyle {
  strokeColor: RgbColor;
  strokeWidth: number;
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

export const DEFAULT_SHAPE_STYLE: ShapeStyle = {
  strokeColor: { r: 255, g: 0, b: 0 },
  strokeWidth: 2,
};

export function usePdfEditState() {
  const [mode, setMode] = useState<EditMode>('idle');
  const [textDraft, setTextDraft] = useState<TextEditDraft | null>(null);
  const [imageDraft, setImageDraft] = useState<ImageEditDraft | null>(null);
  const [paragraphDraft, setParagraphDraft] = useState<ParagraphEditDraft | null>(null);
  const [vectorDraft, setVectorDraft] = useState<VectorEditDraft | null>(null);
  const [shapeDraft, setShapeDraft] = useState<ShapeEditDraft | null>(null);
  /** Whether the paragraph draft is in text-edit mode (textarea) vs selection/move/resize mode. */
  const [paragraphEditing, setParagraphEditing] = useState(false);
  /** Default style applied to newly created text edits. */
  const [style, setStyle] = useState<TextStyle>(DEFAULT_TEXT_STYLE);
  /** Default appearance applied to newly created shapes. */
  const [shapeStyle, setShapeStyle] = useState<ShapeStyle>(DEFAULT_SHAPE_STYLE);
  /** Whether the PDF edit tool is selected. Active independently of any current draft. */
  const [editMode, setEditMode] = useState(false);

  const startEditingText = useCallback((draft: TextEditDraft) => {
    setTextDraft(draft);
    setImageDraft(null);
    setParagraphDraft(null);
    setVectorDraft(null);
    setShapeDraft(null);
    setMode('text');
  }, []);

  const startInsertingText = useCallback(
    (pageIndex: number, point: Point, pageRect: Rect) => {
      setTextDraft({ pageIndex, point, text: '', pageRect, style });
      setImageDraft(null);
      setParagraphDraft(null);
      setVectorDraft(null);
      setShapeDraft(null);
      setMode('text');
    },
    [style]
  );

  const beginTextInsert = useCallback(() => {
    setTextDraft(null);
    setImageDraft(null);
    setParagraphDraft(null);
    setVectorDraft(null);
    setShapeDraft(null);
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
    setShapeDraft(null);
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
    setShapeDraft(null);
    setMode('paragraph');
  }, []);

  const startEditingVector = useCallback((draft: VectorEditDraft) => {
    setVectorDraft({ ...draft, original: draft.original ?? { ...draft.pageRect } });
    setTextDraft(null);
    setImageDraft(null);
    setParagraphDraft(null);
    setShapeDraft(null);
    setMode('vector');
  }, []);

  const startDrawingShape = useCallback((draft: NewShapeDraft) => {
    if (draft.geometry.type === 'box') setShapeDraft({ ...draft, style: shapeStyle });
    else setShapeDraft({ ...draft, style: shapeStyle });
    setTextDraft(null);
    setImageDraft(null);
    setParagraphDraft(null);
    setVectorDraft(null);
    setParagraphEditing(false);
    setMode('shape');
  }, [shapeStyle]);

  const enterParagraphTextEdit = useCallback(() => {
    setParagraphEditing(true);
  }, []);

  const onUpdate = useCallback((patch: Partial<TextEditDraft>) => {
    setTextDraft((prev) => (prev ? { ...prev, ...patch } : null));
  }, []);

  const onUpdateParagraph = useCallback((patch: Partial<ParagraphEditDraft>) => {
    setParagraphDraft((prev) => (prev ? { ...prev, ...patch } : null));
  }, []);

  const onUpdateShapeBox = useCallback((rect: Rect) => {
    setShapeDraft((prev) => {
      if (!prev || !isBoxShapeDraft(prev)) return prev;
      return { pageIndex: prev.pageIndex, kind: prev.kind, style: prev.style, geometry: { type: 'box', rect } };
    });
  }, []);

  const onUpdateShapeLine = useCallback((line: ShapeLineGeometry) => {
    setShapeDraft((prev) => {
      if (!prev || !isLineShapeDraft(prev)) return prev;
      return { pageIndex: prev.pageIndex, kind: prev.kind, style: prev.style, geometry: { type: 'line', line } };
    });
  }, []);

  const updateShapeStyle = useCallback((patch: Partial<ShapeStyle>) => {
    setShapeStyle((prev) => ({ ...prev, ...patch }));
    setShapeDraft((prev) => (prev ? { ...prev, style: { ...prev.style, ...patch } } : null));
  }, []);

  const onCancel = useCallback(() => {
    setMode('idle');
    setTextDraft(null);
    setImageDraft(null);
    setParagraphDraft(null);
    setVectorDraft(null);
    setShapeDraft(null);
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
    } else if (mode === 'shape' && callbacksRef.current.onApplyShape) {
      await callbacksRef.current.onApplyShape();
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
      shapeDraft,
      paragraphEditing,
      style,
      shapeStyle,
      startEditingText,
      startInsertingText,
      beginTextInsert,
      startEditingParagraph,
      enterParagraphTextEdit,
      startEditingImage,
      startEditingVector,
      startDrawingShape,
      updateStyle,
      updateShapeStyle,
      onUpdate,
      onUpdateParagraph,
      onUpdateShapeBox,
      onUpdateShapeLine,
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
      onDeleteShape: onCancel,
      bindEditCallbacks,
    }),
    [
      mode,
      editMode,
      textDraft,
      imageDraft,
      paragraphDraft,
      vectorDraft,
      shapeDraft,
      paragraphEditing,
      style,
      shapeStyle,
      startEditingText,
      startInsertingText,
      beginTextInsert,
      startEditingParagraph,
      enterParagraphTextEdit,
      startEditingImage,
      startEditingVector,
      startDrawingShape,
      updateStyle,
      updateShapeStyle,
      onUpdate,
      onUpdateParagraph,
      onUpdateShapeBox,
      onUpdateShapeLine,
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
