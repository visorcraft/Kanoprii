import { invoke } from '@tauri-apps/api/core';
import { useEffect } from 'react';
import type { PageRangePairController } from '../pageRange/usePageRange';

type UseSourcePdfPageCountsOptions = {
  insertFilePath: string;
  mergeFilePath: string;
  insertRange: PageRangePairController;
  mergeRange: PageRangePairController;
  setInsertSourcePageCount: (count: number | null) => void;
  setMergeSourcePageCount: (count: number | null) => void;
};

export function useSourcePdfPageCounts(opts: UseSourcePdfPageCountsOptions) {
  useEffect(() => {
    if (!opts.insertFilePath) {
      opts.setInsertSourcePageCount(null);
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        const count = await invoke<number>('get_pdf_page_count', { path: opts.insertFilePath });
        if (cancelled) return;
        opts.setInsertSourcePageCount(count);
        opts.insertRange.reset(0, Math.max(0, count - 1));
      } catch {
        if (!cancelled) opts.setInsertSourcePageCount(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.insertFilePath]);

  useEffect(() => {
    if (!opts.mergeFilePath) {
      opts.setMergeSourcePageCount(null);
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        const count = await invoke<number>('get_pdf_page_count', { path: opts.mergeFilePath });
        if (cancelled) return;
        opts.setMergeSourcePageCount(count);
        opts.mergeRange.reset(0, Math.max(0, count - 1));
      } catch {
        if (!cancelled) opts.setMergeSourcePageCount(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.mergeFilePath]);
}
