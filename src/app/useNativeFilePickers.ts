import { invoke } from '@tauri-apps/api/core';
import { open as openNativeDialog } from '@tauri-apps/plugin-dialog';
import { useCallback } from 'react';
import {
  BMP_DIALOG_FILTER,
  CERT_DIALOG_FILTER,
  GIF_DIALOG_FILTER,
  JPEG_DIALOG_FILTER,
  PDF_DIALOG_FILTER,
  PNG_DIALOG_FILTER,
  PPM_DIALOG_FILTER,
  TIFF_DIALOG_FILTER,
  WEBP_DIALOG_FILTER,
} from './constants';
import { ensureExtension, pickPdfWithNativeDialog, pickSaveWithNativeDialog } from './utils';
import type { ImageExportFormat } from '../pdf/imageExportCommands';
import { imageExportExtension } from '../pdf/imageExportCommands';
import type { PngExportScope } from './types';

type UseNativeFilePickersOptions = {
  filePath: string;
  originalPath: string;
  openFilePath: string;
  insertFilePath: string;
  mergeFilePath: string;
  saveAsPath: string;
  extractOutputPath: string;
  pngExportOutputPath: string;
  signCertPath: string;
  lastBrowserDir: string;
  imageExportFormat: ImageExportFormat;
  pngExportScope: PngExportScope;
  pngExportStartPage: number;
  pngExportEndPage: number;
  extractStartPage: number;
  extractEndPage: number;
  currentPage: number;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  rememberOpenedPdf: (path: string) => void;
  rememberBrowserDirectory: (path: string) => void;
  markSaved: () => void;
  defaultExtractOutputPath: (start: number, end: number) => string;
  defaultImageExportOutput: (format: ImageExportFormat, scope: PngExportScope, start: number, end: number) => string;
  setOpenFilePath: (path: string) => void;
  setInsertFilePath: (path: string) => void;
  setMergeFilePath: (path: string) => void;
  setSaveAsPath: (path: string) => void;
  setShowSaveAsModal: (open: boolean) => void;
  setOriginalPath: (path: string) => void;
  setExtractOutputPath: (path: string) => void;
  setPngExportOutputPath: (path: string) => void;
  setSignCertPath: (path: string) => void;
  showToast: (msg: string, kind?: 'error') => void;
};

export function useNativeFilePickers(opts: UseNativeFilePickersOptions) {
  const chooseOpenPdfNative = useCallback(async () => {
    const path = await pickPdfWithNativeDialog(opts.openFilePath || opts.lastBrowserDir || opts.originalPath);
    if (!path) return false;
    opts.setOpenFilePath(path);
    opts.rememberBrowserDirectory(path);
    return true;
  }, [opts]);

  const chooseInsertPdfNative = useCallback(async () => {
    const path = await pickPdfWithNativeDialog(opts.insertFilePath || opts.lastBrowserDir || opts.originalPath);
    if (!path) return;
    opts.setInsertFilePath(path);
    opts.rememberBrowserDirectory(path);
  }, [opts]);

  const chooseMergePdfNative = useCallback(async () => {
    const path = await pickPdfWithNativeDialog(opts.mergeFilePath || opts.lastBrowserDir || opts.originalPath);
    if (!path) return;
    opts.setMergeFilePath(path);
    opts.rememberBrowserDirectory(path);
  }, [opts]);

  const saveAsViaNativeDialog = useCallback(async () => {
    if (!opts.filePath || !opts.originalPath) return false;
    const picked = await pickSaveWithNativeDialog(opts.saveAsPath || opts.originalPath, PDF_DIALOG_FILTER);
    if (!picked) return false;
    const target = ensureExtension(picked, 'pdf');
    let saved = false;
    await opts.withLoading(async () => {
      await invoke('save_working_copy', { working: opts.filePath, target });
      opts.setOriginalPath(target);
      opts.rememberOpenedPdf(target);
      opts.markSaved();
      opts.setShowSaveAsModal(false);
      opts.showToast(`Saved to ${target}`);
      saved = true;
    });
    return saved;
  }, [opts]);

  const chooseSaveAsNative = useCallback(async () => {
    const picked = await pickSaveWithNativeDialog(opts.saveAsPath || opts.originalPath, PDF_DIALOG_FILTER);
    if (!picked) return;
    opts.setSaveAsPath(ensureExtension(picked, 'pdf'));
  }, [opts]);

  const chooseExtractOutputNative = useCallback(async () => {
    const picked = await pickSaveWithNativeDialog(
      opts.extractOutputPath || opts.defaultExtractOutputPath(opts.extractStartPage, opts.extractEndPage),
      PDF_DIALOG_FILTER,
    );
    if (!picked) return;
    opts.setExtractOutputPath(ensureExtension(picked, 'pdf'));
  }, [opts]);

  const chooseExportPngOutputNative = useCallback(async () => {
    const ext = imageExportExtension(opts.imageExportFormat);
    const filters = opts.imageExportFormat === 'jpeg'
      ? JPEG_DIALOG_FILTER
      : opts.imageExportFormat === 'webp'
        ? WEBP_DIALOG_FILTER
        : opts.imageExportFormat === 'bmp'
          ? BMP_DIALOG_FILTER
          : opts.imageExportFormat === 'tiff'
            ? TIFF_DIALOG_FILTER
            : opts.imageExportFormat === 'gif'
              ? GIF_DIALOG_FILTER
              : opts.imageExportFormat === 'ppm'
                ? PPM_DIALOG_FILTER
                : PNG_DIALOG_FILTER;
    if (opts.pngExportScope === 'current') {
      const picked = await pickSaveWithNativeDialog(
        ensureExtension(
          opts.pngExportOutputPath || opts.defaultImageExportOutput(opts.imageExportFormat, 'current', opts.currentPage, opts.currentPage),
          ext,
        ),
        filters,
      );
      if (!picked) return;
      opts.setPngExportOutputPath(ensureExtension(picked, ext));
      return;
    }
    const picked = await pickSaveWithNativeDialog(
      opts.pngExportOutputPath || opts.defaultImageExportOutput(opts.imageExportFormat, opts.pngExportScope, opts.pngExportStartPage, opts.pngExportEndPage),
      filters,
    );
    if (!picked) return;
    opts.setPngExportOutputPath(picked.replace(/\.(png|jpe?g|webp|bmp)$/i, ''));
  }, [opts]);

  const chooseSignCertNative = useCallback(async () => {
    const selected = await openNativeDialog({
      multiple: false,
      directory: false,
      filters: CERT_DIALOG_FILTER,
    });
    if (selected === null) return;
    const path = typeof selected === 'string' ? selected : selected[0] ?? '';
    if (path) opts.setSignCertPath(path);
  }, [opts]);

  return {
    chooseOpenPdfNative,
    chooseInsertPdfNative,
    chooseMergePdfNative,
    saveAsViaNativeDialog,
    chooseSaveAsNative,
    chooseExtractOutputNative,
    chooseExportPngOutputNative,
    chooseSignCertNative,
  };
}
