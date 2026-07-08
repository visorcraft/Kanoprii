import type { useAppLifecycleHooks } from './useAppLifecycleHooks';

type Lifecycle = ReturnType<typeof useAppLifecycleHooks>;

export function useAppLifecycleSlices(lifecycle: Lifecycle) {
  const {
    loadFormFields,
    loadPdfBookmarks,
    loadPdfSignatures,
    loadPageSizes,
    showLaunchTesseractReminder,
    openTesseractGuide,
    showUnsavedModal,
    resolveUnsaved,
    guardUnsaved,
    imageSrc,
    thumbnails,
    annotations,
    setAnnotations,
    loadThumbnails,
    renderPage,
    goToPage,
    markPdfEdited,
    markSaved,
    undo,
    redo,
    openPdf,
    browser,
    search,
    printPages,
    handlePrint,
    openPrintDialog,
    closePdf,
    handleOpenPdfPath,
    handleOpenEncryptedPdf,
    handleOpenRecentPdf,
  } = lifecycle;

  const {
    showBrowserModal,
    setShowBrowserModal,
    browserListing,
    browserPathInput,
    setBrowserPathInput,
    loadPdfBrowser,
    openPdfBrowser,
    commitBrowserPath,
    handleBrowserEntryClick,
  } = browser;

  return {
    loaders: { loadFormFields, loadPdfBookmarks, loadPdfSignatures, loadPageSizes, loadThumbnails, renderPage },
    tesseract: { showLaunchTesseractReminder, openTesseractGuide, closeTesseractReminderModal: lifecycle.closeTesseractReminderModal },
    unsaved: { showUnsavedModal, resolveUnsaved, guardUnsaved },
    viewer: { imageSrc, thumbnails, annotations, setAnnotations, goToPage, openPdf, printPages },
    history: { undo, redo, markPdfEdited, markSaved },
    open: { handleOpenPdfPath, handleOpenEncryptedPdf, handleOpenRecentPdf },
    browser: {
      showBrowserModal,
      setShowBrowserModal,
      browserListing,
      browserPathInput,
      setBrowserPathInput,
      loadPdfBrowser,
      openPdfBrowser,
      commitBrowserPath,
      handleBrowserEntryClick,
    },
    search,
    chrome: { handlePrint, openPrintDialog, closePdf },
    lifecycle,
  };
}
