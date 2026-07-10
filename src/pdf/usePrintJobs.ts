import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useRef, useState } from 'react';

type UsePrintJobsOptions = {
  filePath: string;
  pageCount: number | null;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
};

export function usePrintJobs({ filePath, pageCount, withLoading }: UsePrintJobsOptions) {
  const [printPages, setPrintPages] = useState<string[]>([]);
  // Mirror of `printPages` kept in a ref so the unmount effect can revoke the
  // last-known URLs even after the state has been cleared.
  const printPagesRef = useRef<string[]>([]);

  const clearPrintPages = useCallback(() => {
    printPagesRef.current.forEach((url) => URL.revokeObjectURL(url));
    printPagesRef.current = [];
    setPrintPages([]);
  }, []);

  const handlePrint = async () => {
    if (!filePath || pageCount === null) return;
    await withLoading(async () => {
      // Pre-clear any URLs left from a prior print so a rapid second handlePrint
      // doesn't strand them (state would otherwise be overwritten).
      clearPrintPages();
      const urls: string[] = [];
      for (let i = 0; i < pageCount; i++) {
        const bytes = await invoke<number[]>('render_pdf_page', {
          path: filePath, pageIndex: i, width: 1000, height: 1414,
        });
        const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' });
        urls.push(URL.createObjectURL(blob));
      }
      printPagesRef.current = urls;
      setPrintPages(urls);
    });
  };

  useEffect(() => {
    if (printPages.length === 0) return;
    const timer = setTimeout(() => {
      window.print();
      printPages.forEach((url) => URL.revokeObjectURL(url));
      printPagesRef.current = [];
      setPrintPages([]);
    }, 250);
    return () => clearTimeout(timer);
  }, [printPages]);

  // Mount-only cleanup: revoke any URLs still owned when the hook unmounts
  // (e.g. the print modal closed mid-preprint, or the doc was switched).
  useEffect(
    () => () => {
      printPagesRef.current.forEach((url) => URL.revokeObjectURL(url));
      printPagesRef.current = [];
    },
    [],
  );

  return { printPages, handlePrint, clearPrintPages };
}