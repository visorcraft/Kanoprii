import { buildAppShellChromeInput } from './buildAppShellChromeInput';
import { buildAppShellPageZoomInput } from './buildAppShellPageZoomInput';
import { buildAppShellSource } from './buildAppShellSource';
import { buildHelpChromeInput } from '../app/buildHelpChromeInput';
import { buildAppShellViewerInput } from '../viewer/buildAppShellViewerInput';
import type { AppPdfActions } from '../app/useAppPdfActions';
import type { AnnotationState } from '../app/useAnnotationDraftState';
import type { DocumentState } from '../app/useAppDocumentState';
import type { RefsState } from '../app/useAppRefs';
import type { PanelsState } from '../app/useDocumentPanelsState';
import type { useDrawingGesture } from '../viewer/useDrawingGesture';
import type { HelpState } from '../app/useHelpChromeState';
import type { AppMenus } from '../menu/types';
import type { AppModalsRuntime } from '../modals/appModalsContext';
import type { BuildAppChromeSourceInput } from './buildAppChromeSource';
import type { TabMenuApi } from './useTabContextMenu';
import type { BuildAppViewerSourceInput } from '../viewer/buildAppViewerSource';
import type { PdfPageSize } from '../app/types';
import type { AppSurface, SettingsFocusSection } from '../app/useAppSurfaceState';
import type { ShortcutBindingsState } from '../app/useShortcutBindingsState';
import type { AppearanceKey } from '../settings/appearancePalettes';
import type { WorkspaceViewMode } from '../app/types';
import type { BirdsEyeWorkspace } from '../app/useBirdsEyeWorkspace';

type DrawingState = ReturnType<typeof useDrawingGesture>;

export type BuildAppShellRenderInputArgs = {
  doc: DocumentState;
  onSelectTab: (id: string) => void;
  onCloseTab: (id: string) => void;
  tabMenuApi: TabMenuApi;
  modal: { pageSizes: PdfPageSize[] };
  panels: PanelsState;
  annotation: AnnotationState;
  drawing: Pick<DrawingState, 'highlightStart' | 'highlightRect' | 'inkDraft' | 'shapeLineEnd' | 'drawing'>;
  help: HelpState;
  refs: Pick<RefsState, 'imgRef'>;
  pdfActions: AppPdfActions;
  windowTitle: string;
  appMenus: AppMenus;
  modeExtras: BuildAppChromeSourceInput['modeExtras'];
  modalCtx: AppModalsRuntime;
  printPages: string[];
  activeSurface: AppSurface;
  settingsFocus: SettingsFocusSection;
  closeSettings: () => void;
  workspace: { workspaceView: WorkspaceViewMode; setWorkspaceView: (mode: WorkspaceViewMode) => void };
  birdsEye: BirdsEyeWorkspace;
  shortcuts: ShortcutBindingsState;
  showToast: (message: string, type?: 'success' | 'error') => void;
  dismissToast: () => void;
  appearance: { appearance: AppearanceKey; setAppearance: (key: AppearanceKey) => void };
  viewer: Pick<
    BuildAppViewerSourceInput,
    | 'thumbnails'
    | 'imageSrc'
    | 'annotations'
    | 'scrollRef'
    | 'handleWheel'
    | 'handleImageLoad'
    | 'handleDragStart'
    | 'handleDragOver'
    | 'handleDrop'
    | 'goToPage'
    | 'continuous'
    | 'openPdf'
    | 'loadPdfBookmarks'
    | 'loadPdfSignatures'
    | 'activeSearchRect'
  > & {
    commitPage: () => void;
    commitZoom: () => void;
    zoomIn: () => void;
    zoomOut: () => void;
    resetZoom: () => void;
  };
};

