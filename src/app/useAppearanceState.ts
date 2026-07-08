import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  type AppearanceKey,
  APPEARANCE_KEY_MAP,
  LEGACY_KEY_MAP,
} from '../settings/appearancePalettes';

const STORAGE_KEY = 'kanoprii-appearance';
const LEGACY_THEME_KEY = 'kanoprii-theme';

function getSystemTheme(): 'light' | 'dark' {
  if (typeof window === 'undefined') return 'light';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function normalizeStoredKey(value: string | null): AppearanceKey {
  if (!value) return 'system';
  if (value in APPEARANCE_KEY_MAP) return value as AppearanceKey;
  if (value in LEGACY_KEY_MAP) return LEGACY_KEY_MAP[value];
  return 'system';
}

function getStoredAppearance(): AppearanceKey {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return normalizeStoredKey(stored);
    }
    const legacy = localStorage.getItem(LEGACY_THEME_KEY);
    if (legacy) {
      const normalized = normalizeStoredKey(legacy);
      localStorage.setItem(STORAGE_KEY, normalized);
      return normalized;
    }
  } catch {
    // localStorage may be unavailable
  }
  return 'system';
}

function storeAppearance(key: AppearanceKey): void {
  try {
    localStorage.setItem(STORAGE_KEY, key);
  } catch {
    // ignore
  }
}

function resolvedKey(key: AppearanceKey): Exclude<AppearanceKey, 'system'> {
  return key === 'system' ? getSystemTheme() : key;
}

function cssVariable(name: string, value: string): void {
  document.documentElement.style.setProperty(name, value);
}

function applyColorMix(name: string, color: string, background: string, percent: number): void {
  cssVariable(name, `color-mix(in srgb, ${color} ${percent}%, ${background})`);
}

function applyAppearance(key: AppearanceKey): void {
  const effective = resolvedKey(key);
  const palette = APPEARANCE_KEY_MAP[effective].palette;

  document.documentElement.setAttribute('data-theme', effective === 'light' ? 'light' : 'dark');
  document.documentElement.setAttribute('data-appearance', key);

  cssVariable('--pp-bg', palette.background);
  cssVariable('--pp-surface', palette.alternateBackground);
  cssVariable('--pp-surface-alt', palette.tertiaryBackground);
  cssVariable('--pp-text', palette.text);
  cssVariable('--pp-text-secondary', palette.text);
  cssVariable('--pp-text-muted', palette.disabledText);
  applyColorMix('--pp-border', palette.text, palette.background, 20);
  applyColorMix('--pp-border-light', palette.text, palette.background, 16);
  cssVariable('--pp-accent', palette.highlight);
  cssVariable('--pp-focus-ring', palette.focusRing);
  applyColorMix('--pp-accent-hover', palette.highlight, palette.text, 82);
  cssVariable('--pp-menu-bg', palette.alternateBackground);
  cssVariable('--pp-menu-hover', palette.tertiaryBackground);
  cssVariable('--pp-menu-border', `color-mix(in srgb, ${palette.text} 20%, ${palette.background})`);
  cssVariable('--pp-menu-text', palette.text);
  cssVariable('--pp-menu-disabled', palette.disabledText);
  cssVariable('--pp-tab-bar-bg', palette.background);
  cssVariable('--pp-tab-bar-border', `color-mix(in srgb, ${palette.text} 20%, ${palette.background})`);
  cssVariable('--pp-tab-inactive-bg', palette.alternateBackground);
  cssVariable('--pp-tab-inactive-text', palette.disabledText);
  cssVariable('--pp-tab-active-bg', palette.background);
  cssVariable('--pp-tab-active-text', palette.text);
  cssVariable('--pp-overlay-bg', `color-mix(in srgb, ${palette.background} 60%, transparent)`);
  cssVariable('--pp-sidebar-bg', palette.alternateBackground);
  cssVariable('--pp-sidebar-border', `color-mix(in srgb, ${palette.text} 20%, ${palette.background})`);
  cssVariable('--pp-thumbnail-border', `color-mix(in srgb, ${palette.text} 20%, ${palette.background})`);
  cssVariable('--pp-thumbnail-hover', palette.disabledText);
  cssVariable('--pp-thumbnail-active', palette.highlight);
  cssVariable('--pp-input-border', `color-mix(in srgb, ${palette.text} 20%, ${palette.background})`);
  cssVariable('--pp-input-focus', palette.highlight);
  // Toast text uses the fixed semantic colors from styles.css so notifications
  // stay readable across every palette (some palette positive/negative reds and
  // greens fail contrast on the light toast backgrounds).
  cssVariable('--pp-modal-bg', palette.alternateBackground);
  cssVariable('--pp-modal-text', palette.text);
  cssVariable('--pp-btn-secondary-bg', palette.tertiaryBackground);
  cssVariable('--pp-btn-secondary-border', `color-mix(in srgb, ${palette.text} 20%, ${palette.background})`);
  cssVariable('--pp-btn-hover', palette.tertiaryBackground);

  cssVariable('--pp-highlighted-text', palette.highlightedText);
  cssVariable('--pp-positive', palette.positiveText);
  cssVariable('--pp-negative', palette.negativeText);
  cssVariable('--pp-neutral', palette.neutralText);
  cssVariable('--pp-bg-lift', palette.tertiaryBackground);
}

export function useAppearanceState() {
  const [appearance, setAppearanceState] = useState<AppearanceKey>(getStoredAppearance);

  useEffect(() => {
    applyAppearance(appearance);
  }, [appearance]);

  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = () => {
      if (appearance === 'system') {
        applyAppearance('system');
      }
    };
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, [appearance]);

  const setAppearance = useCallback((next: AppearanceKey) => {
    storeAppearance(next);
    setAppearanceState(next);
  }, []);

  const effectiveAppearance = useMemo(() => resolvedKey(appearance), [appearance]);

  return { appearance, effectiveAppearance, setAppearance };
}
