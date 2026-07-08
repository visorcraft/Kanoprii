import { invoke } from '@tauri-apps/api/core';
import { useRef } from 'react';
import type { SessionSearchState } from '../app/documentSessionTypes';
import type { PdfTextSearchMatch } from '../modals/SearchModal';
import type { ViewMode } from '../app/types';

type UsePdfSearchOptions = {
  filePath: string;
  search: SessionSearchState | undefined;
  patchSearch: (patch: Partial<SessionSearchState>) => void;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  renderPage: (path: string, page: number) => Promise<void>;
  setViewMode: (mode: ViewMode) => void;
  setCurrentPage: (page: number) => void;
  setPageInput: (value: string) => void;
  showToast: (message: string, type?: 'success' | 'error') => void;
};

export function usePdfSearch({
  filePath,
  search,
  patchSearch,
  withLoading,
  renderPage,
  setViewMode,
  setCurrentPage,
  setPageInput,
  showToast,
}: UsePdfSearchOptions) {
  const searchInputRef = useRef<HTMLInputElement>(null);

  const showSearchModal = search?.showSearchModal ?? false;
  const searchQuery = search?.searchQuery ?? '';
  const searchMatchCase = search?.searchMatchCase ?? false;
  const searchWholeWord = search?.searchWholeWord ?? false;
  const searchResults = search?.searchResults ?? [];
  const searchResultIndex = search?.searchResultIndex ?? 0;
  const activeSearchRect = search?.activeSearchRect ?? null;

  const openSearchModal = () => {
    if (!filePath) return;
    patchSearch({ showSearchModal: true });
    window.requestAnimationFrame(() => searchInputRef.current?.focus());
  };

  const closeSearchModal = () => {
    patchSearch({ showSearchModal: false, activeSearchRect: null });
  };

  const goToSearchMatch = async (index: number, results: PdfTextSearchMatch[] = searchResults) => {
    if (!filePath || results.length === 0) return;
    const clamped = Math.max(0, Math.min(index, results.length - 1));
    const match = results[clamped];
    patchSearch({ searchResultIndex: clamped, activeSearchRect: match.rect });
    setViewMode('pdf');
    setCurrentPage(match.page_index);
    setPageInput(String(match.page_index + 1));
    await withLoading(() => renderPage(filePath, match.page_index));
  };

  const runPdfSearch = async () => {
    if (!filePath || !searchQuery.trim()) return;
    await withLoading(async () => {
      const results = await invoke<PdfTextSearchMatch[]>('search_pdf_text', {
        path: filePath,
        query: searchQuery.trim(),
        matchCase: searchMatchCase,
        matchWholeWord: searchWholeWord,
      });
      patchSearch({ searchResults: results });
      if (results.length === 0) {
        patchSearch({ searchResultIndex: 0, activeSearchRect: null });
        showToast('No matches found', 'error');
        return;
      }
      showToast(`${results.length} match${results.length === 1 ? '' : 'es'} found`);
      await goToSearchMatch(0, results);
    });
  };

  const stepSearchMatch = (delta: number) => {
    if (searchResults.length === 0) return;
    const next = (searchResultIndex + delta + searchResults.length) % searchResults.length;
    void goToSearchMatch(next);
  };

  return {
    showSearchModal,
    searchQuery,
    setSearchQuery: (v: string) => patchSearch({ searchQuery: v }),
    searchMatchCase,
    setSearchMatchCase: (v: boolean) => patchSearch({ searchMatchCase: v }),
    searchWholeWord,
    setSearchWholeWord: (v: boolean) => patchSearch({ searchWholeWord: v }),
    searchResults,
    searchResultIndex,
    activeSearchRect,
    searchInputRef,
    openSearchModal,
    closeSearchModal,
    runPdfSearch,
    stepSearchMatch,
  };
}
