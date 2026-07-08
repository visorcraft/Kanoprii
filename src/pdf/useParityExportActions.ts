import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { PageSizePreset } from '../modals/PageSizeModal';
import type { PageRangePairController } from '../pageRange/usePageRange';
import {
  type ImageExportFormat,
  parityImageExportCommand,
} from './imageExportCommands';
import {
  buildParityBatchPayload,
  parityBatchMutatesPdf,
  parityBatchNeedsRange,
} from './parityPayload';

type UseParityExportActionsOptions = {
  filePath: string;
  pageCount: number | null;
  currentPage: number;
  parityRange: PageRangePairController;
  parityRangeCommand: string;
  parityRangeOutputPath: string;
  cropMarginTop: number;
  cropMarginRight: number;
  cropMarginBottom: number;
  cropMarginLeft: number;
  watermarkText: string;
  pageHeaderText: string;
  pageFooterText: string;
  pageBorderInset: number;
  pageSizePreset: PageSizePreset;
  pageNumbersPrefix: string;
  pngExportOutputPath: string;
  imageExportFormat: ImageExportFormat;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  reloadOpenPdf: (page: number) => Promise<void>;
  showToast: (msg: string, kind?: 'error') => void;
  setParityRangeCommand: (command: string) => void;
  setShowParityRangeModal: (open: boolean) => void;
  setShowExportPngModal: (open: boolean) => void;
};

export function useParityExportActions(opts: UseParityExportActionsOptions) {
  const parityBatchContext = useCallback(
    () => ({
      filePath: opts.filePath,
      startPage: opts.parityRange.startPage,
      endPage: opts.parityRange.endPage,
      outputPath: opts.parityRangeOutputPath,
      marginTop: opts.cropMarginTop,
      marginRight: opts.cropMarginRight,
      marginBottom: opts.cropMarginBottom,
      marginLeft: opts.cropMarginLeft,
      watermarkText: opts.watermarkText,
      pageHeaderText: opts.pageHeaderText,
      pageFooterText: opts.pageFooterText,
      pageBorderInset: opts.pageBorderInset,
      pageSizePreset: opts.pageSizePreset,
      pageNumbersPrefix: opts.pageNumbersPrefix,
    }),
    [opts],
  );

  const openParityRangeModal = useCallback(() => {
    if (!opts.filePath || opts.pageCount === null) return;
    opts.parityRange.reset(opts.currentPage, opts.currentPage);
    opts.setParityRangeCommand('rotate_odd_pages_in_range');
    opts.setShowParityRangeModal(true);
  }, [opts]);

  const handleParityRangeAction = useCallback(async () => {
    if (!opts.filePath) return;
    const command = opts.parityRangeCommand;
    if (parityBatchNeedsRange(command)) {
      const range = opts.parityRange.validate();
      if (!range) return;
    }
    if ((command.startsWith('export_') || command.startsWith('extract_')) && !opts.parityRangeOutputPath.trim()) {
      opts.showToast('Output path or directory is required', 'error');
      return;
    }
    const payload = buildParityBatchPayload(command, parityBatchContext());
    if (
      (command.includes('watermark') || command.includes('header') || command.includes('footer'))
      && !payload.text
    ) {
      opts.showToast('Text is required for this action', 'error');
      return;
    }
    await opts.withLoading(async () => {
      const result = await invoke<number | string | string[] | void>(command, payload);
      if (parityBatchMutatesPdf(command)) {
        opts.markPdfEdited();
        await opts.reloadOpenPdf(opts.currentPage);
      }
      opts.setShowParityRangeModal(false);
      if (typeof result === 'number') {
        opts.showToast(`Done - affected ${result} item${result === 1 ? '' : 's'}`);
      } else if (Array.isArray(result)) {
        opts.showToast(`Wrote ${result.length} file${result.length === 1 ? '' : 's'}`);
      } else if (typeof result === 'string') {
        opts.showToast(`Wrote ${result}`);
      } else {
        opts.showToast('Done');
      }
    });
  }, [opts, parityBatchContext]);

  const handleExportOddPagesImage = useCallback(async () => {
    const outputDir = opts.pngExportOutputPath.trim();
    if (!opts.filePath || !outputDir) return;
    await opts.withLoading(async () => {
      const written = await invoke<string[]>(parityImageExportCommand(opts.imageExportFormat, true), {
        path: opts.filePath,
        outputDir,
      });
      opts.setShowExportPngModal(false);
      opts.showToast(`Exported ${written.length} odd page image${written.length === 1 ? '' : 's'} to ${outputDir}`);
    });
  }, [opts]);

  const handleExportEvenPagesImage = useCallback(async () => {
    const outputDir = opts.pngExportOutputPath.trim();
    if (!opts.filePath || !outputDir) return;
    await opts.withLoading(async () => {
      const written = await invoke<string[]>(parityImageExportCommand(opts.imageExportFormat, false), {
        path: opts.filePath,
        outputDir,
      });
      opts.setShowExportPngModal(false);
      opts.showToast(`Exported ${written.length} even page image${written.length === 1 ? '' : 's'} to ${outputDir}`);
    });
  }, [opts]);

  return {
    openParityRangeModal,
    handleParityRangeAction,
    handleExportOddPagesImage,
    handleExportEvenPagesImage,
  };
}
