import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { usePdfRecents } from './usePdfRecents';
import { usePdfDocument } from '../pdf/usePdfDocument';
import { useUndoHistory } from '../pdf/useUndoHistory';
import { usePdfOpen } from './usePdfOpen';
import type { UseAppLifecycleDocumentInput } from './appLifecycleTypes';
import { createWorkingPdf } from './documentImport';

export function useAppLifecycleOpen({ input, loaders }: UseAppLifecycleDocumentInput) {
  const { doc, modal, security, refs, showToast, withLoading, filePathRef, cancelDrawing } = input;
  const {
    filePath,
    originalPath,
    sourcePath,
    sourceKind,
    sourceText,
    activeId,
    updateSession,
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
  const { loadFormFields } = loaders;

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
    originalPath: sourcePath || originalPath,
    openFilePath,
    pendingEncryptedPath,
    pdfPasswordDraft,
    withLoading,
    resetHistoryForOpen,
    renderPage,
    loadThumbnails,
    loadFormFields,
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

  const generatedViewRef = useRef('');
  useEffect(() => {
    if (viewMode !== 'pdf' || sourceKind === 'pdf' || !sourceKind || !sourcePath || !activeId) return;
    const sourceDigest = `${sourceText.length}:${sourceText.slice(0, 64)}`;
    const key = `${activeId}:${sourcePath}:${sourceDigest}`;
    if (generatedViewRef.current === key) return;
    generatedViewRef.current = key;
    const sessionId = activeId;
    void withLoading(async () => {
      showToast('Converting document to PDF…');
      const working = filePath || await createWorkingPdf(sourcePath, sourceText, sourceKind, {
        onProgress: (page, total) => showToast(`Converting document: ${page} of ${total} pages`),
      });
      if (!filePath) {
        await resetHistoryForOpen(working, sessionId);
      }
      // Materialize the generated PDF next to the source so "Save" writes to a
      // real .pdf sibling (not the .md source), but keep the sibling identity in
      // `generatedPdfPath` so `originalPath`/`sourcePath` stays the user's source
      // and tab/title/dedup semantics are not confused with the auto-generated
      // artifact. The sibling is cleaned up on window close via the same field.
      if (!filePath) {
        const generatedPath = await invoke<string>('materialize_document_pdf', { working, source: sourcePath });
        showToast(`Generated ${generatedPath}`);
        updateSession(sessionId, { filePath: working, generatedPdfPath: generatedPath });
      }
      const count = await invoke<number>('get_pdf_page_count', { path: working });
      updateSession(sessionId, { pageCount: count, currentPage: 0, pageInput: '1' });
      await renderPage(working, 0);
      await loadThumbnails(working);
      await loadFormFields(working);
      return true;
    }).then((ok) => {
      if (ok) return;
      generatedViewRef.current = '';
      setViewMode(sourceKind === 'markdown' ? 'markdown' : 'webpage');
    });
  }, [activeId, filePath, loadFormFields, loadThumbnails, originalPath, renderPage, resetHistoryForOpen, setViewMode, showToast, sourceKind, sourcePath, sourceText, updateSession, viewMode, withLoading]);

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
