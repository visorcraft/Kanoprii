import type { ComponentProps } from 'react';
import type { AppShell } from './AppShell';
import type { BuildAppChromeSourceInput } from './buildAppChromeSource';
import { buildAppChromeSource } from './buildAppChromeSource';
import type { BuildAppViewerSourceInput } from '../viewer/buildAppViewerSource';
import { buildAppViewerSource } from '../viewer/buildAppViewerSource';
import type { AppModalsRuntime } from '../modals/appModalsContext';
import type { ShortcutBindingsState } from '../app/useShortcutBindingsState';
import type { AppearanceKey } from '../settings/appearancePalettes';
import type { SettingsFocusSection } from '../app/useAppSurfaceState';

type AppShellInput = ComponentProps<typeof AppShell>;

export type BuildAppShellSourceInput = {
  windowTitle: string;
  toast: AppShellInput['toast'];
  loading: boolean;
  chrome: BuildAppChromeSourceInput;
  viewer: BuildAppViewerSourceInput;
  modalCtx: AppModalsRuntime;
  printPages: string[];
  activeSurface: import('../app/useAppSurfaceState').AppSurface;
  settingsFocus: SettingsFocusSection;
  closeSettings: () => void;
  shortcuts: ShortcutBindingsState;
  showToast: (message: string, type?: 'success' | 'error') => void;
  dismissToast: () => void;
  appearance: { appearance: AppearanceKey; setAppearance: (key: AppearanceKey) => void };
};

export function buildAppShellSource(input: BuildAppShellSourceInput): Omit<AppShellInput, 'children'> {
  return {
    windowTitle: input.windowTitle,
    toast: input.toast,
    loading: input.loading,
    chrome: buildAppChromeSource(input.chrome),
    body: buildAppViewerSource(input.viewer),
    modals: { ctx: input.modalCtx },
    printPages: input.printPages,
    activeSurface: input.activeSurface,
    settingsFocus: input.settingsFocus,
    closeSettings: input.closeSettings,
    shortcuts: input.shortcuts,
    showToast: input.showToast,
    dismissToast: input.dismissToast,
    appearance: input.appearance,
  };
}
