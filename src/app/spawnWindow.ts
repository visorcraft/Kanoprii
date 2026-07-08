import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { listen } from '@tauri-apps/api/event';
import { isTauriRuntime } from './tauriRuntime';

export type SpawnParams = { spawn: boolean; openPath: string | null; token: string | null };

/** Read this window's launch query: spawned document windows carry spawn=1. */
export function readSpawnParams(): SpawnParams {
  const p = new URLSearchParams(window.location.search);
  return { spawn: p.get('spawn') === '1', openPath: p.get('openPath'), token: p.get('token') };
}

let labelCounter = 0;
function uniqueLabel(): string {
  // Globally unique across windows (each window has its own counter, so mix in
  // time + randomness). Must start with `win-` to match the capability glob.
  labelCounter += 1;
  const rand = Math.floor(Math.random() * 1_000_000).toString(36);
  return `win-${Date.now().toString(36)}-${labelCounter}-${rand}`;
}

type SpawnDeps = {
  finalizeClose: (sessionId: string) => void | Promise<void>;
  showToast: (message: string, type?: 'success' | 'error') => void;
};

/**
 * Open `originalPath` in a fresh app window, then close the source tab - but
 * only once the new window acks a successful load (`spawn-loaded`). On creation
 * failure or a 15s timeout with no ack, the source tab is kept.
 */
export async function spawnDocumentWindow(originalPath: string, sourceId: string, deps: SpawnDeps): Promise<void> {
  if (!isTauriRuntime()) return;
  const label = uniqueLabel();
  const token = label;
  const url = `index.html?spawn=1&openPath=${encodeURIComponent(originalPath)}&token=${encodeURIComponent(token)}`;
  let settled = false;

  const unlisten = await listen<{ token: string }>('spawn-loaded', (event) => {
    if (event.payload?.token === token && !settled) {
      settled = true;
      void deps.finalizeClose(sourceId);
      unlisten();
    }
  });

  try {
    const win = new WebviewWindow(label, {
      url,
      width: 950,
      height: 750,
      decorations: false,
      resizable: true,
      title: 'Kanoprii',
    });
    void win.once('tauri://error', () => {
      if (!settled) {
        settled = true;
        unlisten();
        deps.showToast('Could not open a new window', 'error');
      }
    });
  } catch {
    if (!settled) {
      settled = true;
      unlisten();
      deps.showToast('Could not open a new window', 'error');
    }
    return;
  }

  // Fallback: if the spawned window never acks (e.g. load needs a password and
  // the user cancels), stop waiting and keep the source tab open.
  setTimeout(() => {
    if (!settled) {
      settled = true;
      unlisten();
    }
  }, 15_000);
}
