import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useMemo, type MutableRefObject } from 'react';
import { isTauriRuntime } from './tauriRuntime';

type UseWindowTitleOptions = {
  filePath: string;
  originalPath: string;
  isDirty: boolean;
  isDirtyRef: MutableRefObject<boolean>;
  filePathRef: MutableRefObject<string>;
};

export function useWindowTitle(opts: UseWindowTitleOptions) {
  const windowTitle = useMemo(() => {
    const name = opts.originalPath ? (opts.originalPath.split('/').pop() ?? '') : '';
    return name ? `${opts.isDirty ? '• ' : ''}${name} - Kanoprii` : 'Kanoprii';
  }, [opts.isDirty, opts.originalPath]);

  useEffect(() => {
    opts.filePathRef.current = opts.filePath;
  }, [opts.filePath, opts.filePathRef]);

  useEffect(() => {
    opts.isDirtyRef.current = opts.isDirty;
    // Keep the DOM title in step with the OS window title: WebDriver tooling
    // (and a11y tools) match windows by document.title.
    document.title = windowTitle;
    if (isTauriRuntime()) {
      void getCurrentWindow().setTitle(windowTitle);
    }
  }, [opts.isDirty, opts.isDirtyRef, windowTitle]);

  return { windowTitle };
}
