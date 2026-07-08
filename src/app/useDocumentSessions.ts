import { useCallback, useMemo, useRef, useState, type SetStateAction } from 'react';

import type {
  DocumentSessionData,
  DocumentSessionId,
  DocumentTabInfo,
  SessionSearchState,
  SessionUndoRefs,
  SessionViewerCache,
} from './documentSessionTypes';
import {
  createEmptySessionData,
  createSessionUndoRefs,
  fileStemFromPath,
  nextSessionId,
  normalizeDocPath,
} from './useDocumentSession';
import type { MarkdownOcrNotice, ViewMode } from './types';

function tabLabel(session: DocumentSessionData, all: DocumentSessionData[]): string {
  const stem = fileStemFromPath(session.originalPath || session.filePath);
  const dupes = all.filter(
    (s) => fileStemFromPath(s.originalPath || s.filePath) === stem,
  );
  if (dupes.length <= 1) return stem;
  const path = session.originalPath || session.filePath;
  const parts = path.replace(/\\/g, '/').split('/');
  const parent = parts.length >= 2 ? parts[parts.length - 2] : '';
  return parent ? `${stem} (${parent})` : stem;
}

export function useDocumentSessions() {
  const [sessions, setSessions] = useState<DocumentSessionData[]>([]);
  const [activeId, setActiveId] = useState<DocumentSessionId | null>(null);
  const undoRefs = useRef(new Map<DocumentSessionId, SessionUndoRefs>());
  const openingPathsRef = useRef<Set<string>>(new Set());

  const activeSession = useMemo(
    () => sessions.find((s) => s.id === activeId) ?? null,
    [sessions, activeId],
  );

  const getUndoRefs = useCallback((id: DocumentSessionId): SessionUndoRefs => {
    let refs = undoRefs.current.get(id);
    if (!refs) {
      refs = createSessionUndoRefs();
      undoRefs.current.set(id, refs);
    }
    return refs;
  }, []);

  const updateSession = useCallback(
    (id: DocumentSessionId | null, patch: Partial<DocumentSessionData>) => {
      if (!id) return;
      setSessions((prev) => prev.map((s) => (s.id === id ? { ...s, ...patch } : s)));
    },
    [],
  );

  const patchActive = useCallback(
    (patch: Partial<DocumentSessionData>) => {
      if (!activeId) return;
      updateSession(activeId, patch);
    },
    [activeId, updateSession],
  );

  const tabs: DocumentTabInfo[] = useMemo(
    () =>
      sessions.map((s) => ({
        id: s.id,
        label: tabLabel(s, sessions),
        dirty: s.isDirty,
        originalPath: s.originalPath,
        filePath: s.filePath,
      })),
    [sessions],
  );

  const setFilePath = useCallback(
    (v: string) => patchActive({ filePath: v }),
    [patchActive],
  );
  const setOriginalPath = useCallback(
    (v: string) => patchActive({ originalPath: v }),
    [patchActive],
  );
  const setIsDirty = useCallback(
    (v: boolean) => patchActive({ isDirty: v }),
    [patchActive],
  );
  const setPageCount = useCallback(
    (v: SetStateAction<number | null>) => {
      if (!activeId) return;
      setSessions((prev) =>
        prev.map((s) => {
          if (s.id !== activeId) return s;
          const next = typeof v === 'function' ? v(s.pageCount) : v;
          return { ...s, pageCount: next };
        }),
      );
    },
    [activeId],
  );
  const setCurrentPage = useCallback(
    (v: SetStateAction<number>) => {
      if (!activeId) return;
      setSessions((prev) =>
        prev.map((s) => {
          if (s.id !== activeId) return s;
          const next = typeof v === 'function' ? v(s.currentPage) : v;
          return { ...s, currentPage: next };
        }),
      );
    },
    [activeId],
  );
  const setDraggedIndex = useCallback(
    (v: number | null) => patchActive({ draggedIndex: v }),
    [patchActive],
  );
  const setZoom = useCallback(
    (v: SetStateAction<number>) => {
      if (!activeId) return;
      setSessions((prev) =>
        prev.map((s) => {
          if (s.id !== activeId) return s;
          const next = typeof v === 'function' ? v(s.zoom) : v;
          return { ...s, zoom: next };
        }),
      );
    },
    [activeId],
  );
  const setViewMode = useCallback(
    (v: ViewMode) => patchActive({ viewMode: v }),
    [patchActive],
  );
  const setScrollViewMode = useCallback(
    (v: SetStateAction<'single' | 'continuous'>) => {
      if (!activeId) return;
      setSessions((prev) =>
        prev.map((s) => {
          if (s.id !== activeId) return s;
          const next = typeof v === 'function' ? v(s.scrollViewMode) : v;
          return { ...s, scrollViewMode: next };
        }),
      );
    },
    [activeId],
  );
  const setMarkdownText = useCallback(
    (v: string) => patchActive({ markdownText: v }),
    [patchActive],
  );
  const setMarkdownPath = useCallback(
    (v: string) => patchActive({ markdownPath: v }),
    [patchActive],
  );
  const setPdfRevision = useCallback(
    (v: number | ((prev: number) => number)) => {
      if (!activeId) return;
      setSessions((prev) =>
        prev.map((s) => {
          if (s.id !== activeId) return s;
          const next = typeof v === 'function' ? v(s.pdfRevision) : v;
          return { ...s, pdfRevision: next };
        }),
      );
    },
    [activeId],
  );
  const setMarkdownRevision = useCallback(
    (v: SetStateAction<number | null>) => {
      if (!activeId) return;
      setSessions((prev) =>
        prev.map((s) => {
          if (s.id !== activeId) return s;
          const next = typeof v === 'function' ? v(s.markdownRevision) : v;
          return { ...s, markdownRevision: next };
        }),
      );
    },
    [activeId],
  );
  const setMarkdownOcrNotice = useCallback(
    (v: MarkdownOcrNotice | null) => patchActive({ markdownOcrNotice: v }),
    [patchActive],
  );
  const setPageInput = useCallback(
    (v: SetStateAction<string>) => {
      if (!activeId) return;
      setSessions((prev) =>
        prev.map((s) => {
          if (s.id !== activeId) return s;
          const next = typeof v === 'function' ? v(s.pageInput) : v;
          return { ...s, pageInput: next };
        }),
      );
    },
    [activeId],
  );
  const setZoomInput = useCallback(
    (v: SetStateAction<string>) => {
      if (!activeId) return;
      setSessions((prev) =>
        prev.map((s) => {
          if (s.id !== activeId) return s;
          const next = typeof v === 'function' ? v(s.zoomInput) : v;
          return { ...s, zoomInput: next };
        }),
      );
    },
    [activeId],
  );
  const setCanUndo = useCallback(
    (v: boolean) => patchActive({ canUndo: v }),
    [patchActive],
  );
  const setCanRedo = useCallback(
    (v: boolean) => patchActive({ canRedo: v }),
    [patchActive],
  );

  const setViewerCache = useCallback(
    (cache: SessionViewerCache) => patchActive({ viewerCache: cache }),
    [patchActive],
  );

  const patchViewerCache = useCallback(
    (patch: Partial<SessionViewerCache>) => {
      if (!activeId) return;
      setSessions((prev) =>
        prev.map((s) =>
          s.id === activeId ? { ...s, viewerCache: { ...s.viewerCache, ...patch } } : s,
        ),
      );
    },
    [activeId],
  );

  // Async renders resolve their target session by the working-copy path they
  // rendered, not by whichever tab is active when they complete - opening a
  // second document races setActive and would otherwise corrupt the previous
  // tab's cache. Replaced object URLs are revoked here, the cache's single
  // lifecycle owner.
  const patchViewerCacheForPath = useCallback(
    (path: string, patch: Partial<SessionViewerCache>) => {
      const norm = normalizeDocPath(path);
      setSessions((prev) =>
        prev.map((s) => {
          if (normalizeDocPath(s.filePath) !== norm) return s;
          if (
            patch.imageSrc !== undefined &&
            s.viewerCache.imageSrc &&
            s.viewerCache.imageSrc !== patch.imageSrc
          ) {
            URL.revokeObjectURL(s.viewerCache.imageSrc);
          }
          if (patch.thumbnails !== undefined) {
            s.viewerCache.thumbnails.forEach((url) => {
              if (!patch.thumbnails?.includes(url)) URL.revokeObjectURL(url);
            });
          }
          return { ...s, viewerCache: { ...s.viewerCache, ...patch } };
        }),
      );
    },
    [],
  );

  const patchSearch = useCallback(
    (patch: Partial<SessionSearchState>) => {
      if (!activeId) return;
      setSessions((prev) =>
        prev.map((s) =>
          s.id === activeId ? { ...s, search: { ...s.search, ...patch } } : s,
        ),
      );
    },
    [activeId],
  );

  const findSessionByOriginal = useCallback(
    (originalPath: string) => {
      const norm = normalizeDocPath(originalPath);
      return sessions.find((s) => normalizeDocPath(s.originalPath) === norm) ?? null;
    },
    [sessions],
  );

  const addSession = useCallback((data: DocumentSessionData) => {
    setSessions((prev) => [...prev, data]);
    setActiveId(data.id);
    getUndoRefs(data.id);
  }, [getUndoRefs]);

  const findReusableEmptySession = useCallback(() => {
    return sessions.find((s) => !s.filePath && !s.originalPath) ?? null;
  }, [sessions]);

  /** Focus an already-open path, or return the session id to load into. */
  const ensureSessionForOpen = useCallback(
    (originalPath: string): string | null => {
      const norm = normalizeDocPath(originalPath);
      // Prevent rapid duplicate opens before state updates.
      if (openingPathsRef.current.has(norm)) {
        return null;
      }
      const existing = findSessionByOriginal(originalPath);
      if (existing) {
        setActiveId(existing.id);
        return null;
      }
      openingPathsRef.current.add(norm);
      const reusable = findReusableEmptySession();
      if (reusable) {
        setActiveId(reusable.id);
        return reusable.id;
      }
      const id = nextSessionId();
      addSession(createEmptySessionData(id));
      return id;
    },
    [findSessionByOriginal, findReusableEmptySession, addSession],
  );

  const clearOpeningPath = useCallback((originalPath: string) => {
    openingPathsRef.current.delete(normalizeDocPath(originalPath));
  }, []);

  const removeSession = useCallback((id: DocumentSessionId) => {
    undoRefs.current.delete(id);
    setSessions((prev) => {
      const next = prev.filter((s) => s.id !== id);
      setActiveId((cur) => {
        if (cur !== id) return cur;
        return next.length > 0 ? next[next.length - 1]!.id : null;
      });
      return next;
    });
  }, []);

  const setActiveSession = useCallback((id: DocumentSessionId) => {
    setActiveId(id);
  }, []);

  const cycleTab = useCallback((delta: number) => {
    if (sessions.length < 2) return;
    const idx = sessions.findIndex((s) => s.id === activeId);
    const base = idx >= 0 ? idx : 0;
    const next = (base + delta + sessions.length) % sessions.length;
    setActiveId(sessions[next]!.id);
  }, [sessions, activeId]);

  const jumpToTab = useCallback(
    (index: number) => {
      const session = sessions[index];
      if (session) setActiveId(session.id);
    },
    [sessions],
  );

  const moveTabToFirst = useCallback((id: DocumentSessionId) => {
    setSessions((prev) => {
      const i = prev.findIndex((s) => s.id === id);
      if (i <= 0) return prev;
      const next = prev.slice();
      const [moved] = next.splice(i, 1);
      next.unshift(moved!);
      return next;
    });
  }, []);

  const moveTabToLast = useCallback((id: DocumentSessionId) => {
    setSessions((prev) => {
      const i = prev.findIndex((s) => s.id === id);
      if (i < 0 || i === prev.length - 1) return prev;
      const next = prev.slice();
      const [moved] = next.splice(i, 1);
      next.push(moved!);
      return next;
    });
  }, []);

  const dirtySessions = useMemo(
    () => sessions.filter((s) => s.isDirty),
    [sessions],
  );

  const isDirtyRef = useRef(false);
  isDirtyRef.current = activeSession?.isDirty ?? false;

  const empty = createEmptySessionData('');

  /* eslint-disable react-hooks/exhaustive-deps -- identity is stable whenever the session list/active tab have not changed */
  return useMemo(
    () => ({
      sessions,
      activeId,
      activeSession,
      tabs,
      dirtySessions,
      undoRefs,
      getUndoRefs,
      updateSession,
      addSession,
      removeSession,
      setActiveSession,
      cycleTab,
      jumpToTab,
      moveTabToFirst,
      moveTabToLast,
      findSessionByOriginal,
      ensureSessionForOpen,
      clearOpeningPath,
      setViewerCache,
      patchViewerCache,
      patchViewerCacheForPath,
      patchSearch,
      filePath: activeSession?.filePath ?? empty.filePath,
      originalPath: activeSession?.originalPath ?? empty.originalPath,
      isDirty: activeSession?.isDirty ?? false,
      isDirtyRef,
      pageCount: activeSession?.pageCount ?? null,
      currentPage: activeSession?.currentPage ?? 0,
      draggedIndex: activeSession?.draggedIndex ?? null,
      zoom: activeSession?.zoom ?? 1,
      viewMode: activeSession?.viewMode ?? 'pdf',
      scrollViewMode: activeSession?.scrollViewMode ?? 'single',
      markdownText: activeSession?.markdownText ?? '',
      markdownPath: activeSession?.markdownPath ?? '',
      pdfRevision: activeSession?.pdfRevision ?? 0,
      markdownRevision: activeSession?.markdownRevision ?? null,
      markdownOcrNotice: activeSession?.markdownOcrNotice ?? null,
      pageInput: activeSession?.pageInput ?? '1',
      zoomInput: activeSession?.zoomInput ?? '100',
      canUndo: activeSession?.canUndo ?? false,
      canRedo: activeSession?.canRedo ?? false,
      viewerCache: activeSession?.viewerCache,
      search: activeSession?.search,
      setFilePath,
      setOriginalPath,
      setIsDirty,
      setPageCount,
      setCurrentPage,
      setDraggedIndex,
      setZoom,
      setViewMode,
      setScrollViewMode,
      setMarkdownText,
      setMarkdownPath,
      setPdfRevision,
      setMarkdownRevision,
      setMarkdownOcrNotice,
      setPageInput,
      setZoomInput,
      setCanUndo,
      setCanRedo,
      hasOpenPdf: !!activeSession?.filePath,
    }),
    [sessions, activeId],
  );
  /* eslint-enable react-hooks/exhaustive-deps */
}
