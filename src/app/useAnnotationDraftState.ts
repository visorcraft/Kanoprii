import { useState } from 'react';
import { STAMP_PRESETS, type ShapeKind, type StampKind } from './constants';
import type { FormFieldKind } from '../modals/AddFormFieldModal';
import type { PageTextEdit, PageVectorEdit } from './types';

export function useAnnotationDraftState() {
  const [highlightMode, setHighlightMode] = useState(false);
  const [noteMode, setNoteMode] = useState(false);
  const [drawMode, setDrawMode] = useState(false);
  const [shapeMode, setShapeMode] = useState(false);
  const [shapeKind, setShapeKind] = useState<ShapeKind>('square');
  const [stampMode, setStampMode] = useState(false);
  const [stampKind, setStampKind] = useState<StampKind>('text');
  const [stampPreset, setStampPreset] = useState<string>(STAMP_PRESETS[0].id);
  const [redactMode, setRedactMode] = useState(false);
  const [imageInsertMode, setImageInsertMode] = useState(false);
  const [textEditMode, setTextEditMode] = useState(false);
  const [editTextRunMode, setEditTextRunMode] = useState(false);
  const [vectorEditMode, setVectorEditMode] = useState(false);
  const [showPageTextModal, setShowPageTextModal] = useState(false);
  const [showPageEditsModal, setShowPageEditsModal] = useState(false);
  const [pendingTextPos, setPendingTextPos] = useState<{ x: number; y: number } | null>(null);
  const [pageTextDraft, setPageTextDraft] = useState('');
  const [pageTextFontSize, setPageTextFontSize] = useState('14');
  const [editingTextIndex, setEditingTextIndex] = useState<number | null>(null);
  const [pageTextEdits, setPageTextEdits] = useState<PageTextEdit[]>([]);
  const [pageVectorEdits, setPageVectorEdits] = useState<PageVectorEdit[]>([]);
  const [showImageInsertModal, setShowImageInsertModal] = useState(false);
  const [imageSourcePath, setImageSourcePath] = useState('');
  const [imageSourceDraft, setImageSourceDraft] = useState('');
  const [formAddMode, setFormAddMode] = useState(false);
  const [showAddFormFieldModal, setShowAddFormFieldModal] = useState(false);
  const [newFormFieldKind, setNewFormFieldKind] = useState<FormFieldKind>('text');
  const [newFormFieldName, setNewFormFieldName] = useState('');
  const [newFormFieldOptions, setNewFormFieldOptions] = useState('Option A, Option B');
  const [newFormRadioGroup, setNewFormRadioGroup] = useState('');
  const [newFormRadioOption, setNewFormRadioOption] = useState('');
  const [newFormCheckboxChecked, setNewFormCheckboxChecked] = useState(false);
  const [showNoteModal, setShowNoteModal] = useState(false);
  const [noteDraft, setNoteDraft] = useState('');
  const [pendingNotePos, setPendingNotePos] = useState<{ x: number; y: number } | null>(null);

  return {
    highlightMode, setHighlightMode,
    noteMode, setNoteMode,
    drawMode, setDrawMode,
    shapeMode, setShapeMode,
    shapeKind, setShapeKind,
    stampMode, setStampMode,
    stampKind, setStampKind,
    stampPreset, setStampPreset,
    redactMode, setRedactMode,
    imageInsertMode, setImageInsertMode,
    textEditMode, setTextEditMode,
    editTextRunMode, setEditTextRunMode,
    vectorEditMode, setVectorEditMode,
    showPageTextModal, setShowPageTextModal,
    showPageEditsModal, setShowPageEditsModal,
    pendingTextPos, setPendingTextPos,
    pageTextDraft, setPageTextDraft,
    pageTextFontSize, setPageTextFontSize,
    editingTextIndex, setEditingTextIndex,
    pageTextEdits, setPageTextEdits,
    pageVectorEdits, setPageVectorEdits,
    showImageInsertModal, setShowImageInsertModal,
    imageSourcePath, setImageSourcePath,
    imageSourceDraft, setImageSourceDraft,
    formAddMode, setFormAddMode,
    showAddFormFieldModal, setShowAddFormFieldModal,
    newFormFieldKind, setNewFormFieldKind,
    newFormFieldName, setNewFormFieldName,
    newFormFieldOptions, setNewFormFieldOptions,
    newFormRadioGroup, setNewFormRadioGroup,
    newFormRadioOption, setNewFormRadioOption,
    newFormCheckboxChecked, setNewFormCheckboxChecked,
    showNoteModal, setShowNoteModal,
    noteDraft, setNoteDraft,
    pendingNotePos, setPendingNotePos,
  };
}

/** Canonical alias for this hook's state shape. */
export type AnnotationState = ReturnType<typeof useAnnotationDraftState>;
