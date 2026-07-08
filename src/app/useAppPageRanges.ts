import { usePageRange, usePageRangePair } from '../pageRange/usePageRange';

type ToastFn = (message: string, type?: 'success' | 'error') => void;

type UseAppPageRangesOptions = {
  pageCount: number | null;
  currentPage: number;
  showToast: ToastFn;
};

export function useAppPageRanges({
  pageCount,
  currentPage,
  showToast,
}: UseAppPageRangesOptions) {
  const pageNumbersRange = usePageRange({ pageCount, currentPage, showToast });
  const watermarkRange = usePageRange({ pageCount, currentPage, showToast });
  const flattenRange = usePageRange({ pageCount, currentPage, showToast });
  const pageHeaderRange = usePageRange({ pageCount, currentPage, showToast });
  const pageFooterRange = usePageRange({ pageCount, currentPage, showToast });
  const pageSizeRange = usePageRange({ pageCount, currentPage, showToast });
  const pageBorderRange = usePageRange({ pageCount, currentPage, showToast });
  const expandMarginsRange = usePageRange({
    pageCount,
    currentPage,
    showToast,
  });
  const shrinkMarginsRange = usePageRange({
    pageCount,
    currentPage,
    showToast,
  });
  const pngExportRange = usePageRange({
    pageCount,
    currentPage,
    defaultScope: 'current',
    showToast,
  });
  const exportPagesPdfRange = usePageRange({
    pageCount,
    currentPage,
    showToast,
  });
  const duplicateRange = usePageRangePair({ showToast });
  const deleteRange = usePageRangePair({ showToast });
  const extractRange = usePageRangePair({ showToast });
  const interleaveRange = usePageRangePair({ showToast });
  const rotateRange = usePageRange({
    pageCount,
    currentPage,
    defaultScope: 'current',
    showToast,
  });
  const keepRange = usePageRangePair({ showToast });
  const moveRange = usePageRangePair({ showToast });
  const prependRange = usePageRangePair({ showToast });
  const reverseRange = usePageRangePair({ showToast });
  const cropRange = usePageRangePair({ showToast });
  const parityRange = usePageRangePair({ showToast });
  const insertRange = usePageRangePair({ showToast });
  const mergeRange = usePageRangePair({ showToast });
  const batesRange = usePageRangePair({ showToast });

  return {
    pageNumbersRange,
    watermarkRange,
    flattenRange,
    pageHeaderRange,
    pageFooterRange,
    pageSizeRange,
    pageBorderRange,
    expandMarginsRange,
    shrinkMarginsRange,
    pngExportRange,
    exportPagesPdfRange,
    duplicateRange,
    deleteRange,
    extractRange,
    interleaveRange,
    rotateRange,
    keepRange,
    moveRange,
    prependRange,
    reverseRange,
    cropRange,
    parityRange,
    insertRange,
    mergeRange,
    batesRange,
  };
}

/** Canonical alias for this hook's state shape. */
export type PageRangesState = ReturnType<typeof useAppPageRanges>;
