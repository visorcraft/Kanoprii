import type { AppPdfActions } from '../app/useAppPdfActions';
import type { DocumentState } from '../app/useAppDocumentState';
import type { AnnotationState } from '../app/useAnnotationDraftState';
import type { PanelsState } from '../app/useDocumentPanelsState';
import type { HelpState } from '../app/useHelpChromeState';
import type { ViewMode, WorkspaceViewMode } from '../app/types';
import type { AppSurface, SettingsFocusSection } from '../app/useAppSurfaceState';
import type { ShortcutBindings } from '../app/useShortcutBindingsState';
import type { PdfEditState } from '../app/usePdfEditState';

export type BuildAppMenuInputArgs = {
  doc: Pick<DocumentState, 'filePath' | 'originalPath' | 'sourcePath' | 'sourceKind' | 'isDirty' | 'pageCount' | 'currentPage' | 'viewMode' | 'scrollViewMode' | 'ocrAvailable'>;
  annotation: Pick<AnnotationState, 'highlightMode' | 'noteMode' | 'drawMode' | 'shapeMode' | 'stampMode' | 'redactMode' | 'imageInsertMode' | 'textEditMode' | 'editTextRunMode' | 'vectorEditMode'>;
  panels: Pick<PanelsState, 'showFormsPanel' | 'showBookmarksPanel' | 'showSignaturesPanel' | 'showAnnotationsPanel' | 'showPdfUaPanel' | 'showHiddenLayers'>;
  history: { canUndo: boolean; canRedo: boolean; undo: () => void; redo: () => void };
  chrome: {
    guardUnsaved: (action: () => void) => void;
    closePdf: () => void;
    exitApp: () => void;
    setViewMode: (mode: ViewMode) => void;
    setScrollViewMode: DocumentState['setScrollViewMode'];
    setShowBookmarksPanel: PanelsState['setShowBookmarksPanel'];
    setShowAnnotationsPanel: PanelsState['setShowAnnotationsPanel'];
    setShowPdfUaPanel: PanelsState['setShowPdfUaPanel'];
    setShowHiddenLayers: PanelsState['setShowHiddenLayers'];
    setShowPageEditsModal: AnnotationState['setShowPageEditsModal'];
    openTesseractGuide: () => void;
    openPdf: () => void;
    handlePrint: () => void;
    openPrintDialog: () => void;
    openSearchModal: () => void;
  };
  help: Pick<HelpState, 'setShowShortcutsHelp' | 'setShowLicenses' | 'setShowCredits' | 'setShowAbout' | 'setShowUpdateModal' | 'updaterSupported' | 'setShowCommandPalette'>;
  surface: { activeSurface: AppSurface; openSettings: (focus?: SettingsFocusSection) => void };
  workspace: { workspaceView: WorkspaceViewMode; setWorkspaceView: (mode: WorkspaceViewMode) => void };
  shortcutBindings: ShortcutBindings;
  pdfActions: AppPdfActions;
  pdfEdit: Pick<PdfEditState, 'editMode'>;
};
