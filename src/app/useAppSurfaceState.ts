import { useCallback, useRef, useState } from 'react';

export type AppSurface = 'document' | 'settings';
export type SettingsFocusSection = 'appearance' | 'shortcuts' | null;

type SurfaceState = {
  activeSurface: AppSurface;
  settingsFocus: SettingsFocusSection;
  openSettings: (focus?: SettingsFocusSection) => void;
  closeSettings: () => void;
};

export function useAppSurfaceState(): SurfaceState {
  const [activeSurface, setActiveSurface] = useState<AppSurface>('document');
  const [settingsFocus, setSettingsFocus] =
    useState<SettingsFocusSection>(null);
  const previousFocusRef = useRef<Element | null>(null);

  const openSettings = useCallback((focus: SettingsFocusSection = null) => {
    previousFocusRef.current = document.activeElement;
    setSettingsFocus(focus);
    setActiveSurface('settings');
  }, []);

  const closeSettings = useCallback(() => {
    setActiveSurface('document');
    const target = previousFocusRef.current;
    if (target instanceof HTMLElement) {
      target.focus();
    } else {
      document
        .querySelector<HTMLElement>('.menu-bar .menu-bar-trigger')
        ?.focus();
    }
  }, []);

  return { activeSurface, settingsFocus, openSettings, closeSettings };
}
