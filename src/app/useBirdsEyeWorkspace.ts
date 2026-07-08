import { invoke } from '@tauri-apps/api/core';
import { useCallback, useMemo, useState } from 'react';
import type { DragEvent } from 'react';
import type { DocumentSessionData, DocumentTabInfo } from './documentSessionTypes';
import type { WorkspaceViewMode } from './types';
import { fileNameFromPath, pickPdfWithNativeDialog } from './utils';

export type BirdsEyeDocument = {
  id: string;
  label: string;
  filePath: string;
  originalPath: string;
  pageCount: number;
  currentPage: number;
  thumbnails: string[];
  active: boolean;
};

export type BirdsEyeWorkspace = {
  documents: BirdsEyeDocument[];
  totalPages: number;
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onOpenDocument: () => void;
  onSelectDocument: (sessionId: string) => void;
  onSelectPage: (sessionId: string, pageIndex: number) => void;
  onOpenPage: (sessionId: string, pageIndex: number) => void;
  onAddPages: (sessionId: string) => void;
  onPageDragStart: (sessionId: string, pageIndex: number) => void;
  onPageDragEnd: () => void;
  onPageDragOver: (event: DragEvent) => void;
  onPageDrop: (sessionId: string, pageIndex: number) => void;
};

type DraggedPage = {
  sessionId: string;
  pageIndex: number;
};

type UseBirdsEyeWorkspaceOptions = {
  sessions: DocumentSessionData[];
  tabs: DocumentTabInfo[];
  activeId: string | null;
  nativeDialogs: boolean;
  lastBrowserDir: string;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  updateSession: (sessionId: string, patch: Partial<DocumentSessionData>) => void;
  setActiveSession: (sessionId: string) => void;
  setWorkspaceView: (mode: WorkspaceViewMode) => void;
  setOpenFilePath: (path: string) => void;
  setShowOpenModal: (open: boolean) => void;
  loadThumbnails: (path: string) => Promise<void>;
  renderPage: (path: string, page: number) => Promise<void>;
  rememberBrowserDirectory: (path: string) => void;
  showToast: (message: string, type?: 'success' | 'error') => void;
};

const clampZoom = (value: number) => Math.max(0.45, Math.min(1.1, value));

