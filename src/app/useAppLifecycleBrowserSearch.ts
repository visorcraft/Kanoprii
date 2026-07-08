import { useCallback } from 'react';
import { usePdfBrowser } from '../pdf/usePdfBrowser';
import { usePdfSearch } from '../pdf/usePdfSearch';
import { usePrintJobs } from '../pdf/usePrintJobs';
import { useClosePdf } from './usePdfLifecycle';
import type { useAppLifecycleLoaders } from './useAppLifecycleLoaders';
import type { UseAppLifecycleDocumentInput } from './appLifecycleTypes';
import type { useAppLifecycleOpen } from './useAppLifecycleOpen';

type Loaders = ReturnType<typeof useAppLifecycleLoaders>;
type OpenSlice = ReturnType<typeof useAppLifecycleOpen>;

export type UseAppLifecycleBrowserSearchInput = UseAppLifecycleDocumentInput & {
  loaders: Loaders;
  open: OpenSlice;
};

export function useAppLifecycleBrowserSearch({ input, loaders, open }: UseAppLifecycleBrowserSearchInput) {
  const { doc, modal, security, panels, annotation, pageRanges } = input;
  const { filePath, originalPath, pageCount, setFilePath, setOriginalPath, setIsDirty, setPageCount, setCurrentPage, setPageInput, setZoom, setViewMode, setMarkdownText, setMarkdownPath, setMarkdownOcrNotice, setPdfRevision, setMarkdownRevision } = doc;
  const {
    lastBrowserDir,
    openFilePath,
    insertFilePath,
    replaceSourcePath,
    interleaveFilePath,
    prependFilePath,
    mergeFilePath,
    setOpenFilePath,
    setInsertFilePath,
    setReplaceSourcePath,
    setReplaceSourcePageCount,
    setReplaceSourcePage,
    setInterleaveFilePath,
    setInterleaveSourcePageCount,
    setPrependFilePath,
    setPrependSourcePageCount,
    setMergeFilePath,
    setShowDeleteModal,
    setShowPrintDialog,
    setPageSizes,
  } = modal;
  const { setShowSignModal, setShowMetadataModal } = security;
  const {
    setHighlightMode,
    setImageInsertMode,
    setFormAddMode,
    setImageSourcePath,
    setShowImageInsertModal,
    setShowAddFormFieldModal,
    setNewFormFieldName,
    setNewFormFieldKind,
    setNewFormFieldOptions,
    setNewFormRadioGroup,
    setNewFormRadioOption,
    setNewFormCheckboxChecked,
  } = annotation;
  const {
    setFormFields,
    setFormDrafts,
    setPdfBookmarks,
    setPdfSignatures,
    setSignatureVerification,
    setShowFormsPanel,
    setShowSignaturesPanel,
    setShowBookmarksPanel,
  } = panels;
  const { interleaveRange, prependRange } = pageRanges;
  const { showToast } = input;

  const browser = usePdfBrowser({
    lastBrowserDir,
    originalPath,
    openFilePath,
    insertFilePath,
    replaceSourcePath,
    interleaveFilePath,
    prependFilePath,
    mergeFilePath,
    withLoading: input.withLoading,
    rememberBrowserDirectory: loaders.rememberBrowserDirectory,
    interleaveRange,
    prependRange,
    setOpenFilePath,
    setInsertFilePath,
    setReplaceSourcePath,
    setReplaceSourcePageCount,
    setReplaceSourcePage,
    setInterleaveFilePath,
    setInterleaveSourcePageCount,
    setPrependFilePath,
    setPrependSourcePageCount,
    setMergeFilePath,
  });

  const search = usePdfSearch({
    filePath,
    search: doc.search,
    patchSearch: doc.patchSearch,
    withLoading: input.withLoading,
    renderPage: open.renderPage,
    setViewMode,
    setCurrentPage,
    setPageInput,
    showToast,
  });

  const { printPages, handlePrint, clearPrintPages } = usePrintJobs({ filePath, pageCount, withLoading: input.withLoading });

  const openPrintDialog = useCallback(() => {
    if (!filePath) return;
    setShowPrintDialog(true);
  }, [filePath, setShowPrintDialog]);

  const { closePdf } = useClosePdf({
    filePath,
    discardHistory: open.discardHistory,
    cancelDrawing: input.cancelDrawing,
    revokeViewerAssets: open.revokeViewerAssets,
    clearPrintPages,
    showToast,
    setFilePath,
    setOriginalPath,
    setIsDirty,
    setPageCount,
    setCurrentPage,
    setPageInput,
    setZoom,
    setViewMode,
    setMarkdownText,
    setMarkdownPath,
    setMarkdownOcrNotice,
    setPdfRevision,
    setMarkdownRevision,
    setHighlightMode,
    setImageInsertMode,
    setFormAddMode,
    setImageSourcePath,
    setShowImageInsertModal,
    setShowFormsPanel,
    setShowSignaturesPanel,
    setShowBookmarksPanel,
    setPdfBookmarks,
    setPageSizes,
    setPdfSignatures,
    setSignatureVerification,
    setShowSignModal,
    setShowMetadataModal,
    setFormFields,
    setFormDrafts,
    setShowAddFormFieldModal,
    setNewFormFieldName,
    setNewFormFieldKind,
    setNewFormFieldOptions,
    setNewFormRadioGroup,
    setNewFormRadioOption,
    setNewFormCheckboxChecked,
    setShowDeleteModal,
  });

  return { browser, search, printPages, handlePrint, openPrintDialog, closePdf };
}
