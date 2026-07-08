import type { HistorySnapshot } from '../pdf/historyTypes';
import type { PdfTextSearchMatch } from '../modals/SearchModal';
import type { PdfAnnotation } from '../pdf/usePdfDocument';
import type { MarkdownOcrNotice, ViewMode } from './types';

/**
 * Per-session vs app-wide state inventory:
 *
 * | Per-session | App-wide |
 * | --- | --- |
 * | path, originalPath, dirty, revision, page, pageCount, zoom | modal state, OCR status, browser/recents |
 * | viewMode, scrollViewMode, markdown*, pageInput, zoomInput | panel visibility toggles |
 * | undo history, find state, viewer cache (imageSrc, thumbnails) | loading, toast |
 * | draggedIndex | annotation/drawing modes (reset on tab switch) |
 *
 * Annotation/drawing modes stay app-wide; switching tabs clears them via the existing mode helper.
 */
export type DocumentSessionId = string;

export type SessionViewerCache = {
  imageSrc: string;
  thumbnails: string[];
  annotations: PdfAnnotation[];
};

export type SessionSearchState = {
  showSearchModal: boolean;
  searchQuery: string;
  searchMatchCase: boolean;
  searchWholeWord: boolean;
  searchResults: PdfTextSearchMatch[];
  searchResultIndex: number;
  activeSearchRect: [number, number, number, number] | null;
};

export const emptySessionSearch = (): SessionSearchState => ({
  showSearchModal: false,
  searchQuery: '',
  searchMatchCase: false,
  searchWholeWord: false,
  searchResults: [],
  searchResultIndex: 0,
  activeSearchRect: null,
});

export const emptyViewerCache = (): SessionViewerCache => ({
  imageSrc: '',
  thumbnails: [],
  annotations: [],
});

export type DocumentSessionData = {
  id: DocumentSessionId;
  filePath: string;
  originalPath: string;
  isDirty: boolean;
  pageCount: number | null;
  currentPage: number;
  draggedIndex: number | null;
  zoom: number;
  viewMode: ViewMode;
  scrollViewMode: 'single' | 'continuous';
  markdownText: string;
  markdownPath: string;
  pdfRevision: number;
  markdownRevision: number | null;
  markdownOcrNotice: MarkdownOcrNotice | null;
  pageInput: string;
  zoomInput: string;
  canUndo: boolean;
  canRedo: boolean;
  viewerCache: SessionViewerCache;
  search: SessionSearchState;
};

export type SessionUndoRefs = {
  history: HistorySnapshot[];
  histIdx: number;
  savedIdx: number;
  deltaNotified: boolean;
};

export type DocumentTabInfo = {
  id: DocumentSessionId;
  label: string;
  dirty: boolean;
  originalPath: string;
  /** Working-copy path (E2E hygiene checks when VITE_WDIO=1). */
  filePath: string;
};