export function buildAppShellRenderInput(args: BuildAppShellRenderInputArgs) {
  const pageZoom = buildAppShellPageZoomInput({
    doc: args.doc,
    modal: args.modal,
    viewer: args.viewer,
  });

  return buildAppShellSource({
    windowTitle: args.windowTitle,
    toast: args.doc.toast,
    loading: args.doc.loading,
    shortcuts: args.shortcuts,
    showToast: args.showToast,
    dismissToast: args.dismissToast,
    appearance: args.appearance,
    chrome: buildAppShellChromeInput({
      menus: args.appMenus,
      help: buildHelpChromeInput(args.help),
      modeExtras: args.modeExtras,
      tabs: args.doc.tabs,
      activeTabId: args.doc.activeId,
      onSelectTab: args.onSelectTab,
      onCloseTab: args.onCloseTab,
      tabMenuApi: args.tabMenuApi,
      documentChromeVisible: args.activeSurface === 'document',
      workspaceView: args.workspace.workspaceView,
      shortcutBindings: args.shortcuts.bindings,
    }),
    activeSurface: args.activeSurface,
    settingsFocus: args.settingsFocus,
    closeSettings: args.closeSettings,
    viewer: buildAppShellViewerInput({
      document: {
        filePath: args.doc.filePath,
        viewMode: args.doc.viewMode,
        scrollViewMode: args.doc.scrollViewMode,
        pageCount: args.doc.pageCount,
        currentPage: args.doc.currentPage,
        pageSizes: args.modal.pageSizes,
        zoom: args.doc.zoom,
        markdownOcrNotice: args.doc.markdownOcrNotice,
        markdownPath: args.doc.markdownPath,
        markdownText: args.doc.markdownText,
        sourcePath: args.doc.sourcePath,
        sourceText: args.doc.sourceText,
      },
      sidebar: {
        thumbnails: args.viewer.thumbnails,
        currentPage: args.doc.currentPage,
        draggedIndex: args.doc.draggedIndex,
        handleDragStart: args.viewer.handleDragStart,
        handleDragOver: args.viewer.handleDragOver,
        handleDrop: args.viewer.handleDrop,
        goToPage: args.viewer.goToPage,
        showAnnotationsPanel: args.panels.showAnnotationsPanel,
        pdfRevision: args.doc.pdfRevision,
        removeHighlightOnPage: args.pdfActions.removeHighlightOnPage,
        removeTextNoteOnPage: args.pdfActions.removeTextNoteOnPage,
        removeRedactionOnPage: args.pdfActions.removeRedactionOnPage,
        showBookmarksPanel: args.panels.showBookmarksPanel,
        pdfBookmarks: args.panels.pdfBookmarks,
        openAddBookmarkModal: args.pdfActions.openAddBookmarkModal,
        openBookmarkAllModal: args.pdfActions.openBookmarkAllModal,
        handleClearAllBookmarks: args.pdfActions.handleClearAllBookmarks,
        loadPdfBookmarks: args.viewer.loadPdfBookmarks,
        openRenameBookmarkModal: args.pdfActions.openRenameBookmarkModal,
        handleRemoveBookmark: args.pdfActions.handleRemoveBookmark,
        showSignaturesPanel: args.panels.showSignaturesPanel,
        pdfSignatures: args.panels.pdfSignatures,
        signatureVerification: args.panels.signatureVerification,
        loadPdfSignatures: args.viewer.loadPdfSignatures,
        showFormsPanel: args.panels.showFormsPanel,
        showPdfUaPanel: args.panels.showPdfUaPanel,
        showHiddenLayers: args.panels.showHiddenLayers,
        formFields: args.panels.formFields,
        formDrafts: args.panels.formDrafts,
        setFormDrafts: args.panels.setFormDrafts,
        openAddFormFieldModal: args.pdfActions.openAddFormFieldModal,
        applyFormField: args.pdfActions.applyFormField,
      },
      viewer: {
        scrollRef: args.viewer.scrollRef,
        handleWheel: args.viewer.handleWheel,
        openPdf: args.viewer.openPdf,
        openMarkdownSaveAs: args.pdfActions.openMarkdownSaveAs,
        imageSrc: args.viewer.imageSrc,
        imgRef: args.refs.imgRef,
        handleImageLoad: args.viewer.handleImageLoad,
        activeSearchRect: args.viewer.activeSearchRect,
        annotations: args.viewer.annotations,
        pageContainerRef: args.pdfActions.pageContainerRef,
        textRuns: args.pdfActions.textRuns,
        textLayerInteractive: args.pdfActions.textLayerInteractive,
        textEditActiveRun: args.pdfActions.textEditActiveRun,
        textEditActiveLine: args.pdfActions.textEditActiveLine,
        textEditDraft: args.pdfActions.textEditDraft,
        onTextEditDraftChange: args.pdfActions.setTextEditDraft,
        onApplyTextEdit: args.pdfActions.applyTextEdit,
        onCancelTextEdit: args.pdfActions.cancelTextEdit,
        continuous: args.viewer.continuous,
      },
      modes: {
        highlightMode: args.annotation.highlightMode,
        noteMode: args.annotation.noteMode,
        drawMode: args.annotation.drawMode,
        shapeMode: args.annotation.shapeMode,
        stampMode: args.annotation.stampMode,
        redactMode: args.annotation.redactMode,
        imageInsertMode: args.annotation.imageInsertMode,
        textEditMode: args.annotation.textEditMode,
        vectorEditMode: args.annotation.vectorEditMode,
        formAddMode: args.annotation.formAddMode,
        shapeKind: args.annotation.shapeKind,
        drawing: args.drawing.drawing,
        highlightStart: args.drawing.highlightStart,
        highlightRect: args.drawing.highlightRect,
        shapeLineEnd: args.drawing.shapeLineEnd,
        inkDraft: args.drawing.inkDraft,
        pageTextEdits: args.annotation.pageTextEdits,
        pageVectorEdits: args.annotation.pageVectorEdits,
      },
      interaction: {
        handlePageClick: args.pdfActions.handlePageClick,
        handleDrawMouseDown: args.pdfActions.handleDrawMouseDown,
        handlePageMouseMove: args.pdfActions.handlePageMouseMove,
        handleDrawMouseUp: args.pdfActions.handleDrawMouseUp,
        removeHighlight: args.pdfActions.removeHighlight,
        removeRedaction: args.pdfActions.removeRedaction,
        removeStamp: args.pdfActions.removeStamp,
        removeShape: args.pdfActions.removeShape,
        removeInkStroke: args.pdfActions.removeInkStroke,
        removeTextNote: args.pdfActions.removeTextNote,
      },
      pageZoom,
      workspaceView: args.workspace.workspaceView,
      birdsEye: args.birdsEye,
    }),
    modalCtx: args.modalCtx,
    printPages: args.printPages,
  });
}
