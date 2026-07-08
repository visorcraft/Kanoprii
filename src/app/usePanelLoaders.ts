import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type {
  FormFieldData,
  PdfBookmarkEntry,
  PdfPageSize,
  PdfSignatureInfo,
  PdfSignatureVerificationSummary,
} from './types';

type UsePanelLoadersOptions = {
  filePath: string;
  setFormFields: (fields: FormFieldData[]) => void;
  setFormDrafts: (drafts: Record<string, string>) => void;
  setPdfBookmarks: (bookmarks: PdfBookmarkEntry[]) => void;
  setPdfSignatures: (signatures: PdfSignatureInfo[]) => void;
  setSignatureVerification: (summary: PdfSignatureVerificationSummary | null) => void;
  setPageSizes: (sizes: PdfPageSize[]) => void;
};

export function usePanelLoaders({
  filePath,
  setFormFields,
  setFormDrafts,
  setPdfBookmarks,
  setPdfSignatures,
  setSignatureVerification,
  setPageSizes,
}: UsePanelLoadersOptions) {
  const loadFormFields = useCallback(async (path: string = filePath) => {
    if (!path) {
      setFormFields([]);
      setFormDrafts({});
      return;
    }
    try {
      const fields = await invoke<FormFieldData[]>('get_pdf_form_fields', { path });
      setFormFields(fields);
      const drafts: Record<string, string> = {};
      fields.forEach((field) => {
        if (field.field_type === 'checkbox' || field.field_type === 'radio') {
          drafts[field.name] = field.checked ? 'true' : 'false';
        } else {
          drafts[field.name] = field.value;
        }
      });
      setFormDrafts(drafts);
    } catch {
      setFormFields([]);
      setFormDrafts({});
    }
  }, [filePath, setFormFields, setFormDrafts]);

  const loadPdfBookmarks = useCallback(async (path: string = filePath) => {
    if (!path) {
      setPdfBookmarks([]);
      return;
    }
    try {
      const bookmarks = await invoke<PdfBookmarkEntry[]>('get_pdf_bookmarks', { path });
      setPdfBookmarks(bookmarks);
    } catch {
      setPdfBookmarks([]);
    }
  }, [filePath, setPdfBookmarks]);

  const loadPdfSignatures = useCallback(async (path: string = filePath) => {
    if (!path) {
      setPdfSignatures([]);
      setSignatureVerification(null);
      return;
    }
    try {
      const [listed, verified] = await Promise.all([
        invoke<PdfSignatureInfo[]>('list_pdf_signatures', { path }),
        invoke<PdfSignatureVerificationSummary>('verify_pdf_signatures', { path, trustPemPath: null }),
      ]);
      setPdfSignatures(listed);
      setSignatureVerification(verified);
    } catch {
      setPdfSignatures([]);
      setSignatureVerification(null);
    }
  }, [filePath, setPdfSignatures, setSignatureVerification]);

  const loadPageSizes = useCallback(async (path: string = filePath) => {
    if (!path) {
      setPageSizes([]);
      return;
    }
    try {
      const sizes = await invoke<PdfPageSize[]>('get_pdf_page_sizes', { path });
      setPageSizes(sizes);
    } catch {
      setPageSizes([]);
    }
  }, [filePath, setPageSizes]);

  return { loadFormFields, loadPdfBookmarks, loadPdfSignatures, loadPageSizes };
}
