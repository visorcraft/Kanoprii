import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';
import type { PdfBrowserEntry, PdfBrowserListing } from '../modals/PdfBrowserModal';
import type { PdfBrowserTarget } from '../app/types';
import type { PageRangePairController } from '../pageRange/usePageRange';
import { directoryFromPath } from '../app/utils';

type UsePdfBrowserOptions = {
  lastBrowserDir: string;
  originalPath: string;
  openFilePath: string;
  insertFilePath: string;
  replaceSourcePath: string;
  interleaveFilePath: string;
  prependFilePath: string;
  mergeFilePath: string;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  rememberBrowserDirectory: (path: string) => void;
  interleaveRange: PageRangePairController;
  prependRange: PageRangePairController;
  setOpenFilePath: (path: string) => void;
  setInsertFilePath: (path: string) => void;
  setReplaceSourcePath: (path: string) => void;
  setReplaceSourcePageCount: (count: number | null) => void;
  setReplaceSourcePage: (page: number) => void;
  setInterleaveFilePath: (path: string) => void;
  setInterleaveSourcePageCount: (count: number | null) => void;
  setPrependFilePath: (path: string) => void;
  setPrependSourcePageCount: (count: number | null) => void;
  setMergeFilePath: (path: string) => void;
};

export function usePdfBrowser({
  lastBrowserDir,
  originalPath,
  openFilePath,
  insertFilePath,
  replaceSourcePath,
  interleaveFilePath,
  prependFilePath,
  mergeFilePath,
  withLoading,
  rememberBrowserDirectory,
  interleaveRange,
  prependRange,
  setOpenFilePath,
  setInsertFilePath,
  setReplaceSourcePath,
  setReplaceSourcePageCount,
  setReplaceSourcePage,
  setInterleaveFilePath,
  setInterleaveSourcePageCount,
  setPrependFilePath,
  setPrependSourcePageCount,
  setMergeFilePath,
}: UsePdfBrowserOptions) {
  const [showBrowserModal, setShowBrowserModal] = useState(false);
  const [browserTarget, setBrowserTarget] = useState<PdfBrowserTarget>('open');
  const [browserListing, setBrowserListing] = useState<PdfBrowserListing | null>(null);
  const [browserPathInput, setBrowserPathInput] = useState('');

  const loadPdfBrowser = async (path?: string) => {
    await withLoading(async () => {
      const listing = await invoke<PdfBrowserListing>('list_pdf_browser_entries', {
        path: path && path.trim() ? path.trim() : null,
      });
      setBrowserListing(listing);
      setBrowserPathInput(listing.currentDir);
    });
  };

  const openPdfBrowser = (target: PdfBrowserTarget) => {
    setBrowserTarget(target);
    setShowBrowserModal(true);
    const sourcePath = target === 'insert'
      ? insertFilePath
      : target === 'replace'
        ? replaceSourcePath
        : target === 'interleave'
          ? interleaveFilePath
          : target === 'prepend'
            ? prependFilePath
            : mergeFilePath;
    const startPath = target === 'open'
      ? lastBrowserDir || directoryFromPath(openFilePath) || directoryFromPath(originalPath)
      : directoryFromPath(sourcePath) || lastBrowserDir || directoryFromPath(originalPath);
    void loadPdfBrowser(startPath);
  };

  const commitBrowserPath = () => {
    void loadPdfBrowser(browserPathInput);
  };

  const handleBrowserEntryClick = async (entry: PdfBrowserEntry) => {
    if (entry.isDir) {
      await loadPdfBrowser(entry.path);
      return;
    }

    if (browserTarget === 'open') {
      setOpenFilePath(entry.path);
      rememberBrowserDirectory(entry.path);
    } else if (browserTarget === 'insert') {
      setInsertFilePath(entry.path);
      rememberBrowserDirectory(entry.path);
    } else if (browserTarget === 'replace') {
      setReplaceSourcePath(entry.path);
      rememberBrowserDirectory(entry.path);
      void invoke<number>('get_pdf_page_count', { path: entry.path }).then((count) => {
        setReplaceSourcePageCount(count);
        setReplaceSourcePage(0);
      });
    } else if (browserTarget === 'interleave') {
      setInterleaveFilePath(entry.path);
      rememberBrowserDirectory(entry.path);
      void invoke<number>('get_pdf_page_count', { path: entry.path }).then((count) => {
        setInterleaveSourcePageCount(count);
        interleaveRange.reset(0, Math.max(0, count - 1));
      });
    } else if (browserTarget === 'prepend') {
      setPrependFilePath(entry.path);
      rememberBrowserDirectory(entry.path);
      void invoke<number>('get_pdf_page_count', { path: entry.path }).then((count) => {
        setPrependSourcePageCount(count);
        prependRange.reset(0, Math.max(0, count - 1));
      });
    } else {
      setMergeFilePath(entry.path);
      rememberBrowserDirectory(entry.path);
    }
    setShowBrowserModal(false);
  };

  return {
    showBrowserModal,
    setShowBrowserModal,
    browserTarget,
    browserListing,
    browserPathInput,
    setBrowserPathInput,
    loadPdfBrowser,
    openPdfBrowser,
    commitBrowserPath,
    handleBrowserEntryClick,
  };
}
