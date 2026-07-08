import { buildAppModalCtxInput } from '../modals/buildAppModalCtxInput';
import type { AppPdfActions } from './useAppPdfActions';
import type { AnnotationState } from './useAnnotationDraftState';
import type { ModalState } from './useAppModalState';
import type { PageRangesState } from './useAppPageRanges';
import type { HelpState } from './useHelpChromeState';
import type { SecurityState } from './useSecurityFormState';
import type { DocumentSessionData } from './documentSessionTypes';
import type { useAppLifecycleSlices } from './useAppLifecycleSlices';
import type { WorkspaceViewMode } from './types';

type Slices = ReturnType<typeof useAppLifecycleSlices>;

export type UseAppModalCtxBindingInput = {
  modal: ModalState;
  security: SecurityState;
  annotation: AnnotationState;
  pageRanges: PageRangesState;
  help: HelpState;
  doc: { activeSession: DocumentSessionData | null; currentPage: number; pageCount: number | null; ocrAvailable: boolean | null };
  workspace: { workspaceView: WorkspaceViewMode; setWorkspaceView: (mode: WorkspaceViewMode) => void };
  slices: Slices;
  pdfActions: AppPdfActions;
  showToast: (message: string, type?: 'success' | 'error') => void;
};

export function useAppModalCtxBinding(input: UseAppModalCtxBindingInput) {
  const { slices } = input;
  const { browser, search, unsaved, tesseract, open } = slices;

  return buildAppModalCtxInput({
    modal: input.modal,
    security: input.security,
    annotation: input.annotation,
    pageRanges: input.pageRanges,
    doc: input.doc,
    browser: {
      showBrowserModal: browser.showBrowserModal,
      setShowBrowserModal: browser.setShowBrowserModal,
      browserListing: browser.browserListing,
      browserPathInput: browser.browserPathInput,
      setBrowserPathInput: browser.setBrowserPathInput,
      loadPdfBrowser: browser.loadPdfBrowser,
      openPdfBrowser: browser.openPdfBrowser,
      commitBrowserPath: browser.commitBrowserPath,
      handleBrowserEntryClick: browser.handleBrowserEntryClick,
    },
    search: {
      showSearchModal: search.showSearchModal,
      searchQuery: search.searchQuery,
      setSearchQuery: search.setSearchQuery,
      searchMatchCase: search.searchMatchCase,
      setSearchMatchCase: search.setSearchMatchCase,
      searchWholeWord: search.searchWholeWord,
      setSearchWholeWord: search.setSearchWholeWord,
      searchResults: search.searchResults,
      searchResultIndex: search.searchResultIndex,
      searchInputRef: search.searchInputRef,
      closeSearchModal: search.closeSearchModal,
      runPdfSearch: search.runPdfSearch,
      stepSearchMatch: search.stepSearchMatch,
    },
    unsaved,
    tesseract,
    help: input.help,
    lifecycle: {
      handleOpenPdfPath: open.handleOpenPdfPath,
      handleOpenEncryptedPdf: open.handleOpenEncryptedPdf,
      handleOpenRecentPdf: open.handleOpenRecentPdf,
      loadPdfBrowser: browser.loadPdfBrowser,
      openPdfBrowser: browser.openPdfBrowser,
      handleUseSystemPrint: slices.chrome.handlePrint,
    },
    runtime: { showToast: input.showToast },
    workspace: input.workspace,
    pdfActions: input.pdfActions,
  });
}
