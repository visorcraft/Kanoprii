import { useCallback } from 'react';
import { RECENT_PDF_LIMIT, RECENT_PDFS_KEY } from './constants';
import { writeStoredStringArray } from './utils';

type UsePdfRecentsOptions = {
  rememberBrowserDirectory: (path: string) => void;
  setRecentPdfs: React.Dispatch<React.SetStateAction<string[]>>;
};

export function usePdfRecents({ rememberBrowserDirectory, setRecentPdfs }: UsePdfRecentsOptions) {
  const rememberOpenedPdf = useCallback((path: string) => {
    rememberBrowserDirectory(path);
    setRecentPdfs((prev) => {
      const next = [path, ...prev.filter((item) => item !== path)].slice(0, RECENT_PDF_LIMIT);
      writeStoredStringArray(RECENT_PDFS_KEY, next);
      return next;
    });
  }, [rememberBrowserDirectory, setRecentPdfs]);

  return { rememberOpenedPdf };
}