export function useBirdsEyeWorkspace({
  sessions,
  tabs,
  activeId,
  nativeDialogs,
  lastBrowserDir,
  withLoading,
  updateSession,
  setActiveSession,
  setWorkspaceView,
  setOpenFilePath,
  setShowOpenModal,
  loadThumbnails,
  renderPage,
  rememberBrowserDirectory,
  showToast,
}: UseBirdsEyeWorkspaceOptions): BirdsEyeWorkspace {
  const [draggedPage, setDraggedPage] = useState<DraggedPage | null>(null);
  const [zoom, setZoom] = useState(0.68);

  const openSessions = useMemo(
    () => sessions.filter((session) => !!session.filePath),
    [sessions],
  );

  const documents = useMemo(
    () =>
      openSessions.map((session) => {
        const tab = tabs.find((item) => item.id === session.id);
        return {
          id: session.id,
          label: tab?.label ?? fileNameFromPath(session.originalPath || session.filePath),
          filePath: session.filePath,
          originalPath: session.originalPath,
          pageCount: session.pageCount ?? session.viewerCache.thumbnails.length,
          currentPage: session.currentPage,
          thumbnails: session.viewerCache.thumbnails,
          active: session.id === activeId,
        };
      }),
    [activeId, openSessions, tabs],
  );

  const totalPages = useMemo(
    () => documents.reduce((sum, document) => sum + document.pageCount, 0),
    [documents],
  );

  const findSession = useCallback(
    (sessionId: string) => openSessions.find((session) => session.id === sessionId) ?? null,
    [openSessions],
  );

  const autosaveSession = useCallback(
    async (session: DocumentSessionData) => {
      if (!session.originalPath) return;
      await invoke('save_working_copy', {
        working: session.filePath,
        target: session.originalPath,
      });
      updateSession(session.id, { isDirty: false });
    },
    [updateSession],
  );

  const refreshSession = useCallback(
    async (sessionId: string, preferredPage = 0) => {
      const session = findSession(sessionId);
      if (!session) return;
      const count = await invoke<number>('get_pdf_page_count', { path: session.filePath });
      const page = Math.max(0, Math.min(preferredPage, Math.max(0, count - 1)));
      updateSession(sessionId, {
        pageCount: count,
        currentPage: page,
        pageInput: String(page + 1),
        viewMode: 'pdf',
        markdownText: '',
        markdownPath: '',
        markdownOcrNotice: null,
        pdfRevision: session.pdfRevision + 1,
        markdownRevision: null,
        isDirty: false,
      });
      await loadThumbnails(session.filePath);
      if (count > 0) await renderPage(session.filePath, page);
    },
    [findSession, loadThumbnails, renderPage, updateSession],
  );

  const onZoomIn = useCallback(() => {
    setZoom((current) => clampZoom(current + 0.08));
  }, []);

  const onZoomOut = useCallback(() => {
    setZoom((current) => clampZoom(current - 0.08));
  }, []);

  const onOpenDocument = useCallback(() => {
    setOpenFilePath('');
    setShowOpenModal(true);
  }, [setOpenFilePath, setShowOpenModal]);

  const onSelectDocument = useCallback(
    (sessionId: string) => {
      if (!findSession(sessionId)) return;
      setActiveSession(sessionId);
    },
    [findSession, setActiveSession],
  );

  const onSelectPage = useCallback(
    (sessionId: string, pageIndex: number) => {
      const session = findSession(sessionId);
      if (!session) return;
      setActiveSession(sessionId);
      updateSession(sessionId, {
        currentPage: pageIndex,
        pageInput: String(pageIndex + 1),
        viewMode: 'pdf',
      });
      void renderPage(session.filePath, pageIndex);
    },
    [findSession, renderPage, setActiveSession, updateSession],
  );

  const onOpenPage = useCallback(
    (sessionId: string, pageIndex: number) => {
      onSelectPage(sessionId, pageIndex);
      setWorkspaceView('tabs');
    },
    [onSelectPage, setWorkspaceView],
  );

  const onAddPages = useCallback(
    (sessionId: string) => {
      const session = findSession(sessionId);
      if (!session) return;
      void withLoading(async () => {
        const insertPath = nativeDialogs
          ? await pickPdfWithNativeDialog(lastBrowserDir || session.originalPath)
          : window.prompt('PDF path to insert')?.trim() || null;
        if (!insertPath) return;
        const sourceCount = await invoke<number>('get_pdf_page_count', { path: insertPath });
        if (sourceCount < 1) {
          showToast('Source PDF has no pages', 'error');
          return;
        }
        const atIndex = session.pageCount ?? session.viewerCache.thumbnails.length;
        await invoke('insert_pdf', {
          path: session.filePath,
          insertPath,
          atIndex,
          insertStart: 0,
          insertEnd: sourceCount - 1,
        });
        rememberBrowserDirectory(insertPath);
        await autosaveSession(session);
        await refreshSession(sessionId, atIndex);
        showToast(`Inserted ${sourceCount} page${sourceCount === 1 ? '' : 's'}`);
      });
    },
    [
      autosaveSession,
      findSession,
      lastBrowserDir,
      nativeDialogs,
      refreshSession,
      rememberBrowserDirectory,
      showToast,
      withLoading,
    ],
  );

  const onPageDragStart = useCallback((sessionId: string, pageIndex: number) => {
    setDraggedPage({ sessionId, pageIndex });
  }, []);

  const onPageDragEnd = useCallback(() => {
    setDraggedPage(null);
  }, []);

  const onPageDragOver = useCallback((event: DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const onPageDrop = useCallback(
    (destSessionId: string, destIndex: number) => {
      if (!draggedPage) return;
      const source = findSession(draggedPage.sessionId);
      const dest = findSession(destSessionId);
      if (!source || !dest) {
        setDraggedPage(null);
        return;
      }
      if (source.id === dest.id && draggedPage.pageIndex === destIndex) {
        setDraggedPage(null);
        return;
      }

      void withLoading(async () => {
        if (source.id === dest.id) {
          await invoke('move_page_between_pdfs', {
            sourcePath: source.filePath,
            destPath: dest.filePath,
            sourceIndex: draggedPage.pageIndex,
            destIndex,
          });
          await autosaveSession(source);
          await refreshSession(source.id, destIndex);
          showToast('Page reordered and saved');
        } else {
          await invoke('move_page_between_pdfs', {
            sourcePath: source.filePath,
            destPath: dest.filePath,
            sourceIndex: draggedPage.pageIndex,
            destIndex,
          });
          await autosaveSession(source);
          await autosaveSession(dest);
          await refreshSession(source.id, Math.min(draggedPage.pageIndex, Math.max(0, (source.pageCount ?? 1) - 2)));
          await refreshSession(dest.id, destIndex);
          showToast('Page moved and saved');
        }
        setDraggedPage(null);
      });
    },
    [autosaveSession, draggedPage, findSession, refreshSession, showToast, withLoading],
  );

  return {
    documents,
    totalPages,
    zoom,
    onZoomIn,
    onZoomOut,
    onOpenDocument,
    onSelectDocument,
    onSelectPage,
    onOpenPage,
    onAddPages,
    onPageDragStart,
    onPageDragEnd,
    onPageDragOver,
    onPageDrop,
  };
}
