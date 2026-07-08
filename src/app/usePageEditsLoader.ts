import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { PageTextEdit, PageVectorEdit } from './types';

type UsePageEditsLoaderOptions = {
  setPageTextEdits: (edits: PageTextEdit[]) => void;
  setPageVectorEdits: (edits: PageVectorEdit[]) => void;
};

export function usePageEditsLoader(opts: UsePageEditsLoaderOptions) {
  const loadPageEdits = useCallback(async (path: string, page: number) => {
    if (!path) {
      opts.setPageTextEdits([]);
      opts.setPageVectorEdits([]);
      return;
    }
    try {
      const [texts, vectors] = await Promise.all([
        invoke<PageTextEdit[]>('list_page_text_edits', { path, pageIndex: page }),
        invoke<PageVectorEdit[]>('list_page_vectors', { path, pageIndex: page }),
      ]);
      opts.setPageTextEdits(texts);
      opts.setPageVectorEdits(vectors);
    } catch {
      opts.setPageTextEdits([]);
      opts.setPageVectorEdits([]);
    }
  }, [opts]);

  return { loadPageEdits };
}
