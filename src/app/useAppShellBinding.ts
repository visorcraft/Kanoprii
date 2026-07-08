import { buildAppShellRenderInput } from '../chrome/buildAppShellRenderInput';
import type { AppPdfActions } from './useAppPdfActions';
import type { AnnotationState } from './useAnnotationDraftState';
import type { DocumentState } from './useAppDocumentState';
import type { ModalState } from './useAppModalState';
import type { RefsState } from './useAppRefs';
import type { PanelsState } from './useDocumentPanelsState';
import type { useDrawingGesture } from '../viewer/useDrawingGesture';
import type { HelpState } from './useHelpChromeState';
import type { useAppViewerWorkflow } from './useAppViewerWorkflow';
import type { useAppLifecycleSlices } from './useAppLifecycleSlices';
import type { AppMenus } from '../menu/types';
import type { AppModalsRuntime } from '../modals/appModalsContext';
import type { BuildAppChromeSourceInput } from '../chrome/buildAppChromeSource';
import type { AppSurface, SettingsFocusSection } from './useAppSurfaceState';
import type { ShortcutBindingsState } from './useShortcutBindingsState';
import type { AppearanceKey } from '../settings/appearancePalettes';
import type { WorkspaceViewMode } from './types';
import type { BirdsEyeWorkspace } from './useBirdsEyeWorkspace';

type DrawingState = ReturnType<typeof useDrawingGesture>;
type ViewerWorkflow = ReturnType<typeof useAppViewerWorkflow>;
type Slices = ReturnType<typeof useAppLifecycleSlices>;

export type UseAppShellBindingInput = {
  doc: DocumentState;
  onSelectTab: (id: string) => void;
  onCloseTab: (id: string) => void;
  tabMenuApi: import('../chrome/useTabContextMenu').TabMenuApi;
  modal: ModalState;
  panels: PanelsState;
  annotation: AnnotationState;
  drawing: DrawingState;
  help: HelpState;
  refs: Pick<RefsState, 'imgRef'>;
  pdfActions: AppPdfActions;
  windowTitle: string;
  appMenus: AppMenus;
  modeToolbarExtras: BuildAppChromeSourceInput['modeExtras'];
  modalCtx: AppModalsRuntime;
  slices: Slices;
  viewerWorkflow: ViewerWorkflow;
  surface: { activeSurface: AppSurface; settingsFocus: SettingsFocusSection; closeSettings: () => void };
  workspace: { workspaceView: WorkspaceViewMode; setWorkspaceView: (mode: WorkspaceViewMode) => void };
  birdsEye: BirdsEyeWorkspace;
  shortcuts: ShortcutBindingsState;
  showToast: (message: string, type?: 'success' | 'error') => void;
  dismissToast: () => void;
  appearance: { appearance: AppearanceKey; setAppearance: (key: AppearanceKey) => void };
};

export function useAppShellBinding(input: UseAppShellBindingInput) {
  const { viewer, search } = input.slices;
  const { viewerWorkflow } = input;

  return buildAppShellRenderInput({
    doc: input.doc,
    onSelectTab: input.onSelectTab,
    onCloseTab: input.onCloseTab,
    tabMenuApi: input.tabMenuApi,
    modal: input.modal,
    panels: input.panels,
    annotation: input.annotation,
    drawing: input.drawing,
    help: input.help,
    refs: input.refs,
    pdfActions: input.pdfActions,
    windowTitle: input.windowTitle,
    appMenus: input.appMenus,
    modeExtras: input.modeToolbarExtras,
    modalCtx: input.modalCtx,
    printPages: viewer.printPages,
    activeSurface: input.surface.activeSurface,
    settingsFocus: input.surface.settingsFocus,
    closeSettings: input.surface.closeSettings,
    workspace: input.workspace,
    birdsEye: input.birdsEye,
    shortcuts: input.shortcuts,
    showToast: input.showToast,
    dismissToast: input.dismissToast,
    appearance: input.appearance,
    viewer: {
      thumbnails: viewer.thumbnails,
      imageSrc: viewer.imageSrc,
      annotations: viewer.annotations,
      scrollRef: viewerWorkflow.scrollRef,
      handleWheel: viewerWorkflow.handleWheel,
      handleImageLoad: viewerWorkflow.handleImageLoad,
      handleDragStart: viewerWorkflow.handleDragStart,
      handleDragOver: viewerWorkflow.handleDragOver,
      handleDrop: viewerWorkflow.handleDrop,
      goToPage: viewerWorkflow.goToPage,
      continuous: viewerWorkflow.continuous,
      openPdf: viewer.openPdf,
      loadPdfBookmarks: input.slices.loaders.loadPdfBookmarks,
      loadPdfSignatures: input.slices.loaders.loadPdfSignatures,
      activeSearchRect: search.activeSearchRect,
      commitPage: viewerWorkflow.commitPage,
      commitZoom: viewerWorkflow.commitZoom,
      zoomIn: viewerWorkflow.zoomIn,
      zoomOut: viewerWorkflow.zoomOut,
      resetZoom: viewerWorkflow.resetZoom,
    },
  });
}
