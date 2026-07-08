import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useState } from 'react';

type UsePrintJobsOptions = {
  filePath: string;
  pageCount: number | null;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
};

export function usePrintJobs({ filePath, pageCount, withLoading }: UsePrintJobsOptions) {
  const [printPages, setPrintPages] = useState<string[]>([]);

  const clearPrintPages = useCallback(() => {
    setPrintPages((prev) => {
      prev.forEach((url) => URL.revokeObjectURL(url));
      return [];
    });
  }, []);

  const handlePrint = async () => {
    if (!filePath || pageCount === null) return;
    await withLoading(async () => {
      const urls: string[] = [];
      for (let i = 0; i < pageCount; i++) {
        const bytes = await invoke<number[]>('render_pdf_page', {
          path: filePath, pageIndex: i, width: 1000, height: 1414,
        });
        const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' });
        urls.push(URL.createObjectURL(blob));
      }
      setPrintPages(urls);
    });
  };

  useEffect(() => {
    if (printPages.length === 0) return;
    const timer = setTimeout(() => {
      window.print();
      printPages.forEach((url) => URL.revokeObjectURL(url));
      setPrintPages([]);
    }, 250);
    return () => clearTimeout(timer);
  }, [printPages]);

  return { printPages, handlePrint, clearPrintPages };
}
