import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { FormFieldKind } from '../modals/AddFormFieldModal';
import type { ViewMode } from './types';

type UsePdfLifecycleOptions = {
  filePath: string;
  discardHistory: () => void;
  cancelDrawing: () => void;
  revokeViewerAssets: () => void;
  clearPrintPages: () => void;
  showToast: (message: string, type?: 'success' | 'error') => void;
  setFilePath: (path: string) => void;
  setOriginalPath: (path: string) => void;
  setIsDirty: (dirty: boolean) => void;
  setPageCount: (count: number | null) => void;
  setCurrentPage: (page: number) => void;
  setPageInput: (value: string) => void;
  setZoom: (zoom: number) => void;
  setViewMode: (mode: ViewMode) => void;
  setMarkdownText: (text: string) => void;
  setMarkdownPath: (path: string) => void;
  setMarkdownOcrNotice: (notice: null) => void;
  setPdfRevision: React.Dispatch<React.SetStateAction<number>>;
  setMarkdownRevision: (rev: number | null) => void;
  setHighlightMode: (on: boolean) => void;
  setImageInsertMode: (on: boolean) => void;
  setFormAddMode: (on: boolean) => void;
  setImageSourcePath: (path: string) => void;
  setShowImageInsertModal: (show: boolean) => void;
  setShowFormsPanel: (show: boolean) => void;
  setShowSignaturesPanel: (show: boolean) => void;
  setShowBookmarksPanel: (show: boolean) => void;
  setPdfBookmarks: (bookmarks: []) => void;
  setPageSizes: (sizes: []) => void;
  setPdfSignatures: (sigs: []) => void;
  setSignatureVerification: (summary: null) => void;
  setShowSignModal: (show: boolean) => void;
  setShowMetadataModal: (show: boolean) => void;
  setFormFields: (fields: []) => void;
  setFormDrafts: (drafts: Record<string, never>) => void;
  setShowAddFormFieldModal: (show: boolean) => void;
  setNewFormFieldName: (name: string) => void;
  setNewFormFieldKind: (kind: FormFieldKind) => void;
  setNewFormFieldOptions: (options: string) => void;
  setNewFormRadioGroup: (group: string) => void;
  setNewFormRadioOption: (option: string) => void;
  setNewFormCheckboxChecked: (checked: boolean) => void;
  setShowDeleteModal: (show: boolean) => void;
};

export function useClosePdf(opts: UsePdfLifecycleOptions) {
  const closePdf = useCallback(() => {
    if (opts.filePath) {
      void invoke('discard_working_copy', { working: opts.filePath }).catch(() => {});
    }
    opts.discardHistory();
    opts.cancelDrawing();
    opts.setFilePath('');
    opts.setOriginalPath('');
    opts.setIsDirty(false);
    opts.setPageCount(null);
    opts.setCurrentPage(0);
    opts.setPageInput('1');
    opts.setZoom(1);
    opts.setViewMode('pdf');
    opts.setMarkdownText('');
    opts.setMarkdownPath('');
    opts.setMarkdownOcrNotice(null);
    opts.setPdfRevision(0);
    opts.setMarkdownRevision(null);
    opts.setHighlightMode(false);
    opts.setImageInsertMode(false);
    opts.setFormAddMode(false);
    opts.setImageSourcePath('');
    opts.setShowImageInsertModal(false);
    opts.setShowFormsPanel(false);
    opts.setShowSignaturesPanel(false);
    opts.setShowBookmarksPanel(false);
    opts.setPdfBookmarks([]);
    opts.setPageSizes([]);
    opts.setPdfSignatures([]);
    opts.setSignatureVerification(null);
    opts.setShowSignModal(false);
    opts.setShowMetadataModal(false);
    opts.setFormFields([]);
    opts.setFormDrafts({});
    opts.setShowAddFormFieldModal(false);
    opts.setNewFormFieldName('');
    opts.setNewFormFieldKind('text');
    opts.setNewFormFieldOptions('Option A, Option B');
    opts.setNewFormRadioGroup('');
    opts.setNewFormRadioOption('');
    opts.setNewFormCheckboxChecked(false);
    opts.setShowDeleteModal(false);
    opts.revokeViewerAssets();
    opts.clearPrintPages();
    opts.showToast('PDF closed');
  // Setters from useState are stable; filePath is read at invoke time.
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.filePath]);

  return { closePdf };
}
