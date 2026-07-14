import type { AppMenuContextSource } from './types';
import type { ScrollViewMode, ViewMode, WorkspaceViewMode } from '../app/types';
import type { AppSurface, SettingsFocusSection } from '../app/useAppSurfaceState';
import type { ShortcutBindings } from '../app/useShortcutBindingsState';

/** Inputs from App hooks/state before void-wrapping into the menu context. */
export type BuildAppMenuSourceInput = Omit<
  AppMenuContextSource,
  | 'hasPdf'
  | 'hasDocument'
  | 'tesseractInstalled'
  | 'workspaceView'
  | 'requestClosePdf'
  | 'quitApp'
  | 'setWorkspaceViewBirdseye'
  | 'setWorkspaceViewTabs'
  | 'setViewModePdf'
  | 'toggleBookmarksPanel'
  | 'toggleAnnotationsPanel'
  | 'togglePdfUaPanel'
  | 'toggleShowHiddenLayers'
  | 'toggleContinuousScroll'
  | 'openPageEditsModal'
  | 'openShortcutsHelp'
  | 'openLicenses'
  | 'openCredits'
  | 'openAbout'
  | 'openUpdateModal'
  | 'openCommandPalette'
  | 'activeSurface'
  | 'openSettings'
  | 'shortcutBindings'
> & {
  filePath: string;
  hasDocument: boolean;
  pdfReady: boolean;
  ocrAvailable: boolean | null;
  surface: { activeSurface: AppSurface; openSettings: (focus?: SettingsFocusSection) => void };
  workspace: { workspaceView: WorkspaceViewMode; setWorkspaceView: (mode: WorkspaceViewMode) => void };
  guardUnsaved: (action: () => void) => void;
  closePdf: () => void;
  exitApp: () => void;
  setViewMode: (mode: ViewMode) => void;
  scrollViewMode: ScrollViewMode;
  setScrollViewMode: (fn: (prev: ScrollViewMode) => ScrollViewMode) => void;
  setShowBookmarksPanel: (fn: (prev: boolean) => boolean) => void;
  setShowAnnotationsPanel: (fn: (prev: boolean) => boolean) => void;
  setShowPdfUaPanel: (fn: (prev: boolean) => boolean) => void;
  setShowHiddenLayers: (fn: (prev: boolean) => boolean) => void;
  setShowPageEditsModal: (open: boolean) => void;
  setShowShortcutsHelp: (open: boolean) => void;
  setShowLicenses: (open: boolean) => void;
  setShowCredits: (open: boolean) => void;
  setShowAbout: (open: boolean) => void;
  setShowUpdateModal: (open: boolean) => void;
  updaterSupported: boolean;
  setShowCommandPalette: (open: boolean) => void;
  shortcutBindings: ShortcutBindings;
};

export function buildAppMenuSource(input: BuildAppMenuSourceInput): AppMenuContextSource {
  const {
    filePath,
    hasDocument,
    pdfReady,
    ocrAvailable,
    guardUnsaved,
    closePdf,
    exitApp,
    setViewMode,
    scrollViewMode,
    setScrollViewMode,
    setShowBookmarksPanel,
    setShowAnnotationsPanel,
    setShowPdfUaPanel,
    setShowHiddenLayers,
    setShowPageEditsModal,
    setShowShortcutsHelp,
    setShowLicenses,
    setShowCredits,
    setShowAbout,
    setShowUpdateModal,
    updaterSupported,
    setShowCommandPalette,
    surface,
    workspace,
    shortcutBindings,
    ...passthrough
  } = input;
  return {
    ...passthrough,
    updaterSupported,
    hasPdf: !!filePath && pdfReady,
    hasDocument,
    tesseractInstalled: ocrAvailable === true,
    requestClosePdf: () => guardUnsaved(closePdf),
    quitApp: () => guardUnsaved(exitApp),
    workspaceView: workspace.workspaceView,
    setWorkspaceViewBirdseye: () => workspace.setWorkspaceView('birdseye'),
    setWorkspaceViewTabs: () => workspace.setWorkspaceView('tabs'),
    setViewModePdf: () => {
      workspace.setWorkspaceView('tabs');
      setViewMode('pdf');
    },
    scrollViewMode,
    toggleContinuousScroll: () => setScrollViewMode((prev) => (prev === 'continuous' ? 'single' : 'continuous')),
    toggleBookmarksPanel: () => setShowBookmarksPanel((prev) => !prev),
    toggleAnnotationsPanel: () => setShowAnnotationsPanel((prev) => !prev),
    togglePdfUaPanel: () => setShowPdfUaPanel((prev) => !prev),
    toggleShowHiddenLayers: () => setShowHiddenLayers((prev) => !prev),
    openPageEditsModal: () => setShowPageEditsModal(true),
    openShortcutsHelp: () => setShowShortcutsHelp(true),
    openLicenses: () => setShowLicenses(true),
    openCredits: () => setShowCredits(true),
    openAbout: () => setShowAbout(true),
    openUpdateModal: () => setShowUpdateModal(true),
    openCommandPalette: () => setShowCommandPalette(true),
    activeSurface: surface.activeSurface,
    openSettings: (focus?: SettingsFocusSection) => surface.openSettings(focus),
    shortcutBindings,
  };
}
