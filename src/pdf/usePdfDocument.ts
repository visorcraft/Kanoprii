import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SessionViewerCache } from '../app/documentSessionTypes';

// Base resolution each page is rendered at. Zoom is applied as a CSS transform
// on top of this so the rendered image and the annotation overlays scale
// together and stay aligned at any zoom level.
export const PDF_BASE_WIDTH = 800;
export const PDF_BASE_HEIGHT = 1132;

export interface PdfAnnotation {
  subtype: string;
  rect: [number, number, number, number];
  color: [number, number, number] | null;
  contents: string | null;
  ink_points: number[] | null;
  line_endpoints: [number, number, number, number] | null;
  stamp_kind: string | null;
  stamp_preset: string | null;
  is_redaction: boolean;
}

type ViewMode = 'pdf' | 'markdown';

export type UsePdfDocumentDeps = {
  filePath: string;
  pageCount: number | null;
  currentPage: number;
  viewMode: ViewMode;
  setPageCount: React.Dispatch<React.SetStateAction<number | null>>;
  setCurrentPage: React.Dispatch<React.SetStateAction<number>>;
  setPageInput: React.Dispatch<React.SetStateAction<string>>;
  setViewMode: (mode: ViewMode) => void;
  setPdfRevision: React.Dispatch<React.SetStateAction<number>>;
  setMarkdownRevision: React.Dispatch<React.SetStateAction<number | null>>;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  loadPageEdits: (path: string, page: number) => Promise<void>;
  loadPdfBookmarks?: (path: string) => void;
  loadPageSizes?: (path: string) => void;
  cancelDrawing: () => void;
  showHiddenLayers: boolean;
  activeSessionId: string | null;
  viewerCache?: SessionViewerCache;
  patchViewerCache: (patch: Partial<SessionViewerCache>) => void;
  patchViewerCacheForPath: (path: string, patch: Partial<SessionViewerCache>) => void;
};

