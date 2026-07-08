import type { Dispatch, SetStateAction } from 'react';

export type ModeSetters = {
  cancelDrawing: () => void;
  setHighlightMode: Dispatch<SetStateAction<boolean>>;
  setNoteMode: Dispatch<SetStateAction<boolean>>;
  setDrawMode: Dispatch<SetStateAction<boolean>>;
  setShapeMode: Dispatch<SetStateAction<boolean>>;
  setStampMode: Dispatch<SetStateAction<boolean>>;
  setRedactMode: Dispatch<SetStateAction<boolean>>;
  setImageInsertMode: Dispatch<SetStateAction<boolean>>;
  setFormAddMode: Dispatch<SetStateAction<boolean>>;
  setTextEditMode: Dispatch<SetStateAction<boolean>>;
  setEditTextRunMode: Dispatch<SetStateAction<boolean>>;
  setVectorEditMode: Dispatch<SetStateAction<boolean>>;
  setShowNoteModal: (open: boolean) => void;
  setPendingNotePos: (pos: null) => void;
  setNoteDraft: (draft: string) => void;
};

export function clearOtherModes(
  modes: ModeSetters,
  except?: 'highlight' | 'note' | 'draw' | 'shape' | 'stamp' | 'redact' | 'image' | 'form' | 'text' | 'editText' | 'vector',
) {
  modes.cancelDrawing();
  if (except !== 'highlight') modes.setHighlightMode(false);
  if (except !== 'note') modes.setNoteMode(false);
  if (except !== 'draw') modes.setDrawMode(false);
  if (except !== 'shape') modes.setShapeMode(false);
  if (except !== 'stamp') modes.setStampMode(false);
  if (except !== 'redact') modes.setRedactMode(false);
  if (except !== 'image') modes.setImageInsertMode(false);
  if (except !== 'form') modes.setFormAddMode(false);
  if (except !== 'text') modes.setTextEditMode(false);
  if (except !== 'editText') modes.setEditTextRunMode(false);
  if (except !== 'vector') modes.setVectorEditMode(false);
  modes.setShowNoteModal(false);
  modes.setPendingNotePos(null);
}
