import { useCallback, type Dispatch, type SetStateAction } from 'react';
import { clearOtherModes, type ModeSetters } from './annotationModeHelpers';

export type UseAnnotationModesMarkupOptions = ModeSetters & {
  setShowFormsPanel: Dispatch<SetStateAction<boolean>>;
};

export function useAnnotationModesMarkup(opts: UseAnnotationModesMarkupOptions) {
  const {
    cancelDrawing,
    setHighlightMode,
    setNoteMode,
    setDrawMode,
    setShapeMode,
    setStampMode,
    setRedactMode,
    setImageInsertMode,
    setFormAddMode,
    setTextEditMode,
    setEditTextRunMode,
    setVectorEditMode,
    setShowNoteModal,
    setPendingNotePos,
    setNoteDraft,
  } = opts;

  // Only stable setters: callbacks below intentionally omit `modes` from their
  // dependency arrays so they stay referentially stable across renders.
  const modes: ModeSetters = {
    cancelDrawing,
    setHighlightMode,
    setNoteMode,
    setDrawMode,
    setShapeMode,
    setStampMode,
    setRedactMode,
    setImageInsertMode,
    setFormAddMode,
    setTextEditMode,
    setEditTextRunMode,
    setVectorEditMode,
    setShowNoteModal,
    setPendingNotePos,
    setNoteDraft,
  };

  const toggleHighlightMode = useCallback(() => {
    clearOtherModes(modes, 'highlight');
    setHighlightMode((m) => !m);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [setHighlightMode]);

  const exitHighlightMode = useCallback(() => {
    cancelDrawing();
    setHighlightMode(false);
  }, [cancelDrawing, setHighlightMode]);

  const toggleNoteMode = useCallback(() => {
    clearOtherModes(modes, 'note');
    setNoteMode((m) => !m);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [setNoteMode]);

  const toggleDrawMode = useCallback(() => {
    clearOtherModes(modes, 'draw');
    setDrawMode((m) => !m);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [setDrawMode]);

  const exitDrawMode = useCallback(() => {
    cancelDrawing();
    setDrawMode(false);
  }, [cancelDrawing, setDrawMode]);

  const toggleShapeMode = useCallback(() => {
    clearOtherModes(modes, 'shape');
    setShapeMode((m) => !m);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [setShapeMode]);

  const exitShapeMode = useCallback(() => {
    cancelDrawing();
    setShapeMode(false);
  }, [cancelDrawing, setShapeMode]);

  const toggleStampMode = useCallback(() => {
    clearOtherModes(modes, 'stamp');
    setStampMode((m) => !m);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [setStampMode]);

  const exitStampMode = useCallback(() => {
    setStampMode(false);
  }, [setStampMode]);

  const toggleTextEditMode = useCallback(() => {
    clearOtherModes(modes, 'text');
    setTextEditMode((mode) => !mode);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [setTextEditMode]);

  const toggleEditTextRunMode = useCallback(() => {
    clearOtherModes(modes, 'editText');
    setEditTextRunMode((mode) => !mode);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [setEditTextRunMode]);

  const exitEditTextRunMode = useCallback(() => {
    setEditTextRunMode(false);
  }, [setEditTextRunMode]);

  const toggleVectorEditMode = useCallback(() => {
    clearOtherModes(modes, 'vector');
    setVectorEditMode((mode) => !mode);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [setVectorEditMode]);

  const toggleRedactMode = useCallback(() => {
    clearOtherModes(modes, 'redact');
    setRedactMode((m) => !m);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [setRedactMode]);

  const exitRedactMode = useCallback(() => {
    cancelDrawing();
    setRedactMode(false);
  }, [cancelDrawing, setRedactMode]);

  const exitNoteMode = useCallback(() => {
    setNoteMode(false);
    setShowNoteModal(false);
    setPendingNotePos(null);
    setNoteDraft('');
  }, [setNoteMode, setShowNoteModal, setPendingNotePos, setNoteDraft]);

  const toggleFormsPanel = useCallback(() => {
    opts.setShowFormsPanel((open) => !open);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.setShowFormsPanel]);

  return {
    toggleHighlightMode,
    exitHighlightMode,
    toggleNoteMode,
    toggleDrawMode,
    exitDrawMode,
    toggleShapeMode,
    exitShapeMode,
    toggleStampMode,
    exitStampMode,
    toggleTextEditMode,
    toggleEditTextRunMode,
    exitEditTextRunMode,
    toggleVectorEditMode,
    toggleRedactMode,
    exitRedactMode,
    exitNoteMode,
    toggleFormsPanel,
  };
}
