import { useEffect } from 'react';

type UsePdfRevisionSyncOptions = {
  filePath: string;
  pdfRevision: number;
  loadFormFields: (path: string) => Promise<void>;
  loadPdfSignatures: (path: string) => Promise<void>;
  loadPdfBookmarks: (path: string) => Promise<void>;
  loadPageSizes: (path: string) => Promise<void>;
};

export function usePdfRevisionSync(opts: UsePdfRevisionSyncOptions) {
  useEffect(() => {
    if (opts.filePath) void opts.loadFormFields(opts.filePath);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.filePath, opts.pdfRevision, opts.loadFormFields]);

  useEffect(() => {
    if (opts.filePath) void opts.loadPdfSignatures(opts.filePath);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.filePath, opts.pdfRevision, opts.loadPdfSignatures]);

  useEffect(() => {
    if (opts.filePath) void opts.loadPdfBookmarks(opts.filePath);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.filePath, opts.pdfRevision, opts.loadPdfBookmarks]);

  useEffect(() => {
    if (opts.filePath) void opts.loadPageSizes(opts.filePath);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [opts.filePath, opts.pdfRevision, opts.loadPageSizes]);
}
