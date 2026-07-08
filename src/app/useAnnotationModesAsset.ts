import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { FormFieldKind } from '../modals/AddFormFieldModal';
import { clearOtherModes, type ModeSetters } from './annotationModeHelpers';

export type UseAnnotationModesAssetOptions = ModeSetters & {
  filePath: string;
  imageSourcePath: string;
  imageSourceDraft: string;
  newFormFieldKind: FormFieldKind;
  newFormFieldName: string;
  newFormFieldOptions: string;
  newFormRadioGroup: string;
  newFormRadioOption: string;
  newFormCheckboxChecked: boolean;
  showToast: (msg: string, kind?: 'error') => void;
  setImageSourceDraft: (path: string) => void;
  setImageSourcePath: (path: string) => void;
  setShowImageInsertModal: (open: boolean) => void;
  setShowAddFormFieldModal: (open: boolean) => void;
  setNewFormFieldKind: (kind: FormFieldKind) => void;
  setNewFormFieldName: (name: string) => void;
  setNewFormFieldOptions: (options: string) => void;
  setNewFormRadioGroup: (group: string) => void;
  setNewFormRadioOption: (option: string) => void;
  setNewFormCheckboxChecked: (checked: boolean) => void;
};

export function useAnnotationModesAsset(opts: UseAnnotationModesAssetOptions) {
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
    filePath,
    imageSourcePath,
    imageSourceDraft,
    newFormFieldKind,
    newFormFieldName,
    newFormFieldOptions,
    newFormRadioGroup,
    newFormRadioOption,
    showToast,
    setImageSourceDraft,
    setImageSourcePath,
    setShowImageInsertModal,
    setShowAddFormFieldModal,
    setNewFormFieldKind,
    setNewFormFieldName,
    setNewFormFieldOptions,
    setNewFormRadioGroup,
    setNewFormRadioOption,
    setNewFormCheckboxChecked,
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

  const openImageInsertModal = useCallback(() => {
    if (!filePath) return;
    setImageSourceDraft(imageSourcePath);
    setShowImageInsertModal(true);
  }, [filePath, imageSourcePath, setImageSourceDraft, setShowImageInsertModal]);

  const confirmImageSource = useCallback(async () => {
    const path = imageSourceDraft.trim();
    if (!path) {
      showToast('Enter an image path', 'error');
      return;
    }
    try {
      await invoke<[number, number]>('get_image_dimensions', { path });
      setImageSourcePath(path);
      setShowImageInsertModal(false);
      clearOtherModes(modes);
      setImageInsertMode(true);
      showToast('Click twice on the page to place the image');
    } catch (err) {
      showToast(String(err), 'error');
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [imageSourceDraft, showToast, setImageSourcePath, setShowImageInsertModal, setImageInsertMode]);

  const toggleImageInsertMode = useCallback(() => {
    if (!imageSourcePath) {
      openImageInsertModal();
      return;
    }
    clearOtherModes(modes);
    setImageInsertMode((m) => !m);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [imageSourcePath, openImageInsertModal, setImageInsertMode]);

  const exitImageInsertMode = useCallback(() => {
    cancelDrawing();
    setImageInsertMode(false);
    setFormAddMode(false);
  }, [cancelDrawing, setImageInsertMode, setFormAddMode]);

  const openAddFormFieldModal = useCallback(() => {
    if (!filePath) return;
    setNewFormFieldKind('text');
    setNewFormFieldName('');
    setNewFormFieldOptions('Option A, Option B');
    setNewFormRadioGroup('');
    setNewFormRadioOption('');
    setNewFormCheckboxChecked(false);
    setShowAddFormFieldModal(true);
  }, [
    filePath,
    setNewFormFieldKind,
    setNewFormFieldName,
    setNewFormFieldOptions,
    setNewFormRadioGroup,
    setNewFormRadioOption,
    setNewFormCheckboxChecked,
    setShowAddFormFieldModal,
  ]);

  const confirmAddFormField = useCallback(() => {
    if (newFormFieldKind === 'radio') {
      if (!newFormRadioGroup.trim() || !newFormRadioOption.trim()) {
        showToast('Enter group and option names', 'error');
        return;
      }
    } else if (!newFormFieldName.trim()) {
      showToast('Enter a field name', 'error');
      return;
    }
    if (newFormFieldKind === 'choice') {
      const options = newFormFieldOptions.split(',').map((o) => o.trim()).filter(Boolean);
      if (options.length === 0) {
        showToast('Enter at least one option', 'error');
        return;
      }
    }
    setShowAddFormFieldModal(false);
    clearOtherModes(modes);
    setFormAddMode(true);
    const placementHint = newFormFieldKind === 'text' || newFormFieldKind === 'choice'
      ? 'Click twice on the page to draw the field box'
      : 'Click on the page to place the field';
    showToast(placementHint);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [
    newFormFieldKind,
    newFormFieldName,
    newFormFieldOptions,
    newFormRadioGroup,
    newFormRadioOption,
    showToast,
    setShowAddFormFieldModal,
    setFormAddMode,
  ]);

  const exitFormAddMode = useCallback(() => {
    cancelDrawing();
    setFormAddMode(false);
  }, [cancelDrawing, setFormAddMode]);

  return {
    openImageInsertModal,
    confirmImageSource,
    toggleImageInsertMode,
    exitImageInsertMode,
    openAddFormFieldModal,
    confirmAddFormField,
    exitFormAddMode,
  };
}
