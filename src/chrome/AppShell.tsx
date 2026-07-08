import { TitleBar } from './TitleBar';
import { ResizeBorders } from './ResizeBorders';
import { Toast } from '../ui/Toast';
import { AppChrome } from './AppChrome';
import { useFocusCycle } from './useFocusCycle';
import { AppBody } from '../viewer/AppBody';
import { AppModals } from '../modals/AppModals';
import { PrintSurface } from '../viewer/PrintSurface';
import { SettingsPage } from '../settings/SettingsPage';
import { ErrorBoundary } from '../ui/ErrorBoundary';
import type { ShortcutBindingsState } from '../app/useShortcutBindingsState';
import type { AppearanceKey } from '../settings/appearancePalettes';
import type { SettingsFocusSection } from '../app/useAppSurfaceState';

type ToastState = { message: string; type: 'success' | 'error' } | null;

type AppSurface = import('../app/useAppSurfaceState').AppSurface;

type AppShellProps = {
  windowTitle: string;
  toast: ToastState;
  loading: boolean;
  chrome: React.ComponentProps<typeof AppChrome>;
  body: React.ComponentProps<typeof AppBody>;
  modals: React.ComponentProps<typeof AppModals>;
  printPages: string[];
  activeSurface: AppSurface;
  settingsFocus: SettingsFocusSection;
  closeSettings: () => void;
  shortcuts: ShortcutBindingsState;
  showToast: (message: string, type?: 'success' | 'error') => void;
  dismissToast: () => void;
  appearance: {
    appearance: AppearanceKey;
    setAppearance: (key: AppearanceKey) => void;
  };
};

function panelFallback(name: string) {
  return (error: Error | null, onReset: () => void) => (
    <div className="error-boundary panel-error" role="alert">
      <h2>{name} error</h2>
      {error && (
        <p className="error-boundary-message">{error.message}</p>
      )}
      <p>This panel failed to render.</p>
      <button type="button" onClick={onReset}>
        Try again
      </button>
    </div>
  );
}

export function AppShell({
  windowTitle,
  toast,
  loading,
  chrome,
  body,
  modals,
  printPages,
  activeSurface,
  settingsFocus,
  closeSettings,
  shortcuts,
  showToast,
  dismissToast,
  appearance,
}: AppShellProps) {
  const hasDocument = !!body.filePath;
  useFocusCycle(activeSurface === 'document');

  const handlePanelError = (error: Error) => {
    showToast(`${error.name}: ${error.message}`, 'error');
  };

  return (
    <div className="app" data-active-surface={activeSurface}>
      <ResizeBorders />
      <TitleBar title={windowTitle} />
      <Toast notification={toast} onClose={dismissToast} />

      {loading && (
        <div className="loading-overlay">
          <div className="spinner" />
        </div>
      )}

      <ErrorBoundary
        fallback={panelFallback('Chrome')}
        onError={handlePanelError}
      >
        <AppChrome
          {...chrome}
          documentChromeVisible={activeSurface === 'document'}
          shortcutBindings={shortcuts.bindings}
        />
      </ErrorBoundary>

      {activeSurface === 'settings' ? (
        <ErrorBoundary
          fallback={panelFallback('Settings')}
          onError={handlePanelError}
        >
          <SettingsPage
            closeSettings={closeSettings}
            hasDocument={hasDocument}
            focusSection={settingsFocus}
            appearance={appearance}
            shortcuts={shortcuts}
          />
        </ErrorBoundary>
      ) : (
        <ErrorBoundary
          fallback={panelFallback('Viewer')}
          onError={handlePanelError}
          resetKeys={[body.filePath]}
        >
          <AppBody {...body} />
        </ErrorBoundary>
      )}

      <ErrorBoundary
        fallback={() => null}
        onError={handlePanelError}
      >
        <AppModals {...modals} />
      </ErrorBoundary>
      <PrintSurface pages={printPages} />
    </div>
  );
}
