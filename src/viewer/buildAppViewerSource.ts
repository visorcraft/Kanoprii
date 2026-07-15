import type {
  ComponentProps,
  Dispatch,
  RefObject,
  SetStateAction,
} from 'react';
import type { PdfPageView } from './PdfPageView';
import type { PdfSidebar } from './PdfSidebar';
import type { ViewerMain } from './ViewerMain';
import type { PageControls } from './PageControls';
import type { WorkspaceViewMode } from '../app/types';
import type { BirdsEyeWorkspace } from '../app/useBirdsEyeWorkspace';
import type { PdfEditState } from '../app/usePdfEditState';

type SidebarProps = ComponentProps<typeof PdfSidebar>;
type PdfPageProps = ComponentProps<typeof PdfPageView>;
type ViewerMainProps = ComponentProps<typeof ViewerMain>;
type PageControlsProps = ComponentProps<typeof PageControls>;

type BuildViewerContextInput = {
  filePath: string;
  sidebar: SidebarProps;
  viewer: Omit<ViewerMainProps, 'filePath'> & { pdfPage: PdfPageProps };
  workspaceView: WorkspaceViewMode;
  birdsEye: BirdsEyeWorkspace;
};

export type BuildAppViewerSourceInput = {
  filePath: string;
  thumbnails: SidebarProps['thumbnails'];
  currentPage: SidebarProps['currentPage'];
  draggedIndex: SidebarProps['draggedIndex'];
  handleDragStart: SidebarProps['onDragStart'];
  handleDragOver: SidebarProps['onDragOver'];
  handleDrop: SidebarProps['onDrop'];
  goToPage: SidebarProps['onGoToPage'];
  showAnnotationsPanel: SidebarProps['showAnnotationsPanel'];
  pdfRevision: SidebarProps['pdfRevision'];
  removeHighlightOnPage: SidebarProps['onRemoveHighlightOnPage'];
  removeTextNoteOnPage: SidebarProps['onRemoveTextNoteOnPage'];
  removeRedactionOnPage: SidebarProps['onRemoveRedactionOnPage'];
  showBookmarksPanel: SidebarProps['showBookmarksPanel'];
  pdfBookmarks: SidebarProps['pdfBookmarks'];
  openAddBookmarkModal: SidebarProps['onOpenAddBookmarkModal'];
  openBookmarkAllModal: SidebarProps['onOpenBookmarkAllModal'];
  handleClearAllBookmarks: SidebarProps['onClearAllBookmarks'];
  loadPdfBookmarks: SidebarProps['onReloadBookmarks'];
  openRenameBookmarkModal: SidebarProps['onOpenRenameBookmarkModal'];
  handleRemoveBookmark: SidebarProps['onRemoveBookmark'];
  showSignaturesPanel: SidebarProps['showSignaturesPanel'];
  pdfSignatures: SidebarProps['pdfSignatures'];
  signatureVerification: SidebarProps['signatureVerification'];
  loadPdfSignatures: SidebarProps['onReloadSignatures'];
  showFormsPanel: SidebarProps['showFormsPanel'];
  showPdfUaPanel: SidebarProps['showPdfUaPanel'];
  showHiddenLayers: boolean;
  formFields: SidebarProps['formFields'];
  formDrafts: SidebarProps['formDrafts'];
  setFormDrafts: Dispatch<SetStateAction<Record<string, string>>>;
  openAddFormFieldModal: SidebarProps['onOpenAddFormFieldModal'];
  applyFormField: SidebarProps['onApplyFormField'];
  viewMode: ViewerMainProps['viewMode'];
  scrollViewMode: ViewerMainProps['scrollViewMode'];
  pageCount: ViewerMainProps['pageCount'];
  pageSizes: ViewerMainProps['pageSizes'];
  continuous: Omit<
    NonNullable<ViewerMainProps['continuous']>,
    'pdfPage' | 'pageImageSrc' | 'pageCount' | 'currentPage' | 'pageSizes'
  > | null;
  scrollRef: RefObject<HTMLDivElement | null>;
  handleWheel: ViewerMainProps['onWheel'];
  openPdf: ViewerMainProps['onOpenPdf'];
  markdownOcrNotice: ViewerMainProps['markdownOcrNotice'];
  markdownPath: ViewerMainProps['markdownPath'];
  markdownText: ViewerMainProps['markdownText'];
  sourcePath: ViewerMainProps['sourcePath'];
  sourceText: ViewerMainProps['sourceText'];
  openMarkdownSaveAs: ViewerMainProps['onOpenMarkdownSaveAs'];
  zoom: PdfPageProps['zoom'];
  imageSrc: PdfPageProps['imageSrc'];
  pageContainerRef: PdfPageProps['pageContainerRef'];
  textRuns: PdfPageProps['textRuns'];
  textLayerInteractive: PdfPageProps['textLayerInteractive'];
  textEditActiveRun: PdfPageProps['textEditActiveRun'];
  textEditActiveLine: PdfPageProps['textEditActiveLine'];
  textEditDraft: PdfPageProps['textEditDraft'];
  onTextEditDraftChange: PdfPageProps['onTextEditDraftChange'];
  onApplyTextEdit: PdfPageProps['onApplyTextEdit'];
  onCancelTextEdit: PdfPageProps['onCancelTextEdit'];
  pdfEdit: PdfEditState;
  imgRef: PdfPageProps['imgRef'];
  handleImageLoad: PdfPageProps['onImageLoad'];
  highlightMode: PdfPageProps['highlightMode'];
  noteMode: PdfPageProps['noteMode'];
  drawMode: PdfPageProps['drawMode'];
  shapeMode: PdfPageProps['shapeMode'];
  stampMode: PdfPageProps['stampMode'];
  redactMode: PdfPageProps['redactMode'];
  imageInsertMode: PdfPageProps['imageInsertMode'];
  textEditMode: PdfPageProps['textEditMode'];
  editTextRunMode: PdfPageProps['editTextRunMode'];
  vectorEditMode: PdfPageProps['vectorEditMode'];
  formAddMode: PdfPageProps['formAddMode'];
  handlePageClick: PdfPageProps['onPageClick'];
  handleDrawMouseDown: PdfPageProps['onMouseDown'];
  handlePageMouseMove: PdfPageProps['onMouseMove'];
  handleDrawMouseUp: PdfPageProps['onMouseUp'];
  activeSearchRect: PdfPageProps['activeSearchRect'];
  annotations: PdfPageProps['annotations'];
  shapeKind: PdfPageProps['shapeKind'];
  drawing: PdfPageProps['drawing'];
  highlightStart: PdfPageProps['highlightStart'];
  highlightRect: PdfPageProps['highlightRect'];
  shapeLineEnd: PdfPageProps['shapeLineEnd'];
  inkDraft: PdfPageProps['inkDraft'];
  pageTextEdits: PdfPageProps['pageTextEdits'];
  pageVectorEdits: PdfPageProps['pageVectorEdits'];
  removeHighlight: PdfPageProps['onRemoveHighlight'];
  removeRedaction: PdfPageProps['onRemoveRedaction'];
  removeStamp: PdfPageProps['onRemoveStamp'];
  removeShape: PdfPageProps['onRemoveShape'];
  removeInkStroke: PdfPageProps['onRemoveInkStroke'];
  removeTextNote: PdfPageProps['onRemoveTextNote'];
  pageInput: string;
  setPageInput: (value: string) => void;
  commitPage: () => void;
  zoomInput: string;
  setZoomInput: (value: string) => void;
  commitZoom: () => void;
  zoomIn: () => void;
  zoomOut: () => void;
  resetZoom: () => void;
  workspaceView: WorkspaceViewMode;
  birdsEye: BirdsEyeWorkspace;
};

