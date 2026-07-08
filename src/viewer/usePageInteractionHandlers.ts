import { useCallback, useEffect, useRef } from 'react';
import type { FormFieldKind } from '../modals/AddFormFieldModal';
import type { ShapeKind, StampKind } from '../app/constants';
import type { createStructuralEditRunner } from '../pdf/runStructuralEdit';
import { getImageCoords as imageCoordsFromClick } from './getImageCoords';

type CoordFn = (clientX: number, clientY: number) => { x: number; y: number };

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
    (clientX, clientY) => imageCoordsFromClick(opts.imgRef, opts.zoom, clientX, clientY),
    [opts.imgRef, opts.zoom],
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

    const isRectMode = opts.highlightMode || opts.shapeMode || opts.redactMode || opts.imageInsertMode || opts.vectorEditMode || opts.formAddMode;
    if (!isRectMode) return;

    e.preventDefault();
    dragStateRef.current.armedByThisDown = false;

    if (dragStateRef.current.phase === 'idle') {
      dragStateRef.current.phase = 'armed';
      dragStateRef.current.armedByThisDown = true;
      const coords = getImageCoords(e.clientX, e.clientY);
      opts.setHighlightStart(coords);
      if (opts.shapeMode && opts.shapeKind === 'line') {
        opts.setShapeLineEnd(coords);
      } else {
        opts.setHighlightRect({ x: coords.x, y: coords.y, w: 0, h: 0 });
      }
      opts.setDrawing(true);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.drawMode, opts.highlightMode, opts.shapeMode, opts.redactMode, opts.imageInsertMode, opts.vectorEditMode, opts.formAddMode, opts.shapeKind, getImageCoords, opts.setHighlightStart, opts.setShapeLineEnd, opts.setHighlightRect, opts.setDrawing, opts.setInkDrawing, opts.setInkDraft]);

  const handleDrawMouseUp = useCallback((e: React.MouseEvent) => {
    if (opts.drawMode && opts.inkDrawing) {
      opts.setInkDrawing(false);
      const points = opts.inkDraft;
      opts.setInkDraft([]);
      opts.commitInkStroke(points);
      return;
    }

    if (!opts.drawing || !opts.highlightStart) return;

    const isRectMode = opts.highlightMode || opts.shapeMode || opts.redactMode || opts.imageInsertMode || opts.vectorEditMode || opts.formAddMode;
    if (!isRectMode) return;

    const coords = getImageCoords(e.clientX, e.clientY);
    const start = opts.highlightStart;

    if (dragStateRef.current.phase === 'dragging') {
      // Drag commit
      if (opts.shapeMode && opts.shapeKind === 'line') {
        const dist = Math.hypot(coords.x - start.x, coords.y - start.y);
        opts.setDrawing(false);
        opts.setHighlightStart(null);
        opts.setShapeLineEnd(null);
        dragStateRef.current.phase = 'idle';
        dragStateRef.current.armedByThisDown = false;
        if (dist >= 5) {
          void opts.runEdit({
            command: 'add_line',
            args: { pageIndex: opts.currentPage, x1: start.x, y1: start.y, x2: coords.x, y2: coords.y },
            afterEdit: async () => { await opts.refreshAnnotations(); },
            toast: 'Line added',
          });
        }
        return;
      }

      const rect = {
        x: Math.min(start.x, coords.x),
        y: Math.min(start.y, coords.y),
        w: Math.abs(coords.x - start.x),
        h: Math.abs(coords.y - start.y),
      };

      let minW = 5;
      let minH = 5;
      if (opts.formAddMode) { minW = 20; minH = 10; }
      else if (opts.vectorEditMode) { minW = 4; minH = 4; }

      opts.setDrawing(false);
      opts.setHighlightStart(null);
      opts.setHighlightRect(null);
      dragStateRef.current.phase = 'idle';
      dragStateRef.current.armedByThisDown = false;

      if (rect.w < minW || rect.h < minH) return;

      if (opts.redactMode) {
        void opts.runEdit({
          command: 'add_redaction',
          args: { pageIndex: opts.currentPage, x1: rect.x, y1: rect.y, x2: rect.x + rect.w, y2: rect.y + rect.h },
          afterEdit: async () => { await opts.refreshAnnotations(); },
          toast: 'Redaction added',
        });
      } else if (opts.imageInsertMode) {
        if (!opts.imageSourcePath) return;
        void opts.runEdit({
          command: 'add_page_image',
          args: {
            pageIndex: opts.currentPage,
            x: rect.x,
            y: rect.y,
            width: rect.w,
            height: rect.h,
            imagePath: opts.imageSourcePath,
          },
          afterEdit: async () => { await opts.renderPage(opts.filePath, opts.currentPage); },
          toast: 'Image inserted',
        });
      } else if (opts.vectorEditMode) {
        void opts.runEdit({
          command: 'add_page_vector_rect',
          args: { pageIndex: opts.currentPage, x: rect.x, y: rect.y, width: rect.w, height: rect.h },
          afterEdit: async () => { await opts.renderPage(opts.filePath, opts.currentPage); },
          toast: 'Vector shape added',
        });
      } else if (opts.formAddMode) {
        if (!opts.newFormFieldName.trim()) return;
        const base = { pageIndex: opts.currentPage, x: rect.x, y: rect.y, width: rect.w, height: rect.h };
        let command: string;
        let args: Record<string, unknown>;
        if (opts.newFormFieldKind === 'choice') {
          const options = opts.newFormFieldOptions.split(',').map((o) => o.trim()).filter(Boolean);
          command = 'add_choice_form_field';
          args = { ...base, name: opts.newFormFieldName.trim(), options, combo: true };
        } else {
          command = 'add_text_form_field';
          args = { ...base, name: opts.newFormFieldName.trim() };
        }
        void opts.runEdit({
          command,
          args,
          afterEdit: async () => {
            opts.setFormAddMode(false);
            opts.setShowAddFormFieldModal(false);
            opts.setNewFormFieldName('');
            await opts.loadFormFields(opts.filePath);
          },
          toast: 'Form field added',
        });
      } else if (opts.highlightMode) {
        void opts.runEdit({
          command: 'add_highlight',
          args: { pageIndex: opts.currentPage, x1: rect.x, y1: rect.y, x2: rect.x + rect.w, y2: rect.y + rect.h },
          afterEdit: async () => { await opts.refreshAnnotations(); },
          toast: 'Highlight added',
        });
      } else if (opts.shapeMode) {
        void opts.runEdit({
          command: opts.shapeKind === 'circle' ? 'add_circle' : 'add_square',
          args: { pageIndex: opts.currentPage, x1: rect.x, y1: rect.y, x2: rect.x + rect.w, y2: rect.y + rect.h },
          afterEdit: async () => { await opts.refreshAnnotations(); },
          toast: opts.shapeKind === 'circle' ? 'Ellipse added' : 'Rectangle added',
        });
      }
    } else if (dragStateRef.current.phase === 'armed') {
      // No significant drag - stay armed for click-click fallback
      // Do NOT disarm; the next click (handled in handlePageClick) will commit
    }
  }, [opts, getImageCoords]);

  const handlePageClick = useCallback((e: React.MouseEvent) => {
    if (opts.drawMode) return;

    if (opts.editTextRunMode && opts.handleEditTextRunClick) {
      const coords = getImageCoords(e.clientX, e.clientY);
      if (opts.handleEditTextRunClick(coords.x, coords.y)) return;
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
      if (dragStateRef.current.armedByThisDown) {
        dragStateRef.current.armedByThisDown = false;
        return;
      }
      const coords = getImageCoords(e.clientX, e.clientY);
      const start = opts.highlightStart;

      if (opts.shapeMode && opts.shapeKind === 'line') {
        const dist = Math.hypot(coords.x - start.x, coords.y - start.y);
        opts.cancelDrawing();
        if (dist < 5) return;
        void opts.runEdit({
          command: 'add_line',
          args: { pageIndex: opts.currentPage, x1: start.x, y1: start.y, x2: coords.x, y2: coords.y },
          afterEdit: async () => { await opts.refreshAnnotations(); },
          toast: 'Line added',
        });
        return;
      }

      const rect = {
        x: Math.min(start.x, coords.x),
        y: Math.min(start.y, coords.y),
        w: Math.abs(coords.x - start.x),
        h: Math.abs(coords.y - start.y),
      };

      let minW = 5;
      let minH = 5;
      if (opts.formAddMode) { minW = 20; minH = 10; }
      else if (opts.vectorEditMode) { minW = 4; minH = 4; }

      opts.cancelDrawing();
      if (rect.w < minW || rect.h < minH) return;

      if (opts.redactMode) {
        void opts.runEdit({
          command: 'add_redaction',
          args: { pageIndex: opts.currentPage, x1: rect.x, y1: rect.y, x2: rect.x + rect.w, y2: rect.y + rect.h },
          afterEdit: async () => { await opts.refreshAnnotations(); },
          toast: 'Redaction added',
        });
      } else if (opts.imageInsertMode) {
        if (!opts.imageSourcePath) return;
        void opts.runEdit({
          command: 'add_page_image',
          args: {
            pageIndex: opts.currentPage,
            x: rect.x,
            y: rect.y,
            width: rect.w,
            height: rect.h,
            imagePath: opts.imageSourcePath,
          },
          afterEdit: async () => { await opts.renderPage(opts.filePath, opts.currentPage); },
          toast: 'Image inserted',
        });
      } else if (opts.vectorEditMode) {
        void opts.runEdit({
          command: 'add_page_vector_rect',
          args: { pageIndex: opts.currentPage, x: rect.x, y: rect.y, width: rect.w, height: rect.h },
          afterEdit: async () => { await opts.renderPage(opts.filePath, opts.currentPage); },
          toast: 'Vector shape added',
        });
      } else if (opts.formAddMode) {
        if (!opts.newFormFieldName.trim()) return;
        const base = { pageIndex: opts.currentPage, x: rect.x, y: rect.y, width: rect.w, height: rect.h };
        let command: string;
        let args: Record<string, unknown>;
        if (opts.newFormFieldKind === 'choice') {
          const options = opts.newFormFieldOptions.split(',').map((o) => o.trim()).filter(Boolean);
          command = 'add_choice_form_field';
          args = { ...base, name: opts.newFormFieldName.trim(), options, combo: true };
        } else {
          command = 'add_text_form_field';
          args = { ...base, name: opts.newFormFieldName.trim() };
        }
        void opts.runEdit({
          command,
          args,
          afterEdit: async () => {
            opts.setFormAddMode(false);
            opts.setShowAddFormFieldModal(false);
            opts.setNewFormFieldName('');
            await opts.loadFormFields(opts.filePath);
          },
          toast: 'Form field added',
        });
      } else if (opts.highlightMode) {
        void opts.runEdit({
          command: 'add_highlight',
          args: { pageIndex: opts.currentPage, x1: rect.x, y1: rect.y, x2: rect.x + rect.w, y2: rect.y + rect.h },
          afterEdit: async () => { await opts.refreshAnnotations(); },
          toast: 'Highlight added',
        });
      } else if (opts.shapeMode) {
        void opts.runEdit({
          command: opts.shapeKind === 'circle' ? 'add_circle' : 'add_square',
          args: { pageIndex: opts.currentPage, x1: rect.x, y1: rect.y, x2: rect.x + rect.w, y2: rect.y + rect.h },
          afterEdit: async () => { await opts.refreshAnnotations(); },
          toast: opts.shapeKind === 'circle' ? 'Ellipse added' : 'Rectangle added',
        });
      }
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
  }, [opts, getImageCoords]);

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

    if (opts.shapeMode && opts.shapeKind === 'line') {
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
