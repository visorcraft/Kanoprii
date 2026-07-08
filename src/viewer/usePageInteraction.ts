import type { FormFieldKind } from '../modals/AddFormFieldModal';
import type { ShapeKind, StampKind } from '../app/constants';
import type { createStructuralEditRunner } from '../pdf/runStructuralEdit';
import { usePageInteractionAnnot } from './usePageInteractionAnnot';
import { usePageInteractionHandlers } from './usePageInteractionHandlers';

type UsePageInteractionOptions = {
  filePath: string;
  currentPage: number;
  zoom: number;
  imgRef: React.RefObject<HTMLImageElement | null>;
  renderPage: (path: string, page: number) => Promise<void>;
  loadFormFields: (path: string) => Promise<void>;
  runEdit: ReturnType<typeof createStructuralEditRunner>;
  setAnnotations: (annots: import('../app/types').AnnotationData[]) => void;
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
};

export function usePageInteraction(opts: UsePageInteractionOptions) {
  const annot = usePageInteractionAnnot({
    filePath: opts.filePath,
    currentPage: opts.currentPage,
    runEdit: opts.runEdit,
    setAnnotations: opts.setAnnotations,
  });

  const handlers = usePageInteractionHandlers({
    ...opts,
    refreshAnnotations: annot.refreshAnnotations,
    commitInkStroke: annot.commitInkStroke,
  });

  return { ...annot, ...handlers };
}
