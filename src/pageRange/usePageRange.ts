import { useCallback, useState } from 'react';
import { resolvePageRange } from './resolvePageRange';
import type { PageRangeScope, ResolvedPageRange } from './types';

type ToastFn = (message: string, type?: 'success' | 'error') => void;

export type PageRangeController = {
  scope: PageRangeScope;
  startPage: number;
  endPage: number;
  setScope: (scope: PageRangeScope) => void;
  setStartPage: (page: number) => void;
  setEndPage: (page: number) => void;
  reset: (overrides?: Partial<{ scope: PageRangeScope; start: number; end: number }>) => void;
  resolve: () => ResolvedPageRange;
  validateAndResolve: () => ResolvedPageRange | null;
};

type UsePageRangeOptions = {
  pageCount: number | null;
  currentPage: number;
  defaultScope?: PageRangeScope;
  showToast: ToastFn;
};

export function usePageRange({
  pageCount,
  currentPage,
  defaultScope = 'all',
  showToast,
}: UsePageRangeOptions): PageRangeController {
  const lastPage = Math.max(0, (pageCount ?? 1) - 1);
  const [scope, setScope] = useState<PageRangeScope>(defaultScope);
  const [startPage, setStartPage] = useState(0);
  const [endPage, setEndPage] = useState(lastPage);

  const reset = useCallback((overrides?: Partial<{ scope: PageRangeScope; start: number; end: number }>) => {
    setScope(overrides?.scope ?? defaultScope);
    setStartPage(overrides?.start ?? 0);
    setEndPage(overrides?.end ?? Math.max(0, (pageCount ?? 1) - 1));
  }, [defaultScope, pageCount]);

  const resolve = useCallback(
    () => resolvePageRange(scope, startPage, endPage, currentPage, pageCount),
    [scope, startPage, endPage, currentPage, pageCount],
  );

  const validateAndResolve = useCallback(() => {
    const range = resolve();
    if (scope === 'range' && range.start > range.end) {
      showToast('From page must be ≤ To page', 'error');
      return null;
    }
    return range;
  }, [resolve, scope, showToast]);

  return {
    scope,
    startPage,
    endPage,
    setScope,
    setStartPage,
    setEndPage,
    reset,
    resolve,
    validateAndResolve,
  };
}

export type PageRangePairController = {
  startPage: number;
  endPage: number;
  setStartPage: (page: number) => void;
  setEndPage: (page: number) => void;
  reset: (start?: number, end?: number) => void;
  validate: () => ResolvedPageRange | null;
};

type UsePageRangePairOptions = {
  showToast: ToastFn;
};

export function usePageRangePair({ showToast }: UsePageRangePairOptions): PageRangePairController {
  const [startPage, setStartPage] = useState(0);
  const [endPage, setEndPage] = useState(0);

  const reset = useCallback((start = 0, end = 0) => {
    setStartPage(start);
    setEndPage(end);
  }, []);

  const validate = useCallback(() => {
    if (startPage > endPage) {
      showToast('From page must be ≤ To page', 'error');
      return null;
    }
    return { start: startPage, end: endPage };
  }, [startPage, endPage, showToast]);

  return {
    startPage,
    endPage,
    setStartPage,
    setEndPage,
    reset,
    validate,
  };
}
