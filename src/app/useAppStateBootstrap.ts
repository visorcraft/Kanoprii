import { useState } from 'react';
import { useAppDocumentState } from './useAppDocumentState';
import { useAppModalState } from './useAppModalState';
import { useSecurityFormState } from './useSecurityFormState';
import { useDocumentPanelsState } from './useDocumentPanelsState';
import { useAnnotationDraftState } from './useAnnotationDraftState';
import { useHelpChromeState } from './useHelpChromeState';
import { useAppRefs } from './useAppRefs';
import { useDrawingGesture } from '../viewer/useDrawingGesture';
import { useAppLoading } from './useAppLoading';
import { useAppPageRanges } from './useAppPageRanges';
import { buildAppLifecycleInput } from './buildAppLifecycleInput';
import { useAppLifecycleHooks } from './useAppLifecycleHooks';
import { useAppLifecycleSlices } from './useAppLifecycleSlices';
import { useAppSetupHooks } from './useAppSetupHooks';
import { useAppViewerWorkflow } from './useAppViewerWorkflow';
import { useSessionPersistence } from './useSessionPersistence';
import { readSpawnParams } from './spawnWindow';
import { useAppearanceState } from './useAppearanceState';
import { useShortcutBindingsState } from './useShortcutBindingsState';
import { useAppSurfaceState } from './useAppSurfaceState';
import type { WorkspaceViewMode } from './types';

export function useAppStateBootstrap() {
  const doc = useAppDocumentState();
  const modal = useAppModalState();
  const security = useSecurityFormState();
  const panels = useDocumentPanelsState();
  const help = useHelpChromeState();
  const annotation = useAnnotationDraftState();
  const drawingGesture = useDrawingGesture();
  const refs = useAppRefs();
  const { showToast, dismissToast, withLoading } = useAppLoading({ setToast: doc.setToast, setLoading: doc.setLoading });
  const pageRanges = useAppPageRanges({ pageCount: doc.pageCount, currentPage: doc.currentPage, showToast });

  const lifecycle = useAppLifecycleHooks(
    buildAppLifecycleInput({
      doc,
      modal,
      security,
      panels,
      annotation,
      refs,
      pageRanges,
      ocrAvailable: doc.ocrAvailable,
      tesseractReminderSource: help.tesseractReminderSource,
      setTesseractReminderSource: help.setTesseractReminderSource,
      tesseractDoNotRemind: help.tesseractDoNotRemind,
      setTesseractDoNotRemind: help.setTesseractDoNotRemind,
      setShowTesseractModal: help.setShowTesseractModal,
      showToast,
      withLoading,
      cancelDrawing: () => refs.cancelDrawingRef.current(),
    }),
  );

  const slices = useAppLifecycleSlices(lifecycle);
  const { loaders, history, tesseract } = slices;

  const { windowTitle } = useAppSetupHooks({
    doc,
    modal,
    help,
    pageRanges,
    refs: { filePathRef: refs.filePathRef },
    onShowTesseractReminder: tesseract.showLaunchTesseractReminder,
  });

  const [workspaceView, setWorkspaceView] =
    useState<WorkspaceViewMode>('tabs');

  const viewerWorkflow = useAppViewerWorkflow({
    pageCount: doc.pageCount,
    viewMode: doc.viewMode,
    scrollViewMode: doc.scrollViewMode,
    currentPage: doc.currentPage,
    filePath: doc.filePath,
    pdfRevision: doc.pdfRevision,
    pageSizes: modal.pageSizes,
    draggedIndex: doc.draggedIndex,
    zoom: doc.zoom,
    zoomInput: doc.zoomInput,
    pageInput: doc.pageInput,
    setDraggedIndex: doc.setDraggedIndex,
    setCurrentPage: doc.setCurrentPage,
    setZoom: doc.setZoom,
    setZoomInput: doc.setZoomInput,
    setPageInput: doc.setPageInput,
    goToPage: slices.viewer.goToPage,
    withLoading,
    markPdfEdited: history.markPdfEdited,
    loadThumbnails: loaders.loadThumbnails,
    renderPage: loaders.renderPage,
    showHiddenLayers: panels.showHiddenLayers,
    workspaceView,
  });

  const persistence = useSessionPersistence({
    sessions: doc.sessions,
    activeId: doc.activeId,
    workspaceView,
    updateSession: doc.updateSession,
    removeSession: doc.removeSession,
    ensureSessionForOpen: doc.ensureSessionForOpen,
    loadPdfFromPath: lifecycle.loadPdfFromPath,
    setActiveSession: doc.setActiveSession,
    setWorkspaceView,
    showToast,
    isSpawned: readSpawnParams().spawn,
  });

  const appearance = useAppearanceState();
  const shortcutBindings = useShortcutBindingsState();
  const surface = useAppSurfaceState();

  return {
    doc,
    modal,
    security,
    panels,
    help,
    annotation,
    drawingGesture,
    refs,
    pageRanges,
    showToast,
    dismissToast,
    withLoading,
    lifecycle,
    slices,
    windowTitle,
    viewerWorkflow,
    persistence,
    appearance,
    shortcutBindings,
    surface,
    workspace: { workspaceView, setWorkspaceView },
  };
}
