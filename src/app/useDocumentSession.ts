import type { DocumentSessionData, SessionUndoRefs } from './documentSessionTypes';
import { emptySessionSearch, emptyViewerCache } from './documentSessionTypes';

let sessionCounter = 0;

export function nextSessionId(): string {
  sessionCounter += 1;
  return `session-${sessionCounter}-${Date.now()}`;
}

export function createEmptySessionData(id: string): DocumentSessionData {
  return {
    id,
    filePath: '',
    originalPath: '',
    isDirty: false,
    pageCount: null,
    currentPage: 0,
    draggedIndex: null,
    zoom: 1,
    viewMode: 'pdf',
    scrollViewMode: 'single',
    markdownText: '',
    markdownPath: '',
    pdfRevision: 0,
    markdownRevision: null,
    markdownOcrNotice: null,
    pageInput: '1',
    zoomInput: '100',
    canUndo: false,
    canRedo: false,
    viewerCache: emptyViewerCache(),
    search: emptySessionSearch(),
  };
}

export function createSessionUndoRefs(): SessionUndoRefs {
  return {
    history: [],
    histIdx: 0,
    savedIdx: 0,
    deltaNotified: false,
  };
}

export function fileStemFromPath(path: string): string {
  if (!path) return 'Untitled';
  const parts = path.replace(/\\/g, '/').split('/');
  const name = parts[parts.length - 1] ?? path;
  const dot = name.lastIndexOf('.');
  return dot > 0 ? name.slice(0, dot) : name;
}

export function normalizeDocPath(path: string): string {
  return path.replace(/\\/g, '/').toLowerCase();
}
