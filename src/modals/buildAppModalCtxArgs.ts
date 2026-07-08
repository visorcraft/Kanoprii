import type { AppPdfActions } from '../app/useAppPdfActions';
import type { ModalState } from '../app/useAppModalState';
import type { SecurityState } from '../app/useSecurityFormState';
import type { AnnotationState } from '../app/useAnnotationDraftState';
import type { DocumentState } from '../app/useAppDocumentState';
import type { PageRangesState } from '../app/useAppPageRanges';
import type { HelpState } from '../app/useHelpChromeState';
import type { PdfBrowserEntry, PdfBrowserListing } from './PdfBrowserModal';
import type { PdfTextSearchMatch } from './SearchModal';
import type { PdfBrowserTarget } from '../app/types';
import type { WorkspaceViewMode } from '../app/types';

export type BrowserSlice = {
  showBrowserModal: boolean;
  setShowBrowserModal: (show: boolean) => void;
  browserListing: PdfBrowserListing | null;
  browserPathInput: string;
  setBrowserPathInput: (v: string) => void;
  loadPdfBrowser: (path?: string) => void | Promise<void>;
  openPdfBrowser: (target: PdfBrowserTarget) => void;
  commitBrowserPath: () => void;
  handleBrowserEntryClick: (entry: PdfBrowserEntry) => Promise<void>;
};

export type SearchSlice = {
  showSearchModal: boolean;
  searchQuery: string;
  setSearchQuery: (q: string) => void;
  searchMatchCase: boolean;
  setSearchMatchCase: (v: boolean) => void;
  searchWholeWord: boolean;
  setSearchWholeWord: (v: boolean) => void;
  searchResults: PdfTextSearchMatch[];
  searchResultIndex: number;
  searchInputRef: React.RefObject<HTMLInputElement | null>;
  closeSearchModal: () => void;
  runPdfSearch: () => void | Promise<void>;
  stepSearchMatch: (dir: 1 | -1) => void;
};

export type UnsavedSlice = {
  showUnsavedModal: boolean;
  resolveUnsaved: (choice: import('../modals/UnsavedChangesModal').UnsavedChoice) => void | Promise<void>;
};

export type TesseractSlice = {
  closeTesseractReminderModal: () => void;
  openTesseractGuide: () => void;
};

export type LifecycleSlice = {
  handleOpenPdfPath: (path?: string) => void | Promise<void>;
  handleOpenEncryptedPdf: () => void | Promise<void>;
  handleOpenRecentPdf: (path: string) => void | Promise<void>;
  loadPdfBrowser: (path?: string) => void | Promise<void>;
  openPdfBrowser: (target: PdfBrowserTarget) => void;
  handleUseSystemPrint: () => void;
};

export type RuntimeSlice = {
  showToast: (message: string, type?: 'success' | 'error') => void;
};

export type BuildAppModalCtxInputArgs = {
  modal: ModalState;
  security: SecurityState;
  annotation: AnnotationState;
  pageRanges: PageRangesState;
  doc: Pick<DocumentState, 'activeSession' | 'currentPage' | 'pageCount' | 'ocrAvailable'>;
  browser: BrowserSlice;
  search: SearchSlice;
  unsaved: UnsavedSlice;
  tesseract: TesseractSlice;
  help: HelpState;
  lifecycle: LifecycleSlice;
  runtime: RuntimeSlice;
  workspace: { workspaceView: WorkspaceViewMode; setWorkspaceView: (mode: WorkspaceViewMode) => void };
  pdfActions: AppPdfActions;
};
