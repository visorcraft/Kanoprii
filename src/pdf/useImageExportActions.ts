import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { ImageExportFormat } from './imageExportCommands';
import { imageExportCommand, imageExportExtension, imageExportLabel } from './imageExportCommands';
import type { PageRangeController } from '../pageRange/usePageRange';
import type { PngExportScope } from '../app/types';
import { ensureExtension } from '../app/utils';

type UseImageExportActionsOptions = {
  filePath: string;
  originalPath: string;
  currentPage: number;
  pageCount: number | null;
  imageExportFormat: ImageExportFormat;
  pngExportOutputPath: string;
  pngExportRange: PageRangeController;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  showToast: (msg: string, kind?: 'error') => void;
  setPngExportOutputPath: (path: string) => void;
  setShowExportPngModal: (open: boolean) => void;
};

export function useImageExportActions(opts: UseImageExportActionsOptions) {
  const defaultImageExportOutput = useCallback((
    format: ImageExportFormat,
    scope: PngExportScope,
    start: number,
    _end: number,
  ) => {
    const base = (opts.originalPath || opts.filePath).replace(/\.pdf$/i, '');
    const ext = imageExportExtension(format);
    if (scope === 'current') return `${base}_page_${start + 1}.${ext}`;
    return `${base}_pages`;
  }, [opts.filePath, opts.originalPath]);

  const openExportPngModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.pngExportRange.reset({ scope: 'current', start: opts.currentPage, end: opts.currentPage });
    opts.setPngExportOutputPath(defaultImageExportOutput(opts.imageExportFormat, 'current', opts.currentPage, opts.currentPage));
    opts.setShowExportPngModal(true);
  }, [opts, defaultImageExportOutput]);

  const handleExportPng = useCallback(async () => {
    const output = opts.pngExportOutputPath.trim();
    if (!opts.filePath || !output) return;
    const range = opts.pngExportRange.validateAndResolve();
    if (!range) return;
    const { start, end } = range;
    const ext = imageExportExtension(opts.imageExportFormat);
    const label = imageExportLabel(opts.imageExportFormat);
    await opts.withLoading(async () => {
      if (opts.pngExportRange.scope === 'current') {
        const written = await invoke<string>(imageExportCommand(opts.imageExportFormat, false), {
          path: opts.filePath,
          pageIndex: opts.currentPage,
          outputPath: ensureExtension(output, ext),
        });
        opts.showToast(`Exported ${label} to ${written}`);
      } else {
        const written = await invoke<string[]>(imageExportCommand(opts.imageExportFormat, true), {
          path: opts.filePath,
          startPage: start,
          endPage: end,
          outputDir: output,
        });
        opts.showToast(`Exported ${written.length} ${label} file${written.length === 1 ? '' : 's'} to ${output}`);
      }
      opts.setShowExportPngModal(false);
    });
  }, [opts]);

  return { defaultImageExportOutput, openExportPngModal, handleExportPng };
}
