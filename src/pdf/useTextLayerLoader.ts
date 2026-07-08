import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useRef, useState } from 'react';

export type PageTextRun = {
  text: string;
  x: number;
  y: number;
  w: number;
  h: number;
};

type CacheKey = string;

const MAX_CACHE_SIZE = 128;

function cacheKey(path: string, page: number, revision: number): CacheKey {
  return `${path}\0${page}\0${revision}`;
}

function trimCache(cache: Map<CacheKey, PageTextRun[]>) {
  while (cache.size > MAX_CACHE_SIZE) {
    const oldest = cache.keys().next().value;
    if (oldest === undefined) break;
    cache.delete(oldest);
  }
}

export function useTextLayerLoader(filePath: string, page: number, pdfRevision: number) {
  const [runs, setRuns] = useState<PageTextRun[]>([]);
  const [loading, setLoading] = useState(false);
  const cacheRef = useRef(new Map<CacheKey, PageTextRun[]>());

  const load = useCallback(async () => {
    if (!filePath) {
      setRuns([]);
      return;
    }
    const key = cacheKey(filePath, page, pdfRevision);
    const cached = cacheRef.current.get(key);
    if (cached) {
      setRuns(cached);
      return;
    }
    setLoading(true);
    try {
      const layout = await invoke<PageTextRun[]>('get_page_text_layout', {
        path: filePath,
        pageIndex: page,
      });
      cacheRef.current.set(key, layout);
      trimCache(cacheRef.current);
      setRuns(layout);
    } catch {
      setRuns([]);
    } finally {
      setLoading(false);
    }
  }, [filePath, page, pdfRevision]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (pdfRevision === 0) return;
    for (const key of [...cacheRef.current.keys()]) {
      if (key.startsWith(`${filePath}\0`) && key.endsWith(`\0${pdfRevision}`)) continue;
      if (key.startsWith(`${filePath}\0`)) cacheRef.current.delete(key);
    }
  }, [filePath, pdfRevision]);

  useEffect(() => {
    cacheRef.current.clear();
  }, [filePath]);

  return { runs, loading, reload: load };
}
