import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { PDF_BASE_HEIGHT, PDF_BASE_WIDTH } from './usePdfDocument';

const CACHE_LIMIT = 20;

type CacheEntry = {
  url: string;
  key: string;
};

export function usePageRenderQueue(filePath: string, pdfRevision: number, showHiddenLayers: boolean) {
  const cacheRef = useRef(new Map<number, CacheEntry>());
  const inflightRef = useRef(new Set<number>());
  const queueRef = useRef<Promise<void>>(Promise.resolve());
  const renderKey = `${filePath}\0${pdfRevision}\0${showHiddenLayers ? 1 : 0}`;
  const renderKeyRef = useRef(renderKey);
  const generationRef = useRef(0);
  const [, bump] = useState(0);

  const revokeAll = useCallback((nextKey = renderKeyRef.current) => {
    renderKeyRef.current = nextKey;
    generationRef.current += 1;
    for (const entry of cacheRef.current.values()) {
      URL.revokeObjectURL(entry.url);
    }
    cacheRef.current.clear();
    inflightRef.current.clear();
    queueRef.current = Promise.resolve();
    bump((n) => n + 1);
  }, []);

  useEffect(() => {
    if (renderKeyRef.current !== renderKey) revokeAll(renderKey);
  }, [renderKey, revokeAll]);

  const evictIfNeeded = useCallback(() => {
    while (cacheRef.current.size > CACHE_LIMIT) {
      const oldest = cacheRef.current.keys().next().value;
      if (oldest === undefined) break;
      const entry = cacheRef.current.get(oldest);
      if (entry) URL.revokeObjectURL(entry.url);
      cacheRef.current.delete(oldest);
    }
  }, []);

  const requestPage = useCallback(
    (page: number) => {
      if (renderKeyRef.current !== renderKey) revokeAll(renderKey);
      if (!filePath || page < 0) return;
      const existing = cacheRef.current.get(page);
      if (existing && existing.key === renderKey) return;
      if (inflightRef.current.has(page)) return;

      inflightRef.current.add(page);
      const pathAtStart = filePath;
      const generationAtStart = generationRef.current;
      const keyAtStart = renderKey;

      queueRef.current = queueRef.current
        .then(async () => {
          try {
            if (generationAtStart !== generationRef.current) return;
            const cmd = showHiddenLayers ? 'render_pdf_page_all_layers' : 'render_pdf_page';
            const bytes = await invoke<number[]>(cmd, {
              path: pathAtStart,
              pageIndex: page,
              width: PDF_BASE_WIDTH,
              height: PDF_BASE_HEIGHT,
            });
            if (generationAtStart !== generationRef.current) return;
            const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' });
            const url = URL.createObjectURL(blob);
            const prev = cacheRef.current.get(page);
            if (prev) URL.revokeObjectURL(prev.url);
            cacheRef.current.set(page, { url, key: keyAtStart });
            evictIfNeeded();
            bump((n) => n + 1);
          } finally {
            inflightRef.current.delete(page);
          }
        })
        .catch(() => {});
    },
    [evictIfNeeded, filePath, renderKey, revokeAll, showHiddenLayers],
  );

  const getPageUrl = useCallback(
    (page: number): string | null => {
      const entry = cacheRef.current.get(page);
      if (!entry || entry.key !== renderKey) return null;
      return entry.url;
    },
    [renderKey],
  );

  return { requestPage, getPageUrl, revokeAll };
}
