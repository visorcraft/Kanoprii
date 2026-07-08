import { invoke } from '@tauri-apps/api/core';

export type ToastType = 'success' | 'error';

export interface StructuralEditDeps {
  filePath: string;
  currentPage: number;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  reloadOpenPdf: (nextPage?: number) => Promise<void>;
  showToast: (message: string, type?: ToastType) => void;
}

export type ReloadAt<T> =
  | number
  | ((result: T, deps: StructuralEditDeps) => number | Promise<number>);

export interface RunStructuralEditOptions<T = void> {
  command: string;
  args?: Record<string, unknown>;
  /** Default true. */
  markDirty?: boolean;
  /** Default: reload at currentPage. Ignored when afterEdit is set. */
  reloadAt?: ReloadAt<T>;
  /** Custom post-edit work (reload, refresh bookmarks, annotations, etc.). Replaces default reload. */
  afterEdit?: (result: T, deps: StructuralEditDeps) => void | Promise<void>;
  /** Skip reload when no afterEdit is needed (metadata-only edits). */
  skipReload?: boolean;
  toast?: string | ((result: T) => string);
  toastType?: ToastType;
  onSuccess?: (result: T) => void;
}

export async function runStructuralEdit<T = unknown>(
  deps: StructuralEditDeps,
  options: RunStructuralEditOptions<T>,
): Promise<T | undefined> {
  if (!deps.filePath) return undefined;
  return deps.withLoading(async () => {
    const args = { path: deps.filePath, ...options.args };
    const result = await invoke(options.command, args) as T;
    if (options.markDirty !== false) deps.markPdfEdited();

    if (options.afterEdit) {
      await options.afterEdit(result, deps);
    } else if (!options.skipReload) {
      let page = deps.currentPage;
      if (typeof options.reloadAt === 'number') page = options.reloadAt;
      else if (typeof options.reloadAt === 'function') page = await options.reloadAt(result, deps);
      await deps.reloadOpenPdf(page);
    }

    if (options.toast) {
      const message = typeof options.toast === 'function' ? options.toast(result) : options.toast;
      deps.showToast(message, options.toastType ?? 'success');
    }
    options.onSuccess?.(result);
    return result;
  });
}

export function createStructuralEditRunner(deps: StructuralEditDeps) {
  return <T = unknown>(options: RunStructuralEditOptions<T>) => runStructuralEdit<T>(deps, options);
}
