import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useRef } from 'react';
import type { DocumentState } from './useAppDocumentState';
import type { AnnotationState } from './useAnnotationDraftState';
import type { ModalState } from './useAppModalState';
import type { PanelsState } from './useDocumentPanelsState';
import type { SecurityState } from './useSecurityFormState';
import type { WorkspaceViewMode } from './types';

type TabActionsDeps = {
  doc: DocumentState;
  modal: ModalState;
  security: SecurityState;
  panels: PanelsState;
  annotation: AnnotationState;
  setWorkspaceView: (mode: WorkspaceViewMode) => void;
  cancelDrawing: () => void;
  showToast: (message: string, type?: 'success' | 'error') => void;
  guardUnsavedForSession: (sessionId: string, action: () => void | Promise<void>) => void;
  discardHistory: (sessionId?: string) => void;
  clearModesOnTabSwitch: () => void;
  renderPage: (path: string, page: number) => Promise<void>;
  loadThumbnails: (path: string) => Promise<void>;
  loadFormFields: (path: string) => Promise<void>;
};

function revokeCache(cache: { imageSrc: string; thumbnails: string[] }) {
  if (cache.imageSrc) URL.revokeObjectURL(cache.imageSrc);
  cache.thumbnails.forEach((url) => URL.revokeObjectURL(url));
}

export function useDocumentTabActions(deps: TabActionsDeps) {
  const prevActiveRef = useRef<string | null>(null);

  const finalizeCloseSession = useCallback(
    async (sessionId: string) => {
      const session = deps.doc.sessions.find((s) => s.id === sessionId);
      if (!session) return;
      if (session.filePath) {
        await invoke('discard_working_copy', { working: session.filePath }).catch(() => {});
      }
      revokeCache(session.viewerCache);
      deps.discardHistory(sessionId);
      deps.doc.removeSession(sessionId);
      if (deps.doc.sessions.length <= 1) {
        deps.setWorkspaceView('tabs');
        deps.modal.setPageSizes([]);
        deps.panels.setFormFields([]);
        deps.panels.setFormDrafts({});
        deps.panels.setPdfBookmarks([]);
        deps.panels.setPdfSignatures([]);
        deps.panels.setSignatureVerification(null);
        deps.panels.setShowFormsPanel(false);
        deps.panels.setShowSignaturesPanel(false);
        deps.panels.setShowBookmarksPanel(false);
        deps.security.setShowSignModal(false);
        deps.security.setShowMetadataModal(false);
        deps.annotation.setHighlightMode(false);
        deps.annotation.setImageInsertMode(false);
        deps.annotation.setFormAddMode(false);
        deps.modal.setShowDeleteModal(false);
      }
    },
    [deps],
  );

  const requestCloseTab = useCallback(
    (sessionId: string) => {
      const session = deps.doc.sessions.find((s) => s.id === sessionId);
      if (!session) return;
      const close = () => void finalizeCloseSession(sessionId);
      if (session.isDirty) {
        deps.guardUnsavedForSession(sessionId, close);
      } else {
        void close();
      }
    },
    [deps, finalizeCloseSession],
  );

  const selectTab = useCallback(
    (sessionId: string) => {
      if (sessionId === deps.doc.activeId) return;
      deps.clearModesOnTabSwitch();
      deps.cancelDrawing();
      deps.doc.setActiveSession(sessionId);
      const session = deps.doc.sessions.find((s) => s.id === sessionId);
      if (session?.filePath && !session.viewerCache.imageSrc) {
        void deps.renderPage(session.filePath, session.currentPage);
        void deps.loadThumbnails(session.filePath);
        void deps.loadFormFields(session.filePath);
      }
    },
    [deps],
  );

  useEffect(() => {
    const prev = prevActiveRef.current;
    const next = deps.doc.activeId;
    if (prev && prev !== next) {
      deps.clearModesOnTabSwitch();
    }
    prevActiveRef.current = next;
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [deps.doc.activeId, deps.clearModesOnTabSwitch]);

  const requestCloseActiveTab = useCallback(() => {
    if (!deps.doc.activeId) return;
    if (deps.doc.sessions.length === 0) return;
    requestCloseTab(deps.doc.activeId);
  }, [deps.doc.activeId, deps.doc.sessions.length, requestCloseTab]);

  return {
    requestCloseTab,
    requestCloseActiveTab,
    selectTab,
    finalizeCloseSession,
    revokeCache,
  };
}
