import { useCallback, useEffect, useRef } from 'react';
import type { FormFieldKind } from '../modals/AddFormFieldModal';
import type { ShapeKind, StampKind } from '../app/constants';
import type { DocumentSessionData } from '../app/documentSessionTypes';
import type { PdfEditState } from '../app/usePdfEditState';
import type { PageVectorEdit } from '../app/types';
import type { createStructuralEditRunner } from '../pdf/runStructuralEdit';
import { getImageCoords as imageCoordsFromClick } from './getImageCoords';

type CoordFn = (clientX: number, clientY: number) => { x: number; y: number };
type DrawPoint = ReturnType<CoordFn>;
type DrawRect = { x: number; y: number; w: number; h: number };

function isLineShape(kind: ShapeKind): kind is Extract<ShapeKind, 'line' | 'arrow'> {
  return kind === 'line' || kind === 'arrow';
}

function rectBetween(start: DrawPoint, end: DrawPoint): DrawRect {
  return {
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    w: Math.abs(end.x - start.x),
    h: Math.abs(end.y - start.y),
  };
}

export type PageInteractionHandlerOptions = {
  filePath: string;
  currentPage: number;
  zoom: number;
  imgRef: React.RefObject<HTMLImageElement | null>;
  renderPage: (path: string, page: number) => Promise<void>;
  loadFormFields: (path: string) => Promise<void>;
  runEdit: ReturnType<typeof createStructuralEditRunner>;
  drawMode: boolean;
  textEditMode: boolean;
  editTextRunMode?: boolean;
  handleEditTextRunClick?: (x: number, y: number) => boolean;
  vectorEditMode: boolean;
  pageVectorEdits: PageVectorEdit[];
  formAddMode: boolean;
  imageInsertMode: boolean;
  redactMode: boolean;
  stampMode: boolean;
  shapeMode: boolean;
  noteMode: boolean;
  highlightMode: boolean;
  drawing: boolean;
  highlightStart: { x: number; y: number } | null;
  inkDrawing: boolean;
  inkDraft: number[];
  shapeKind: ShapeKind;
  stampKind: StampKind;
  stampPreset: string;
  imageSourcePath: string;
  newFormFieldKind: FormFieldKind;
  newFormFieldName: string;
  newFormFieldOptions: string;
  newFormRadioGroup: string;
  newFormRadioOption: string;
  newFormCheckboxChecked: boolean;
  pdfEdit: PdfEditState;
  session: DocumentSessionData | null;
  handleEditPageClick?: (
    pageIndex: number,
    point: { x: number; y: number },
    session: DocumentSessionData,
    hitTestImage?: (pageIndex: number, x: number, y: number) => Promise<{
      index: number;
      viewerRect: { x: number; y: number; w: number; h: number };
      width?: number;
      height?: number;
    } | null>,
    textOnly?: boolean,
  ) => void | Promise<void>;
  hitTestImage?: (pageIndex: number, x: number, y: number) => Promise<{
    index: number;
    viewerRect: { x: number; y: number; w: number; h: number };
    width?: number;
    height?: number;
  } | null>;
  cancelDrawing: () => void;
  setHighlightStart: (pos: { x: number; y: number } | null) => void;
  setHighlightRect: (rect: { x: number; y: number; w: number; h: number } | null) => void;
  setDrawing: (drawing: boolean) => void;
  setShapeLineEnd: (pos: { x: number; y: number } | null) => void;
  setInkDrawing: (drawing: boolean) => void;
  setInkDraft: React.Dispatch<React.SetStateAction<number[]>>;
  setPendingTextPos: (pos: { x: number; y: number } | null) => void;
  setPageTextDraft: (text: string) => void;
  setEditingTextIndex: (index: number | null) => void;
  setShowPageTextModal: (open: boolean) => void;
  setPendingNotePos: (pos: { x: number; y: number } | null) => void;
  setNoteDraft: (text: string) => void;
  setShowNoteModal: (open: boolean) => void;
  setFormAddMode: (mode: boolean) => void;
  setShowAddFormFieldModal: (open: boolean) => void;
  setNewFormFieldName: (name: string) => void;
  setNewFormRadioGroup: (group: string) => void;
  setNewFormRadioOption: (option: string) => void;
  refreshAnnotations: () => Promise<void>;
  commitInkStroke: (points: number[]) => void;
};

