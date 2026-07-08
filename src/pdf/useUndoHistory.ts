import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAnnouncer } from '../ui/useAnnouncer';
import type { SessionUndoRefs } from '../app/documentSessionTypes';
import {
  type HistorySnapshot,
  MAX_UNDO_HISTORY,
  SNAPSHOT_BYTE_LIMIT,
} from './historyTypes';

type ViewMode = 'pdf' | 'markdown';

export type UseUndoHistoryDeps = {
  filePathRef: React.MutableRefObject<string>;
  activeSessionId: string | null;
  getUndoRefs: (id: string) => SessionUndoRefs;
  setCanUndo: (v: boolean) => void;
  setCanRedo: (v: boolean) => void;
  showToast: (message: string, type?: 'success' | 'error') => void;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  onRestore: () => Promise<void>;
  setPdfRevision: React.Dispatch<React.SetStateAction<number>>;
  setViewMode: (mode: ViewMode) => void;
  setIsDirty: (dirty: boolean) => void;
};

function discardHistoryEntries(entries: HistorySnapshot[]) {
  entries.forEach((entry) => void invoke('discard_history_entry', { entry }).catch(() => {}));
}

export function useUndoHistory({
  filePathRef,
  activeSessionId,
  getUndoRefs,
  setCanUndo,
  setCanRedo,
  showToast,
  withLoading,
  onRestore,
  setPdfRevision,
  setViewMode,
  setIsDirty,
}: UseUndoHistoryDeps) {
  const { announce } = useAnnouncer();
  const sessionRefs = useCallback((): SessionUndoRefs | null => {
    if (!activeSessionId) return null;
    return getUndoRefs(activeSessionId);
  }, [activeSessionId, getUndoRefs]);

  const refreshUndoRedoState = useCallback(() => {
    const refs = sessionRefs();
    if (!refs) {
      setCanUndo(false);
      setCanRedo(false);
      return;
    }
    setCanUndo(refs.histIdx > 0);
    setCanRedo(refs.histIdx < refs.history.length - 1);
    setIsDirty(refs.histIdx !== refs.savedIdx);
  }, [sessionRefs, setCanRedo, setCanUndo, setIsDirty]);

  const clearHistoryState = useCallback(
    (refs: SessionUndoRefs) => {
      refs.history = [];
      refs.histIdx = 0;
      refs.savedIdx = 0;
      refs.deltaNotified = false;
      setCanUndo(false);
      setCanRedo(false);
    },
    [setCanRedo, setCanUndo],
  );

  const pruneUndoHistory = useCallback(async (refs: SessionUndoRefs) => {
    while (refs.history.length > MAX_UNDO_HISTORY) {
      const dropAt = refs.savedIdx === 0 ? 1 : 0;
      if (refs.history.length <= dropAt) break;
      try {
        refs.history = await invoke<HistorySnapshot[]>('prune_history_entry', {
          history: refs.history,
          dropIndex: dropAt,
        });
      } catch {
        /* best-effort */
      }
      if (refs.histIdx > dropAt) refs.histIdx -= 1;
      else if (refs.histIdx === dropAt) refs.histIdx = Math.max(0, dropAt - 1);
      if (refs.savedIdx > dropAt) refs.savedIdx -= 1;
    }
  }, []);

  const recordHistory = useCallback(async () => {
    const working = filePathRef.current;
    const refs = sessionRefs();
    if (!working || !refs) return;
    try {
      const size = await invoke<number>('file_byte_size', { path: working });
      const snapshot = await invoke<HistorySnapshot>('snapshot_pdf_entry', {
        history: refs.history.slice(0, refs.histIdx + 1),
        source: working,
      });
      if (size > SNAPSHOT_BYTE_LIMIT && snapshot.kind === 'delta' && !refs.deltaNotified) {
        refs.deltaNotified = true;
        showToast('Large file: using compact undo snapshots', 'success');
      }
      const redoBranch = refs.history.slice(refs.histIdx + 1);
      discardHistoryEntries(redoBranch);
      refs.history = refs.history.slice(0, refs.histIdx + 1);
      refs.history.push(snapshot);
      refs.histIdx = refs.history.length - 1;
      await pruneUndoHistory(refs);
      refreshUndoRedoState();
    } catch (err) {
      showToast(`Undo snapshot failed: ${String(err)}`, 'error');
    }
  }, [filePathRef, pruneUndoHistory, refreshUndoRedoState, sessionRefs, showToast]);

  const markPdfEdited = useCallback(() => {
    setPdfRevision((revision) => revision + 1);
    setViewMode('pdf');
    setIsDirty(true);
    void recordHistory();
  }, [recordHistory, setIsDirty, setPdfRevision, setViewMode]);

  const resetHistoryForOpen = useCallback(
    async (working: string, sessionId?: string) => {
      const id = sessionId ?? activeSessionId;
      if (!id) return;
      const refs = getUndoRefs(id);
      const oldHistory = [...refs.history];
      try {
        const baseline = await invoke<HistorySnapshot>('snapshot_pdf_entry', { history: [], source: working });
        discardHistoryEntries(oldHistory);
        refs.history = [baseline];
        refs.histIdx = 0;
        refs.savedIdx = 0;
        refs.deltaNotified = false;
        if (id === activeSessionId) {
          setCanUndo(false);
          setCanRedo(false);
          setIsDirty(false);
        }
      } catch (err) {
        discardHistoryEntries(oldHistory);
        clearHistoryState(refs);
        if (id === activeSessionId) setIsDirty(false);
        showToast(`Undo history unavailable: ${String(err)}`, 'error');
      }
    },
    [activeSessionId, clearHistoryState, getUndoRefs, setCanRedo, setCanUndo, setIsDirty, showToast],
  );

  const markSaved = useCallback(() => {
    const refs = sessionRefs();
    if (!refs) return;
    refs.savedIdx = refs.histIdx;
    refreshUndoRedoState();
  }, [refreshUndoRedoState, sessionRefs]);

  const discardHistory = useCallback(
    (sessionId?: string) => {
      const id = sessionId ?? activeSessionId;
      if (!id) return;
      const refs = getUndoRefs(id);
      discardHistoryEntries(refs.history);
      clearHistoryState(refs);
    },
    [activeSessionId, clearHistoryState, getUndoRefs],
  );

  const undo = useCallback(
    async (filePath: string) => {
      const refs = sessionRefs();
      if (!filePath || !refs || refs.histIdx <= 0) return;
      await withLoading(async () => {
        const prevIdx = refs.histIdx;
        refs.histIdx -= 1;
        try {
          await invoke('restore_history_entry', {
            history: refs.history,
            index: refs.histIdx,
            target: filePath,
          });
        } catch (err) {
          refs.histIdx = prevIdx;
          throw err;
        }
        await onRestore();
        refreshUndoRedoState();
        announce('Undo');
      });
    },
    [announce, onRestore, refreshUndoRedoState, sessionRefs, withLoading],
  );

  const redo = useCallback(
    async (filePath: string) => {
      const refs = sessionRefs();
      if (!filePath || !refs || refs.histIdx >= refs.history.length - 1) return;
      await withLoading(async () => {
        const prevIdx = refs.histIdx;
        refs.histIdx += 1;
        try {
          await invoke('restore_history_entry', {
            history: refs.history,
            index: refs.histIdx,
            target: filePath,
          });
        } catch (err) {
          refs.histIdx = prevIdx;
          throw err;
        }
        await onRestore();
        refreshUndoRedoState();
        announce('Redo');
      });
    },
    [announce, onRestore, refreshUndoRedoState, sessionRefs, withLoading],
  );

  return {
    markPdfEdited,
    refreshUndoRedoState,
    resetHistoryForOpen,
    markSaved,
    discardHistory,
    undo,
    redo,
  };
}
