import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { DocumentSessionData } from './documentSessionTypes';
import type { ViewMode } from './types';

type UsePdfOpenOptions = {
  filePath: string;
  originalPath: string;
  openFilePath: string;
  pendingEncryptedPath: string;
  pdfPasswordDraft: string;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  resetHistoryForOpen: (working: string, sessionId?: string) => Promise<void>;
  renderPage: (path: string, page: number) => Promise<void>;
  loadThumbnails: (path: string) => Promise<void>;
  loadFormFields: (path: string) => Promise<void>;
  rememberOpenedPdf: (path: string) => void;
  cancelDrawing: () => void;
  guardUnsaved: (fn: () => void) => void;
  ensureSessionForOpen: (originalPath: string) => string | null;
  clearOpeningPath: (originalPath: string) => void;
  removeSession: (sessionId: string) => void;
  updateSession: (sessionId: string, patch: Partial<DocumentSessionData>) => void;
  showToast: (msg: string, kind?: 'error') => void;
  setOpenFilePath: (path: string) => void;
  setShowOpenModal: (open: boolean) => void;
  setPendingEncryptedPath: (path: string) => void;
  setPdfPasswordDraft: (password: string) => void;
  setShowPasswordModal: (open: boolean) => void;
};

export function usePdfOpen({
  filePath,
  originalPath,
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
  guardUnsaved,
  ensureSessionForOpen,
  clearOpeningPath,
  removeSession,
  updateSession,
  showToast,
  setOpenFilePath,
  setShowOpenModal,
  setPendingEncryptedPath,
  setPdfPasswordDraft,
  setShowPasswordModal,
}: UsePdfOpenOptions) {
  const loadPdfFromPath = useCallback(async (path: string, password?: string, targetSessionId?: string) => {
    const sessionId = targetSessionId ?? ensureSessionForOpen(path);
    if (sessionId === null) {
      clearOpeningPath(path);
      return true;
    }
    const ownSession = targetSessionId === undefined;
    let loaded = false;
    let needsPassword = false;
    try {
      loaded = (await withLoading(async () => {
        const encrypted = await invoke<boolean>('pdf_is_encrypted', { path });
        if (encrypted && !password) {
          setPendingEncryptedPath(path);
          setPdfPasswordDraft('');
          setShowPasswordModal(true);
          needsPassword = true;
          return false;
        }
        const working = password
          ? await invoke<string>('open_working_copy_with_password', { original: path, password })
          : await invoke<string>('open_working_copy', { original: path });
        const count = await invoke<number>('get_pdf_page_count', { path: working });
        updateSession(sessionId, {
          originalPath: path,
          filePath: working,
          viewMode: 'pdf' as ViewMode,
          markdownText: '',
          markdownPath: '',
          markdownOcrNotice: null,
          pdfRevision: 0,
          markdownRevision: null,
          pageCount: count,
          currentPage: 0,
          zoom: 1,
          pageInput: '1',
          zoomInput: '100',
          isDirty: false,
        });
        await resetHistoryForOpen(working, sessionId);
        cancelDrawing();
        await renderPage(working, 0);
        await loadThumbnails(working);
        await loadFormFields(working);
        rememberOpenedPdf(path);
        // The previous document's working copy stays with its session; it is
        // discarded by closeSession/closePdf, never by opening another file.
        return true;
      })) ?? false;
      return loaded;
    } finally {
      clearOpeningPath(path);
      if (!loaded && ownSession && !needsPassword) {
        removeSession(sessionId);
      }
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [
    filePath,
    ensureSessionForOpen,
    clearOpeningPath,
    removeSession,
    updateSession,
    withLoading,
    resetHistoryForOpen,
    renderPage,
    loadThumbnails,
    loadFormFields,
    rememberOpenedPdf,
    cancelDrawing,
    setPendingEncryptedPath,
    setPdfPasswordDraft,
    setShowPasswordModal,
  ]);

  const openPdf = useCallback(() => {
    guardUnsaved(() => {
      setOpenFilePath(originalPath);
      setShowOpenModal(true);
    });
  }, [guardUnsaved, originalPath, setOpenFilePath, setShowOpenModal]);

  const handleOpenPdfPath = useCallback(async (pathOverride?: string) => {
    const path = (pathOverride ?? openFilePath).trim();
    if (!path) return;
    const loaded = await loadPdfFromPath(path);
    if (loaded) setShowOpenModal(false);
  }, [openFilePath, loadPdfFromPath, setShowOpenModal]);

  const handleOpenEncryptedPdf = useCallback(async () => {
    const path = pendingEncryptedPath.trim();
    const password = pdfPasswordDraft;
    if (!path || !password) return;
    try {
      await invoke('verify_pdf_password', { path, password });
    } catch {
      showToast('Incorrect password', 'error');
      return;
    }
    const loaded = await loadPdfFromPath(path, password);
    if (loaded) {
      setShowPasswordModal(false);
      setShowOpenModal(false);
      setPendingEncryptedPath('');
      setPdfPasswordDraft('');
    }
  }, [
    pendingEncryptedPath,
    pdfPasswordDraft,
    loadPdfFromPath,
    showToast,
    setShowPasswordModal,
    setShowOpenModal,
    setPendingEncryptedPath,
    setPdfPasswordDraft,
  ]);

  const handleOpenRecentPdf = useCallback(async (path: string) => {
    setOpenFilePath(path);
    const loaded = await loadPdfFromPath(path);
    if (loaded) setShowOpenModal(false);
  }, [loadPdfFromPath, setOpenFilePath, setShowOpenModal]);

  return {
    loadPdfFromPath,
    openPdf,
    handleOpenPdfPath,
    handleOpenEncryptedPdf,
    handleOpenRecentPdf,
  };
}