export function usePageInteractionHandlers(opts: PageInteractionHandlerOptions) {
  const getImageCoords: CoordFn = useCallback(
    (clientX, clientY) => imageCoordsFromClick(opts.imgRef, clientX, clientY),
    [opts.imgRef],
  );

  const dragStateRef = useRef<{ phase: 'idle' | 'armed' | 'dragging'; armedByThisDown: boolean }>({
    phase: 'idle',
    armedByThisDown: false,
  });

  useEffect(() => {
    if (!opts.drawing) {
      dragStateRef.current.phase = 'idle';
      dragStateRef.current.armedByThisDown = false;
    }
  }, [opts.drawing]);

  const handleDrawMouseDown = useCallback((e: React.MouseEvent) => {
    if (opts.drawMode) {
      e.preventDefault();
      const coords = getImageCoords(e.clientX, e.clientY);
      opts.setInkDrawing(true);
      opts.setInkDraft([coords.x, coords.y]);
      return;
    }

    if (opts.vectorEditMode) {
      e.preventDefault();
      if (opts.pdfEdit.vectorDraft || opts.pdfEdit.shapeDraft) {
        opts.pdfEdit.onCancel();
        return;
      }
      const point = getImageCoords(e.clientX, e.clientY);
      const hit = [...opts.pageVectorEdits].reverse().find((vector) =>
        point.x >= vector.x && point.x <= vector.x + vector.width &&
        point.y >= vector.y && point.y <= vector.y + vector.height
      );
      if (hit) {
        opts.pdfEdit.startEditingVector({
          pageIndex: opts.currentPage,
          index: hit.index,
          pageRect: { x: hit.x, y: hit.y, w: hit.width, h: hit.height },
        });
        return;
      }
    }

    const isRectMode = opts.highlightMode || opts.shapeMode || opts.redactMode || opts.imageInsertMode || opts.vectorEditMode || opts.formAddMode;
    if (!isRectMode) return;

    e.preventDefault();
    dragStateRef.current.armedByThisDown = false;

    if (dragStateRef.current.phase === 'idle') {
      dragStateRef.current.phase = 'armed';
      dragStateRef.current.armedByThisDown = true;
      const coords = getImageCoords(e.clientX, e.clientY);
      opts.setHighlightStart(coords);
      if ((opts.shapeMode || opts.vectorEditMode) && isLineShape(opts.shapeKind)) {
        opts.setShapeLineEnd(coords);
      } else {
        opts.setHighlightRect({ x: coords.x, y: coords.y, w: 0, h: 0 });
      }
      opts.setDrawing(true);
    }
  }, [opts, getImageCoords]);

  const commitRectDrawing = useCallback((rect: DrawRect) => {
    const minSize = opts.formAddMode ? { w: 20, h: 10 } : opts.vectorEditMode ? { w: 4, h: 4 } : { w: 5, h: 5 };
    if (rect.w < minSize.w || rect.h < minSize.h) return;
    const endpoints = { pageIndex: opts.currentPage, x1: rect.x, y1: rect.y, x2: rect.x + rect.w, y2: rect.y + rect.h };

    if (opts.redactMode) {
      void opts.runEdit({
        command: 'add_redaction',
        args: endpoints,
        afterEdit: async () => { await opts.refreshAnnotations(); },
        toast: 'Redaction added',
      });
      return;
    }
    if (opts.imageInsertMode) {
      if (!opts.imageSourcePath) return;
      void opts.runEdit({
        command: 'add_page_image',
        args: { pageIndex: opts.currentPage, x: rect.x, y: rect.y, width: rect.w, height: rect.h, imagePath: opts.imageSourcePath },
        afterEdit: async () => { await opts.renderPage(opts.filePath, opts.currentPage); },
        toast: 'Image inserted',
      });
      return;
    }
    if (opts.vectorEditMode) {
      if (isLineShape(opts.shapeKind)) return;
      opts.pdfEdit.startDrawingShape({
        pageIndex: opts.currentPage,
        kind: opts.shapeKind,
        geometry: { type: 'box', rect },
      });
      return;
    }
    if (opts.formAddMode) {
      const name = opts.newFormFieldName.trim();
      if (!name) return;
      const base = { pageIndex: opts.currentPage, x: rect.x, y: rect.y, width: rect.w, height: rect.h, name };
      const isChoice = opts.newFormFieldKind === 'choice';
      const args = isChoice
        ? { ...base, options: opts.newFormFieldOptions.split(',').map((option) => option.trim()).filter(Boolean), combo: true }
        : base;
      void opts.runEdit({
        command: isChoice ? 'add_choice_form_field' : 'add_text_form_field',
        args,
        afterEdit: async () => {
          opts.setFormAddMode(false);
          opts.setShowAddFormFieldModal(false);
          opts.setNewFormFieldName('');
          await opts.loadFormFields(opts.filePath);
        },
        toast: 'Form field added',
      });
      return;
    }
    if (opts.highlightMode) {
      void opts.runEdit({
        command: 'add_highlight',
        args: endpoints,
        afterEdit: async () => { await opts.refreshAnnotations(); },
        toast: 'Highlight added',
      });
      return;
    }
    if (opts.shapeMode && !isLineShape(opts.shapeKind)) {
      void opts.runEdit({
        command: opts.shapeKind === 'circle' ? 'add_circle' : 'add_square',
        args: endpoints,
        afterEdit: async () => { await opts.refreshAnnotations(); },
        toast: opts.shapeKind === 'circle' ? 'Ellipse added' : 'Rectangle added',
      });
    }
  }, [opts]);

  const finishDrawing = useCallback((end: DrawPoint) => {
    const start = opts.highlightStart;
    if (!start) return;
    opts.cancelDrawing();
    dragStateRef.current.phase = 'idle';
    dragStateRef.current.armedByThisDown = false;

    if ((opts.shapeMode || opts.vectorEditMode) && isLineShape(opts.shapeKind)) {
      if (Math.hypot(end.x - start.x, end.y - start.y) < 5) return;
      if (opts.vectorEditMode) {
        opts.pdfEdit.startDrawingShape({
          pageIndex: opts.currentPage,
          kind: opts.shapeKind,
          geometry: { type: 'line', line: { x1: start.x, y1: start.y, x2: end.x, y2: end.y } },
        });
        return;
      }
      void opts.runEdit({
        command: opts.shapeKind === 'arrow' ? 'add_arrow' : 'add_line',
        args: { pageIndex: opts.currentPage, x1: start.x, y1: start.y, x2: end.x, y2: end.y },
        afterEdit: async () => { await opts.refreshAnnotations(); },
        toast: opts.shapeKind === 'arrow' ? 'Arrow added' : 'Line added',
      });
      return;
    }
    commitRectDrawing(rectBetween(start, end));
  }, [commitRectDrawing, opts]);

  const handleDrawMouseUp = useCallback((e: React.MouseEvent) => {
    if (opts.drawMode && opts.inkDrawing) {
      opts.setInkDrawing(false);
      const points = opts.inkDraft;
      opts.setInkDraft([]);
      opts.commitInkStroke(points);
      return;
    }
    if (!opts.drawing || !opts.highlightStart || dragStateRef.current.phase !== 'dragging') return;
    finishDrawing(getImageCoords(e.clientX, e.clientY));
  }, [finishDrawing, getImageCoords, opts]);

  const handlePageClick = useCallback((e: React.MouseEvent) => {
    if (opts.drawMode) return;

    if (opts.pdfEdit.editMode || opts.editTextRunMode) {
      if (!opts.session) return;
      e.preventDefault();
      const coords = getImageCoords(e.clientX, e.clientY);
      void opts.handleEditPageClick?.(
        opts.currentPage,
        coords,
        opts.session,
        opts.editTextRunMode ? undefined : opts.hitTestImage,
        opts.editTextRunMode,
      );
      return;
    }
    if (opts.textEditMode) {
      const coords = getImageCoords(e.clientX, e.clientY);
      opts.setPendingTextPos(coords);
      opts.setPageTextDraft('');
      opts.setEditingTextIndex(null);
      opts.setShowPageTextModal(true);
      return;
    }

    // Rect modes: click-click fallback (second click commits)
    const isRectMode = opts.highlightMode || opts.shapeMode || opts.redactMode || opts.imageInsertMode || opts.vectorEditMode || opts.formAddMode;
    if (isRectMode) {
      if (!opts.drawing || !opts.highlightStart) return;
      if (dragStateRef.current.phase !== 'armed') return;
      if (dragStateRef.current.armedByThisDown) {
        dragStateRef.current.armedByThisDown = false;
        return;
      }
      finishDrawing(getImageCoords(e.clientX, e.clientY));
      return;
    }

    if (opts.stampMode) {
      const coords = getImageCoords(e.clientX, e.clientY);
      void opts.runEdit({
        command: opts.stampKind === 'image' ? 'add_image_stamp' : 'add_text_stamp',
        args: { pageIndex: opts.currentPage, x: coords.x, y: coords.y, preset: opts.stampPreset },
        afterEdit: async () => { await opts.refreshAnnotations(); },
        toast: 'Stamp added',
      });
      return;
    }
    if (opts.noteMode) {
      const coords = getImageCoords(e.clientX, e.clientY);
      opts.setPendingNotePos(coords);
      opts.setNoteDraft('');
      opts.setShowNoteModal(true);
      return;
    }
  }, [finishDrawing, opts, getImageCoords]);

  const handlePageMouseMove = useCallback((e: React.MouseEvent) => {
    if (opts.drawMode && opts.inkDrawing) {
      const coords = getImageCoords(e.clientX, e.clientY);
      opts.setInkDraft((prev) => {
        if (prev.length < 2) return [...prev, coords.x, coords.y];
        const lx = prev[prev.length - 2];
        const ly = prev[prev.length - 1];
        if (Math.hypot(coords.x - lx, coords.y - ly) < 2) return prev;
        return [...prev, coords.x, coords.y];
      });
      return;
    }

    const isRectDrawing = opts.drawing && opts.highlightStart && (
      opts.shapeMode || opts.redactMode || opts.imageInsertMode || opts.vectorEditMode || opts.formAddMode || opts.highlightMode
    );
    if (!isRectDrawing) return;

    const coords = getImageCoords(e.clientX, e.clientY);

    if (dragStateRef.current.phase === 'armed' && opts.highlightStart) {
      const dx = coords.x - opts.highlightStart.x;
      const dy = coords.y - opts.highlightStart.y;
      if (Math.hypot(dx, dy) > 2) {
        dragStateRef.current.phase = 'dragging';
      }
    }

    if ((opts.shapeMode || opts.vectorEditMode) && isLineShape(opts.shapeKind)) {
      opts.setShapeLineEnd(coords);
      return;
    }

    if (!opts.highlightStart) return;
    opts.setHighlightRect({
      x: Math.min(opts.highlightStart.x, coords.x),
      y: Math.min(opts.highlightStart.y, coords.y),
      w: Math.abs(coords.x - opts.highlightStart.x),
      h: Math.abs(coords.y - opts.highlightStart.y),
    });
  }, [opts, getImageCoords]);

  return {
    handlePageClick,
    handlePageMouseMove,
    handleDrawMouseDown,
    handleDrawMouseUp,
  };
}
