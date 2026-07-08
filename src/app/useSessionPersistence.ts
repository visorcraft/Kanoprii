import { useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { DocumentSessionData } from './documentSessionTypes';
import type { ViewMode, WorkspaceViewMode } from './types';

export interface PersistedSession {
  original_path: string;
  page: number;
  zoom: number;
  view_mode: string;
  scroll_view_mode: string;
}

export interface SessionState {
  version: number;
  active_index: number;
  workspace_view?: string;
  sessions: PersistedSession[];
}

const SAVE_DEBOUNCE_MS = 200;
const VALID_VIEW_MODES: ViewMode[] = ['pdf', 'markdown'];
const VALID_SCROLL_MODES: Array<'single' | 'continuous'> = ['single', 'continuous'];
const VALID_WORKSPACE_VIEW_MODES: WorkspaceViewMode[] = ['tabs', 'birdseye'];

function toState(sessions: DocumentSessionData[], activeId: string | null, workspaceView: WorkspaceViewMode): SessionState {
  const withPath = sessions.filter((s) => s.originalPath);
  const activeIndex = activeId ? withPath.findIndex((s) => s.id === activeId) : 0;
  return {
    version: 1,
    active_index: Math.max(0, activeIndex),
    workspace_view: workspaceView,
    sessions: withPath.map((s) => ({
      original_path: s.originalPath,
      page: s.currentPage,
      zoom: s.zoom,
      view_mode: s.viewMode,
      scroll_view_mode: s.scrollViewMode,
    })),
  };
}

function validateViewMode(v: string): ViewMode {
  return VALID_VIEW_MODES.includes(v as ViewMode) ? (v as ViewMode) : 'pdf';
}

function validateScrollMode(v: string): 'single' | 'continuous' {
  return VALID_SCROLL_MODES.includes(v as 'single' | 'continuous') ? (v as 'single' | 'continuous') : 'single';
}

function validateWorkspaceView(v: string | undefined): WorkspaceViewMode {
  return VALID_WORKSPACE_VIEW_MODES.includes(v as WorkspaceViewMode) ? (v as WorkspaceViewMode) : 'tabs';
}

export function useSessionPersistence({
  sessions,
  activeId,
  workspaceView,
  updateSession,
  removeSession,
  ensureSessionForOpen,
  loadPdfFromPath,
  setActiveSession,
  setWorkspaceView,
  showToast,
  isSpawned = false,
}: {
  sessions: DocumentSessionData[];
  activeId: string | null;
  workspaceView: WorkspaceViewMode;
  updateSession: (id: string, patch: Partial<DocumentSessionData>) => void;
  removeSession: (id: string) => void;
  ensureSessionForOpen: (originalPath: string) => string | null;
  loadPdfFromPath: (path: string, password?: string, targetSessionId?: string) => Promise<boolean>;
  setActiveSession: (id: string) => void;
  setWorkspaceView: (mode: WorkspaceViewMode) => void;
  showToast: (msg: string, kind?: 'error') => void;
  isSpawned?: boolean;
}) {
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isRestoringRef = useRef(false);
  const restoreAttemptedRef = useRef(false);

  const saveSessions = useCallback(async () => {
    // Spawned document windows are ephemeral: never persist (they share one
    // sessions.json with the main window and would clobber it).
    if (isSpawned) return;
    if (!restoreAttemptedRef.current) return;
    if (isRestoringRef.current) return;
    const state = toState(sessions, activeId, workspaceView);
    try {
      await invoke('save_session_state', { state });
    } catch {
      // Silent fail - session restore is best-effort.
    }
  }, [sessions, activeId, workspaceView, isSpawned]);

  const saveSessionsRef = useRef(saveSessions);
  saveSessionsRef.current = saveSessions;

  useEffect(() => {
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => {
      void saveSessions();
    }, SAVE_DEBOUNCE_MS);
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    };
  }, [sessions, activeId, saveSessions]);

  // Flush pending save before window closes.
  useEffect(() => {
    const handler = () => {
      if (saveTimerRef.current) {
        clearTimeout(saveTimerRef.current);
        saveTimerRef.current = null;
      }
      void saveSessionsRef.current();
    };
    window.addEventListener('beforeunload', handler);
    return () => window.removeEventListener('beforeunload', handler);
  }, []);

  const restoreSessions = useCallback(async (shouldSkipActiveRestore?: () => boolean) => {
    try {
      const state = await invoke<SessionState | null>('load_session_state');
      if (!state) {
        restoreAttemptedRef.current = true;
        return;
      }
      const restoredWorkspaceView = validateWorkspaceView(state.workspace_view);
      if (!state.sessions.length) {
        // No documents to show - never land on an empty Bird's Eye workspace.
        if (!shouldSkipActiveRestore?.()) setWorkspaceView('tabs');
        restoreAttemptedRef.current = true;
        return;
      }
      isRestoringRef.current = true;
      const opened: { sessionId: string; page: number; zoom: number; viewMode: ViewMode; scrollViewMode: 'single' | 'continuous' }[] = [];
      for (const s of state.sessions) {
        const sessionId = ensureSessionForOpen(s.original_path);
        if (sessionId === null) {
          // Already open - update its target state for later.
          const existing = sessions.find((es) => es.originalPath === s.original_path);
          if (existing) {
            opened.push({
              sessionId: existing.id,
              page: s.page,
              zoom: s.zoom,
              viewMode: validateViewMode(s.view_mode),
              scrollViewMode: validateScrollMode(s.scroll_view_mode),
            });
          }
          continue;
        }
        try {
          const loaded = await loadPdfFromPath(s.original_path, undefined, sessionId);
          if (loaded) {
            opened.push({
              sessionId,
              page: s.page,
              zoom: s.zoom,
              viewMode: validateViewMode(s.view_mode),
              scrollViewMode: validateScrollMode(s.scroll_view_mode),
            });
          } else {
            removeSession(sessionId);
            showToast(`Skipped restore for ${s.original_path}`, 'error');
          }
        } catch {
          removeSession(sessionId);
          showToast(`Could not restore ${s.original_path}`, 'error');
        }
      }
      // Apply restored per-session state after all opens complete.
      for (const o of opened) {
        updateSession(o.sessionId, {
          currentPage: o.page,
          zoom: o.zoom,
          viewMode: o.viewMode,
          scrollViewMode: o.scrollViewMode,
          pageInput: String(o.page + 1),
          zoomInput: String(Math.round(o.zoom * 100)),
        });
      }
      // Restore the previously active tab.
      const target = opened[state.active_index] ?? opened[0];
      if (!shouldSkipActiveRestore?.()) {
        // Only restore Bird's Eye when at least one document reopened; an empty
        // workspace should always land on the welcome screen.
        setWorkspaceView(opened.length > 0 ? restoredWorkspaceView : 'tabs');
        if (target) setActiveSession(target.sessionId);
      }
      isRestoringRef.current = false;
      restoreAttemptedRef.current = true;
    } catch {
      isRestoringRef.current = false;
      restoreAttemptedRef.current = true;
    }
  }, [ensureSessionForOpen, loadPdfFromPath, updateSession, removeSession, setActiveSession, setWorkspaceView, showToast, sessions]);

  return { restoreSessions };
}