export function usePdfDocument({
  filePath,
  pageCount,
  currentPage,
  viewMode,
  setPageCount,
  setCurrentPage,
  setPageInput,
  setViewMode,
  setPdfRevision,
  setMarkdownRevision,
  withLoading,
  loadPageEdits,
  loadPdfBookmarks,
  loadPageSizes,
  cancelDrawing,
  showHiddenLayers,
  activeSessionId,
  viewerCache,
  patchViewerCache,
  patchViewerCacheForPath,
}: UsePdfDocumentDeps) {
  const [imageSrc, setImageSrc] = useState(viewerCache?.imageSrc ?? '');
  const [thumbnails, setThumbnails] = useState<string[]>(viewerCache?.thumbnails ?? []);
  const [annotations, setAnnotations] = useState<PdfAnnotation[]>(viewerCache?.annotations ?? []);

  // Async renders target the session that owns the rendered path: opening a
  // second document races setActive, so "patch the active session" would write
  // the new document's assets into the previous tab's cache. Local state is
  // only updated while the rendered path is still the displayed document.
  const filePathRef = useRef(filePath);
  useEffect(() => {
    filePathRef.current = filePath;
  }, [filePath]);

  useEffect(() => {
    if (!activeSessionId || !viewerCache) return;
    setImageSrc(viewerCache.imageSrc);
    setThumbnails(viewerCache.thumbnails);
    setAnnotations(viewerCache.annotations);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [activeSessionId, viewerCache?.imageSrc, viewerCache?.thumbnails, viewerCache?.annotations]);

  const syncCache = useCallback(
    (patch: Partial<SessionViewerCache>) => {
      patchViewerCache(patch);
    },
    [patchViewerCache],
  );

  const revokeViewerAssets = useCallback(() => {
    setImageSrc((prev) => {
      if (prev) URL.revokeObjectURL(prev);
      return '';
    });
    setThumbnails((prev) => {
      prev.forEach((url) => URL.revokeObjectURL(url));
      return [];
    });
    setAnnotations([]);
    syncCache({ imageSrc: '', thumbnails: [], annotations: [] });
  }, [syncCache]);

  const loadThumbnails = useCallback(async (path: string) => {
    let thumbBytesArray: number[][];
    if (showHiddenLayers) {
      const count = await invoke<number>('get_pdf_page_count', { path });
      thumbBytesArray = [];
      for (let pageIndex = 0; pageIndex < count; pageIndex += 1) {
        thumbBytesArray.push(await invoke<number[]>('render_pdf_page_all_layers', {
          path,
          pageIndex,
          width: 100,
          height: 141,
        }));
      }
    } else {
      thumbBytesArray = await invoke<number[][]>('get_pdf_thumbnails', {
        path,
        width: 100,
        height: 141,
      });
    }
    const thumbs = thumbBytesArray.map((bytes) => {
      const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' });
      return URL.createObjectURL(blob);
    });
    // The path's session cache owns the URLs (and revokes the replaced ones).
    patchViewerCacheForPath(path, { thumbnails: thumbs });
    if (filePathRef.current === path) setThumbnails(thumbs);
  }, [patchViewerCacheForPath, showHiddenLayers]);

  const renderPage = useCallback(async (path: string, index: number) => {
    const bytes = await invoke<number[]>(showHiddenLayers ? 'render_pdf_page_all_layers' : 'render_pdf_page', {
      path,
      pageIndex: index,
      width: PDF_BASE_WIDTH,
      height: PDF_BASE_HEIGHT,
    });
    const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' });
    const url = URL.createObjectURL(blob);
    patchViewerCacheForPath(path, { imageSrc: url });
    if (filePathRef.current === path) setImageSrc(url);

    const annots = await invoke<PdfAnnotation[]>('get_annotations', { path, pageIndex: index });
    patchViewerCacheForPath(path, { annotations: annots });
    if (filePathRef.current === path) setAnnotations(annots);
    await loadPageEdits(path, index);
  }, [loadPageEdits, patchViewerCacheForPath, showHiddenLayers]);

  const hiddenLayersRef = useRef(showHiddenLayers);
  useEffect(() => {
    if (hiddenLayersRef.current === showHiddenLayers) return;
    hiddenLayersRef.current = showHiddenLayers;
    if (!filePath || pageCount === null) return;
    const page = Math.max(0, Math.min(currentPage, pageCount - 1));
    void withLoading(async () => {
      await renderPage(filePath, page);
      await loadThumbnails(filePath);
    });
  }, [currentPage, filePath, loadThumbnails, pageCount, renderPage, showHiddenLayers, withLoading]);

  const goToPage = useCallback((index: number) => {
    if (pageCount === null || !filePath) return;
    const clamped = Math.max(0, Math.min(index, pageCount - 1));
    setViewMode('pdf');
    setCurrentPage(clamped);
    const render = () => {
      void withLoading(() => renderPage(filePath, clamped));
    };
    if (viewMode === 'markdown') {
      window.requestAnimationFrame(() => window.requestAnimationFrame(render));
      return;
    }
    render();
  }, [filePath, pageCount, renderPage, setCurrentPage, setViewMode, viewMode, withLoading]);

  const reloadOpenPdf = useCallback(async (nextPage = currentPage) => {
    if (!filePath) return;
    const count = await invoke<number>('get_pdf_page_count', { path: filePath });
    const page = Math.max(0, Math.min(nextPage, count - 1));
    setPageCount(count);
    setCurrentPage(page);
    setPageInput(String(page + 1));
    setViewMode('pdf');
    await renderPage(filePath, page);
    await loadThumbnails(filePath);
    loadPdfBookmarks?.(filePath);
    loadPageSizes?.(filePath);
  }, [
    currentPage,
    filePath,
    loadPageSizes,
    loadPdfBookmarks,
    loadThumbnails,
    renderPage,
    setCurrentPage,
    setPageCount,
    setPageInput,
    setViewMode,
  ]);

  const refreshAfterWorkingChange = useCallback(async () => {
    if (!filePath) return;
    const count = await invoke<number>('get_pdf_page_count', { path: filePath });
    setPageCount(count);
    const page = Math.max(0, Math.min(currentPage, count - 1));
    setCurrentPage(page);
    setViewMode('pdf');
    setMarkdownRevision(null);
    setPdfRevision((r) => r + 1);
    cancelDrawing();
    await renderPage(filePath, page);
    await loadThumbnails(filePath);
  }, [
    cancelDrawing,
    currentPage,
    filePath,
    loadThumbnails,
    renderPage,
    setCurrentPage,
    setMarkdownRevision,
    setPageCount,
    setPdfRevision,
    setViewMode,
  ]);

  return {
    imageSrc,
    thumbnails,
    annotations,
    setAnnotations,
    loadThumbnails,
    renderPage,
    goToPage,
    reloadOpenPdf,
    refreshAfterWorkingChange,
    revokeViewerAssets,
  };
}
