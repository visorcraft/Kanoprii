import { useEffect, useRef } from 'react';
import { SettingsCard } from './SettingsCard';
import { AppearanceSelect } from './AppearanceSelect';
import { ShortcutEditor } from './ShortcutEditor';
import type { AppearanceKey } from './appearancePalettes';
import type { ShortcutCommandId } from './shortcutRegistry';
import type { ShortcutBindings } from '../app/useShortcutBindingsState';
import type { SettingsFocusSection } from '../app/useAppSurfaceState';

type SettingsPageProps = {
  closeSettings?: () => void;
  hasDocument?: boolean;
  focusSection?: SettingsFocusSection;
  appearance: {
    appearance: AppearanceKey;
    setAppearance: (key: AppearanceKey) => void;
  };
  shortcuts: {
    bindings: ShortcutBindings;
    setBinding: (commandId: ShortcutCommandId, shortcuts: string[]) => void;
    resetBinding: (commandId: ShortcutCommandId) => void;
    resetAllBindings: () => void;
  };
};

function focusOwnsEscape(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
  if (target.dataset.shortcutCapture === 'true') return true;
  if (target.closest('.modal-backdrop, .command-palette')) return true;
  return false;
}

function overlayOwnsEscape(): boolean {
  return (
    document.querySelector('.modal-backdrop, .command-palette-backdrop') !==
    null
  );
}

export function SettingsPage({
  closeSettings,
  hasDocument,
  focusSection = null,
  appearance,
  shortcuts,
}: SettingsPageProps) {
  const headingRef = useRef<HTMLHeadingElement>(null);
  const appearanceRef = useRef<HTMLElement>(null);
  const shortcutsRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const target =
      focusSection === 'appearance'
        ? appearanceRef.current
        : focusSection === 'shortcuts'
          ? shortcutsRef.current
          : headingRef.current;
    if (!target) return;
    if (focusSection) {
      target.scrollIntoView({ block: 'start' });
      target.focus({ preventScroll: true });
      return;
    }
    target.focus();
  }, [focusSection]);

  useEffect(() => {
    if (!closeSettings) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      if (overlayOwnsEscape()) return;
      if (focusOwnsEscape(e.target)) return;
      e.preventDefault();
      closeSettings();
    };
    window.addEventListener('keydown', onKeyDown, true);
    return () => window.removeEventListener('keydown', onKeyDown, true);
  }, [closeSettings]);

  return (
    <main className="settings-page" role="main" aria-label="Settings">
      <header className="settings-header">
        <div className="settings-header-content">
          <div>
            <h1 ref={headingRef} tabIndex={-1} className="settings-title">
              Settings
            </h1>
            <p className="settings-subtitle">Preferences for Kanoprii.</p>
          </div>
          {closeSettings && (
            <button
              type="button"
              className="settings-back-button"
              onClick={closeSettings}
              data-testid="settings-back-button"
            >
              {hasDocument ? 'Back to document' : 'Back'}
            </button>
          )}
        </div>
      </header>
      <section className="settings-content">
        <SettingsCard
          ref={appearanceRef}
          id="settings-appearance"
          tabIndex={-1}
          title="Appearance"
          subtitle="Choose the color scheme for the app."
        >
          <AppearanceSelect
            appearance={appearance.appearance}
            setAppearance={appearance.setAppearance}
          />
        </SettingsCard>

        <SettingsCard
          ref={shortcutsRef}
          id="settings-shortcuts"
          tabIndex={-1}
          title="Keyboard shortcuts"
          subtitle="Search, view, and customize keyboard shortcuts."
        >
          <ShortcutEditor
            bindings={shortcuts.bindings}
            setBinding={shortcuts.setBinding}
            resetBinding={shortcuts.resetBinding}
          />
        </SettingsCard>

        <SettingsCard
          title="Actions"
          subtitle="Reset preferences to their defaults."
        >
          <div className="settings-actions-row">
            <button
              type="button"
              className="settings-action-button"
              onClick={shortcuts.resetAllBindings}
            >
              Reset all shortcuts
            </button>
            <button
              type="button"
              className="settings-action-button"
              onClick={() => appearance.setAppearance('system')}
            >
              Reset appearance to Follow system
            </button>
          </div>
        </SettingsCard>
      </section>
    </main>
  );
}
