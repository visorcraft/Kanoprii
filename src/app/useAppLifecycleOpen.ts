import { usePdfRecents } from './usePdfRecents';
import { usePdfDocument } from '../pdf/usePdfDocument';
import { useUndoHistory } from '../pdf/useUndoHistory';
import { usePdfOpen } from './usePdfOpen';
import type { UseAppLifecycleDocumentInput } from './appLifecycleTypes';

export function useAppLifecycleOpen({ input, loaders }: UseAppLifecycleDocumentInput) {
  const { doc, modal, security, refs, showToast, withLoading, filePathRef, cancelDrawing } = input;
  const {
    filePath,
    originalPath,
    setIsDirty,
    pageCount,
    currentPage,
    viewMode,
    setPageCount,
    setCurrentPage,
    setPageInput,
    setViewMode,
    setPdfRevision,
    setMarkdownRevision,
  } = doc;

  const { openFilePath, setOpenFilePath, setRecentPdfs, setShowOpenModal } = modal;
  const {
    pendingEncryptedPath,
    pdfPasswordDraft,
    setPendingEncryptedPath,
    setPdfPasswordDraft,
    setShowPasswordModal,
  } = security;

  const { rememberOpenedPdf } = usePdfRecents({ rememberBrowserDirectory: loaders.rememberBrowserDirectory, setRecentPdfs });

  const {
    imageSrc,
    thumbnails,
    annotations,
    setAnnotations,
    loadThumbnails,
    renderPage,
    goToPage,
    reloadOpenPdf,
    refreshAfterWorkingChange,
    revokeViewerAssets,
  } = usePdfDocument({
    filePath,
    pageCount,
    currentPage,
    viewMode,
    setPageCount,
    setCurrentPage,
    setPageInput,
    setViewMode,
    setPdfRevision,
    setMarkdownRevision,
    withLoading,
    loadPageEdits: loaders.loadPageEdits,
    loadPdfBookmarks: (path) => refs.loadPdfBookmarksRef.current(path),
    loadPageSizes: (path) => refs.loadPageSizesRef.current(path),
    cancelDrawing,
    showHiddenLayers: input.panels.showHiddenLayers,
    activeSessionId: doc.activeId,
    viewerCache: doc.viewerCache,
    patchViewerCache: doc.patchViewerCache,
    patchViewerCacheForPath: doc.patchViewerCacheForPath,
  });

  const {
    markPdfEdited,
    resetHistoryForOpen,
    markSaved,
    discardHistory,
    undo: undoHistory,
    redo: redoHistory,
  } = useUndoHistory({
    filePathRef,
    activeSessionId: doc.activeId,
    getUndoRefs: doc.getUndoRefs,
    setCanUndo: doc.setCanUndo,
    setCanRedo: doc.setCanRedo,
    showToast,
    withLoading,
    onRestore: refreshAfterWorkingChange,
    setPdfRevision,
    setViewMode,
    setIsDirty,
  });

  const canUndo = doc.canUndo;
  const canRedo = doc.canRedo;

  const undo = () => undoHistory(filePath);
  const redo = () => redoHistory(filePath);

  const {
    loadPdfFromPath,
    openPdf,
    handleOpenPdfPath,
    handleOpenEncryptedPdf,
    handleOpenRecentPdf,
  } = usePdfOpen({
    filePath,
    originalPath,
    openFilePath,
    pendingEncryptedPath,
    pdfPasswordDraft,
    withLoading,
    resetHistoryForOpen,
    renderPage,
    loadThumbnails,
    loadFormFields: loaders.loadFormFields,
    rememberOpenedPdf,
    cancelDrawing,
    guardUnsaved: loaders.guardUnsaved,
    ensureSessionForOpen: doc.ensureSessionForOpen,
    clearOpeningPath: doc.clearOpeningPath,
    removeSession: doc.removeSession,
    updateSession: doc.updateSession,
    showToast,
    setOpenFilePath,
    setShowOpenModal,
    setPendingEncryptedPath,
    setPdfPasswordDraft,
    setShowPasswordModal,
  });

  return {
    imageSrc,
    thumbnails,
    annotations,
    setAnnotations,
    loadThumbnails,
    renderPage,
    goToPage,
    reloadOpenPdf,
    canUndo,
    canRedo,
    markPdfEdited,
    markSaved,
    undo,
    redo,
    loadPdfFromPath,
    openPdf,
    handleOpenPdfPath,
    handleOpenEncryptedPdf,
    handleOpenRecentPdf,
    rememberOpenedPdf,
    revokeViewerAssets,
    discardHistory,
  };
}
