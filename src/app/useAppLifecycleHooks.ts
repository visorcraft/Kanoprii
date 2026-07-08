import { useAppLifecycleLoaders } from './useAppLifecycleLoaders';
import { useAppLifecycleBrowserSearch } from './useAppLifecycleBrowserSearch';
import { useAppLifecycleOpen } from './useAppLifecycleOpen';
import type { UseAppLifecycleHooksInput } from './appLifecycleTypes';

export function useAppLifecycleHooks(input: UseAppLifecycleHooksInput) {
  const loaders = useAppLifecycleLoaders({
    doc: input.doc,
    modal: input.modal,
    panels: input.panels,
    annotation: input.annotation,
    refs: input.refs,
    help: {
      tesseractReminderSource: input.tesseractReminderSource,
      setTesseractReminderSource: input.setTesseractReminderSource,
      tesseractDoNotRemind: input.tesseractDoNotRemind,
      setTesseractDoNotRemind: input.setTesseractDoNotRemind,
      setShowTesseractModal: input.setShowTesseractModal,
    },
    ocrAvailable: input.ocrAvailable,
  });

  const open = useAppLifecycleOpen({ input, loaders });
  const { browser, search, printPages, handlePrint, openPrintDialog, closePdf } = useAppLifecycleBrowserSearch({ input, loaders, open });

  return {
    showToast: input.showToast,
    withLoading: input.withLoading,
    loadFormFields: loaders.loadFormFields,
    loadPdfBookmarks: loaders.loadPdfBookmarks,
    loadPdfSignatures: loaders.loadPdfSignatures,
    loadPageSizes: loaders.loadPageSizes,
    shouldShowTesseractReminder: loaders.shouldShowTesseractReminder,
    closeTesseractReminderModal: loaders.closeTesseractReminderModal,
    showLaunchTesseractReminder: loaders.showLaunchTesseractReminder,
    openTesseractGuide: loaders.openTesseractGuide,
    rememberBrowserDirectory: loaders.rememberBrowserDirectory,
    showUnsavedModal: loaders.showUnsavedModal,
    resolveUnsaved: loaders.resolveUnsaved,
    guardUnsaved: loaders.guardUnsaved,
    ...open,
    browser,
    search,
    printPages,
    handlePrint,
    openPrintDialog,
    closePdf,
  };
}
