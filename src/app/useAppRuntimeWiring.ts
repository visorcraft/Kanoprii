import { useCallback, useEffect, useMemo, useRef } from 'react';
import { listen, emit } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { readSpawnParams, spawnDocumentWindow } from './spawnWindow';
import { useAppPdfActionsBinding } from './useAppPdfActionsBinding';
import { useAppChromeBindings } from './useAppChromeBindings';
import { useAppModalCtxBinding } from './useAppModalCtxBinding';
import { useAppShellBinding as buildAppShellBinding } from './useAppShellBinding';
import { useDocumentTabActions } from './useDocumentTabActions';
import { isTauriRuntime } from './tauriRuntime';
import { useBirdsEyeWorkspace } from './useBirdsEyeWorkspace';
import type { useAppStateBootstrap } from './useAppStateBootstrap';

type Bootstrap = ReturnType<typeof useAppStateBootstrap>;

export function useAppRuntimeWiring(bootstrap: Bootstrap) {
  const {
    doc,
    modal,
    security,
    panels,
    annotation,
    drawingGesture,
    refs,
    pageRanges,
    help,
    showToast,
    dismissToast,
    withLoading,
    lifecycle,
    slices,
    windowTitle,
    viewerWorkflow,
    persistence,
    appearance,
    shortcutBindings: shortcutBindingsState,
    surface,
    workspace,
  } = bootstrap;

  const { loaders, history, unsaved, browser, search, chrome, tesseract } = slices;

  const tabActions = useDocumentTabActions({
    doc,
    modal,
    security,
    panels,
    annotation,
    setWorkspaceView: workspace.setWorkspaceView,
    cancelDrawing: () => refs.cancelDrawingRef.current(),
    showToast,
    guardUnsavedForSession: (sessionId, action) => {
      const session = doc.sessions.find((s) => s.id === sessionId);
      if (sessionId !== doc.activeId) doc.setActiveSession(sessionId);
      unsaved.guardUnsaved(action, session?.isDirty);
    },
    discardHistory: lifecycle.discardHistory,
    clearModesOnTabSwitch: () => {
      annotation.setHighlightMode(false);
      annotation.setNoteMode(false);
      annotation.setDrawMode(false);
      annotation.setShapeMode(false);
      annotation.setStampMode(false);
      annotation.setRedactMode(false);
      annotation.setImageInsertMode(false);
      annotation.setTextEditMode(false);
      annotation.setEditTextRunMode(false);
      annotation.setVectorEditMode(false);
      annotation.setFormAddMode(false);
      annotation.setShowNoteModal(false);
      annotation.setPendingNotePos(null);
    },
    renderPage: loaders.renderPage,
    loadThumbnails: loaders.loadThumbnails,
    loadFormFields: loaders.loadFormFields,
  });

  const pdfActions = useAppPdfActionsBinding({
    doc,
    modal,
    security,
    panels,
    annotation,
    drawing: drawingGesture,
    pageRanges,
    refs: {
      cancelDrawingRef: refs.cancelDrawingRef,
      handleSaveRef: refs.handleSaveRef,
      handleMarkdownViewRef: refs.handleMarkdownViewRef,
      imgRef: refs.imgRef,
    },
    help,
    runtime: {
      loadFormFields: loaders.loadFormFields,
      loadPageSizes: loaders.loadPageSizes,
      loadPdfBookmarks: loaders.loadPdfBookmarks,
      loadPdfSignatures: loaders.loadPdfSignatures,
      loadThumbnails: loaders.loadThumbnails,
      markPdfEdited: history.markPdfEdited,
      markSaved: history.markSaved,
      reloadOpenPdf: lifecycle.reloadOpenPdf,
      rememberBrowserDirectory: lifecycle.rememberBrowserDirectory,
      rememberOpenedPdf: lifecycle.rememberOpenedPdf,
      renderPage: loaders.renderPage,
      setAnnotations: slices.viewer.setAnnotations,
      shouldShowTesseractReminder: lifecycle.shouldShowTesseractReminder,
      showToast,
      withLoading,
      openTesseractGuide: tesseract.openTesseractGuide,
    },
  });

  const { appMenus, modeToolbarExtras } = useAppChromeBindings({
    doc,
    modal,
    security,
    panels,
    annotation,
    help,
    refs: { keyboardActionsRef: refs.keyboardActionsRef },
    pdfActions,
    history: { ...history, canUndo: doc.canUndo, canRedo: doc.canRedo },
    chrome: {
      guardUnsaved: unsaved.guardUnsaved,
      closePdf: tabActions.requestCloseActiveTab,
      openPdf: slices.viewer.openPdf,
      goToPage: viewerWorkflow.goToPage,
      handlePrint: chrome.handlePrint,
      openPrintDialog: chrome.openPrintDialog,
      openSearchModal: search.openSearchModal,
      openTesseractGuide: tesseract.openTesseractGuide,
    },
    zoom: {
      zoomIn: viewerWorkflow.zoomIn,
      zoomOut: viewerWorkflow.zoomOut,
      resetZoom: viewerWorkflow.resetZoom,
    },
    unsaved,
    browser: { showBrowserModal: browser.showBrowserModal, setShowBrowserModal: browser.setShowBrowserModal },
    search: { showSearchModal: search.showSearchModal, closeSearchModal: search.closeSearchModal },
    shortcutBindings: shortcutBindingsState.bindings,
    surface,
    workspace,
  });

  const modalCtx = useAppModalCtxBinding({
    modal,
    security,
    annotation,
    pageRanges,
    help,
    doc: { activeSession: doc.activeSession, currentPage: doc.currentPage, pageCount: doc.pageCount, ocrAvailable: doc.ocrAvailable },
    workspace,
    slices,
    pdfActions,
    showToast,
  });

  const birdsEye = useBirdsEyeWorkspace({
    sessions: doc.sessions,
    tabs: doc.tabs,
    activeId: doc.activeId,
    nativeDialogs: modal.nativeDialogs,
    lastBrowserDir: modal.lastBrowserDir,
    withLoading,
    updateSession: doc.updateSession,
    setActiveSession: doc.setActiveSession,
    setWorkspaceView: workspace.setWorkspaceView,
    setOpenFilePath: modal.setOpenFilePath,
    setShowOpenModal: modal.setShowOpenModal,
    loadThumbnails: loaders.loadThumbnails,
    renderPage: loaders.renderPage,
    rememberBrowserDirectory: lifecycle.rememberBrowserDirectory,
    showToast,
  });

  const openPathPendingRef = useRef(false);

  // Always point at the latest loader. The listener and the launch-path pull are
  // registered once (empty deps); without this ref they would capture a stale
  // loadPdfFromPath - both calling outdated logic and deduping against stale
  // session state.
  const loadPdfFromPathRef = useRef(lifecycle.loadPdfFromPath);
  loadPdfFromPathRef.current = lifecycle.loadPdfFromPath;

  // Register the 'open-path' listener exactly once. Re-registering on every
  // loadPdfFromPath identity change (which changes on every session change) let
  // listeners leak - the cleanup is a no-op until listen()'s promise resolves -
  // so a single event fired the loader multiple times, opening a new file twice.
  useEffect(() => {
    if (!isTauriRuntime()) return;
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void listen<string[]>('open-path', (event) => {
      openPathPendingRef.current = true;
      for (const path of event.payload) {
        void loadPdfFromPathRef.current(path);
      }
    }).then((fn) => {
      // If the effect was torn down before listen() resolved, unlisten now
      // instead of leaking the registration.
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, []);

  useEffect(() => {
    void (async () => {
      // Spawned document windows ("Move tab → New window") open only their one
      // file, skip session restore (persistence is gated off elsewhere), and ack
      // their load so the source window can close the moved tab.
      const spawnParams = readSpawnParams();
      if (spawnParams.spawn && spawnParams.openPath) {
        const ok = await loadPdfFromPathRef.current(spawnParams.openPath);
        if (ok && isTauriRuntime()) {
          void emit('spawn-loaded', { token: spawnParams.token });
        }
        return;
      }
      // Drain any file paths this process was launched with (file-association /
      // "Open With"). Pulling after mount is race-free, where the old launch-time
      // event could fire before this component registered its 'open-path'
      // listener and be dropped (the bug where the app opened but the PDF didn't).
      let launchPaths: string[] = [];
      if (isTauriRuntime()) {
        try {
          launchPaths = await invoke<string[]>('take_pending_open_paths');
        } catch {
          launchPaths = [];
        }
      }
      if (launchPaths.length > 0) openPathPendingRef.current = true;
      // Restore the previous session first, skipping its active-tab restore when
      // a launch file is pending so that file wins focus.
      if (persistence?.restoreSessions) {
        await persistence.restoreSessions(() => openPathPendingRef.current);
      }
      // Open launch files after restore so an already-restored document is
      // focused (deduped) rather than opened a second time. Use the ref so the
      // dedup sees the just-restored sessions.
      for (const path of launchPaths) {
        void loadPdfFromPathRef.current(path);
      }
      openPathPendingRef.current = false;
    })();
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, []);

  // Move a tab's document into a fresh window. Opening it there is a fresh load
  // of the on-disk file, so route a dirty source through the unsaved guard first
  // (Save keeps edits; the new window then opens the saved file).
  const moveToNewWindow = useCallback(
    (id: string) => {
      const session = doc.sessions.find((s) => s.id === id);
      if (!session?.originalPath) return;
      const run = () =>
        spawnDocumentWindow(session.originalPath, id, {
          finalizeClose: tabActions.finalizeCloseSession,
          showToast,
        });
      if (session.isDirty) {
        if (id !== doc.activeId) doc.setActiveSession(id);
        unsaved.guardUnsaved(run, true);
      } else {
        void run();
      }
    },
    [doc, tabActions.finalizeCloseSession, unsaved, showToast],
  );

  const openProperties = useCallback(
    (filePath: string) => {
      void pdfActions.openMetadataModal(filePath);
    },
    [pdfActions],
  );

  const tabMenuApi = useMemo(
    () => ({
      selectTab: tabActions.selectTab,
      requestCloseTab: tabActions.requestCloseTab,
      finalizeClose: tabActions.finalizeCloseSession,
      moveTabToFirst: doc.moveTabToFirst,
      moveTabToLast: doc.moveTabToLast,
      moveToNewWindow,
      updateSession: doc.updateSession,
      openPrint: chrome.openPrintDialog,
      openProperties,
      showToast,
    }),
    [
      tabActions.selectTab,
      tabActions.requestCloseTab,
      tabActions.finalizeCloseSession,
      doc.moveTabToFirst,
      doc.moveTabToLast,
      moveToNewWindow,
      doc.updateSession,
      chrome.openPrintDialog,
      openProperties,
      showToast,
    ],
  );

  return useMemo(
    () =>
      buildAppShellBinding({
        doc,
        modal,
        panels,
        annotation,
        drawing: drawingGesture,
        help,
        refs: { imgRef: refs.imgRef },
        pdfActions,
        windowTitle,
        appMenus,
        modeToolbarExtras,
        modalCtx,
        slices,
        viewerWorkflow,
        surface,
        workspace,
        birdsEye,
        onSelectTab: tabActions.selectTab,
        onCloseTab: tabActions.requestCloseTab,
        tabMenuApi,
        shortcuts: shortcutBindingsState,
        showToast,
        dismissToast,
        appearance,
      }),
    [
      doc,
      modal,
      panels,
      annotation,
      drawingGesture,
      help,
      refs,
      pdfActions,
      windowTitle,
      appMenus,
      modeToolbarExtras,
      modalCtx,
      slices,
      viewerWorkflow,
      surface,
      workspace,
      birdsEye,
      tabActions.selectTab,
      tabActions.requestCloseTab,
      tabMenuApi,
      shortcutBindingsState,
      showToast,
      dismissToast,
      appearance,
    ],
  );
}
