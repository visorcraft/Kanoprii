import type { BuildAppViewerSourceInput } from './buildAppViewerSource';
import type { PdfEditState } from '../app/usePdfEditState';

export type BuildAppShellViewerInputArgs = {
  document: Pick<
    BuildAppViewerSourceInput,
    | 'filePath'
    | 'viewMode'
    | 'scrollViewMode'
    | 'pageCount'
    | 'currentPage'
    | 'pageSizes'
    | 'zoom'
    | 'markdownOcrNotice'
    | 'markdownPath'
    | 'markdownText'
    | 'sourcePath'
    | 'sourceText'
  >;
  sidebar: Pick<
    BuildAppViewerSourceInput,
    | 'thumbnails'
    | 'currentPage'
    | 'draggedIndex'
    | 'handleDragStart'
    | 'handleDragOver'
    | 'handleDrop'
    | 'goToPage'
    | 'showAnnotationsPanel'
    | 'pdfRevision'
    | 'removeHighlightOnPage'
    | 'removeTextNoteOnPage'
    | 'removeRedactionOnPage'
    | 'showBookmarksPanel'
    | 'pdfBookmarks'
    | 'openAddBookmarkModal'
    | 'openBookmarkAllModal'
    | 'handleClearAllBookmarks'
    | 'loadPdfBookmarks'
    | 'openRenameBookmarkModal'
    | 'handleRemoveBookmark'
    | 'showSignaturesPanel'
    | 'pdfSignatures'
    | 'signatureVerification'
    | 'loadPdfSignatures'
    | 'showFormsPanel'
    | 'showPdfUaPanel'
    | 'showHiddenLayers'
    | 'formFields'
    | 'formDrafts'
    | 'setFormDrafts'
    | 'openAddFormFieldModal'
    | 'applyFormField'
  >;
  viewer: Pick<
    BuildAppViewerSourceInput,
    | 'scrollRef'
    | 'handleWheel'
    | 'openPdf'
    | 'openMarkdownSaveAs'
    | 'imageSrc'
    | 'imgRef'
    | 'handleImageLoad'
    | 'activeSearchRect'
    | 'annotations'
    | 'pageContainerRef'
    | 'textRuns'
    | 'textLayerInteractive'
    | 'textEditActiveRun'
    | 'textEditActiveLine'
    | 'textEditDraft'
    | 'onTextEditDraftChange'
    | 'onApplyTextEdit'
    | 'onCancelTextEdit'
    | 'continuous'
  > & {
    pdfEdit: PdfEditState;
  };
  modes: Pick<
    BuildAppViewerSourceInput,
    | 'highlightMode'
    | 'noteMode'
    | 'drawMode'
    | 'shapeMode'
    | 'stampMode'
    | 'redactMode'
    | 'imageInsertMode'
    | 'textEditMode'
    | 'editTextRunMode'
    | 'vectorEditMode'
    | 'formAddMode'
    | 'shapeKind'
    | 'drawing'
    | 'highlightStart'
    | 'highlightRect'
    | 'shapeLineEnd'
    | 'inkDraft'
    | 'pageTextEdits'
    | 'pageVectorEdits'
  >;
  interaction: Pick<
    BuildAppViewerSourceInput,
    | 'handlePageClick'
    | 'handleDrawMouseDown'
    | 'handlePageMouseMove'
    | 'handleDrawMouseUp'
    | 'removeHighlight'
    | 'removeRedaction'
    | 'removeStamp'
    | 'removeShape'
    | 'removeInkStroke'
    | 'removeTextNote'
  >;
  pageZoom: Pick<
    BuildAppViewerSourceInput,
    | 'pageInput'
    | 'setPageInput'
    | 'commitPage'
    | 'goToPage'
    | 'zoomInput'
    | 'setZoomInput'
    | 'commitZoom'
    | 'zoomIn'
    | 'zoomOut'
    | 'resetZoom'
  >;
  workspaceView: BuildAppViewerSourceInput['workspaceView'];
  birdsEye: BuildAppViewerSourceInput['birdsEye'];
};

export function buildAppShellViewerInput(
  args: BuildAppShellViewerInputArgs
): BuildAppViewerSourceInput {
  return {
    ...args.document,
    ...args.sidebar,
    ...args.viewer,
    ...args.modes,
    ...args.interaction,
    ...args.pageZoom,
    workspaceView: args.workspaceView,
    birdsEye: args.birdsEye,
  };
}