export function buildAppViewerSource(
  input: BuildAppViewerSourceInput
): BuildViewerContextInput {
  const sidebar: BuildViewerContextInput['sidebar'] = {
    filePath: input.filePath,
    thumbnails: input.thumbnails,
    currentPage: input.currentPage,
    draggedIndex: input.draggedIndex,
    onDragStart: input.handleDragStart,
    onDragOver: input.handleDragOver,
    onDrop: input.handleDrop,
    onGoToPage: input.goToPage,
    showAnnotationsPanel: input.showAnnotationsPanel,
    pdfRevision: input.pdfRevision,
    onRemoveHighlightOnPage: input.removeHighlightOnPage,
    onRemoveTextNoteOnPage: input.removeTextNoteOnPage,
    onRemoveRedactionOnPage: input.removeRedactionOnPage,
    showBookmarksPanel: input.showBookmarksPanel,
    pdfBookmarks: input.pdfBookmarks,
    onOpenAddBookmarkModal: input.openAddBookmarkModal,
    onOpenBookmarkAllModal: input.openBookmarkAllModal,
    onClearAllBookmarks: input.handleClearAllBookmarks,
    onReloadBookmarks: input.loadPdfBookmarks,
    onOpenRenameBookmarkModal: input.openRenameBookmarkModal,
    onRemoveBookmark: input.handleRemoveBookmark,
    showSignaturesPanel: input.showSignaturesPanel,
    pdfSignatures: input.pdfSignatures,
    signatureVerification: input.signatureVerification,
    onReloadSignatures: input.loadPdfSignatures,
    showFormsPanel: input.showFormsPanel,
    showPdfUaPanel: input.showPdfUaPanel,
    showHiddenLayers: input.showHiddenLayers,
    formFields: input.formFields,
    formDrafts: input.formDrafts,
    onFormDraftsChange: input.setFormDrafts,
    onOpenAddFormFieldModal: input.openAddFormFieldModal,
    onApplyFormField: input.applyFormField,
  };

  const pdfPage: BuildViewerContextInput['viewer']['pdfPage'] = {
    zoom: input.zoom,
    imageSrc: input.imageSrc,
    pageContainerRef: input.pageContainerRef,
    textRuns: input.textRuns,
    textLayerInteractive: input.textLayerInteractive,
    textEditActiveRun: input.textEditActiveRun,
    textEditActiveLine: input.textEditActiveLine,
    textEditDraft: input.textEditDraft,
    onTextEditDraftChange: input.onTextEditDraftChange,
    onApplyTextEdit: input.onApplyTextEdit,
    onCancelTextEdit: input.onCancelTextEdit,
    pdfEdit: input.pdfEdit,
    imgRef: input.imgRef,
    onImageLoad: input.handleImageLoad,
    highlightMode: input.highlightMode,
    noteMode: input.noteMode,
    drawMode: input.drawMode,
    shapeMode: input.shapeMode,
    stampMode: input.stampMode,
    redactMode: input.redactMode,
    imageInsertMode: input.imageInsertMode,
    textEditMode: input.textEditMode,
    editTextRunMode: input.editTextRunMode,
    vectorEditMode: input.vectorEditMode,
    formAddMode: input.formAddMode,
    onPageClick: input.handlePageClick,
    onMouseDown: input.handleDrawMouseDown,
    onMouseMove: input.handlePageMouseMove,
    onMouseUp: input.handleDrawMouseUp,
    activeSearchRect: input.activeSearchRect,
    annotations: input.annotations,
    shapeKind: input.shapeKind,
    drawing: input.drawing,
    highlightStart: input.highlightStart,
    highlightRect: input.highlightRect,
    shapeLineEnd: input.shapeLineEnd,
    inkDraft: input.inkDraft,
    pageTextEdits: input.pageTextEdits,
    pageVectorEdits: input.pageVectorEdits,
    showFormsPanel: input.showFormsPanel,
    formFields: input.formFields,
    currentPage: input.currentPage,
    onRemoveHighlight: input.removeHighlight,
    onRemoveRedaction: input.removeRedaction,
    onRemoveStamp: input.removeStamp,
    onRemoveShape: input.removeShape,
    onRemoveInkStroke: input.removeInkStroke,
    onRemoveTextNote: input.removeTextNote,
  };

  const continuous = input.continuous
    ? {
        ...input.continuous,
        pdfPage,
        pageImageSrc: input.imageSrc,
        pageCount: input.pageCount ?? 0,
        currentPage: input.currentPage,
        pageSizes: input.pageSizes,
      }
    : null;

  const showPageControls = input.pageCount !== null && input.viewMode === 'pdf';
  const pageControls: PageControlsProps | null = showPageControls
    ? {
        pageCount: input.pageCount!,
        currentPage: input.currentPage,
        pageInput: input.pageInput,
        pageSizes: input.pageSizes,
        onPageInputChange: input.setPageInput,
        onCommitPage: input.commitPage,
        onGoToPage: input.goToPage,
        zoom: input.zoom,
        zoomInput: input.zoomInput,
        onZoomInputChange: input.setZoomInput,
        onCommitZoom: input.commitZoom,
        onZoomIn: input.zoomIn,
        onZoomOut: input.zoomOut,
        onResetZoom: input.resetZoom,
      }
    : null;

  const viewer: BuildViewerContextInput['viewer'] = {
    viewMode: input.viewMode,
    scrollViewMode: input.scrollViewMode,
    pageCount: input.pageCount,
    currentPage: input.currentPage,
    pageSizes: input.pageSizes,
    continuous,
    scrollRef: input.scrollRef,
    onWheel: input.handleWheel,
    onOpenPdf: input.openPdf,
    markdownOcrNotice: input.markdownOcrNotice,
    markdownPath: input.markdownPath,
    markdownText: input.markdownText,
    sourcePath: input.sourcePath,
    sourceText: input.sourceText,
    onOpenMarkdownSaveAs: input.openMarkdownSaveAs,
    pdfPage,
    pageControls,
  };

  return {
    filePath: input.filePath,
    sidebar,
    viewer,
    workspaceView: input.workspaceView,
    birdsEye: input.birdsEye,
  };
}
