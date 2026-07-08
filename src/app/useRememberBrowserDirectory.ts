import { useCallback } from 'react';
import { LAST_BROWSER_DIR_KEY } from './constants';
import { directoryFromPath, writeStoredString } from './utils';

type UseRememberBrowserDirectoryOptions = {
  setLastBrowserDir: (dir: string) => void;
};

export function useRememberBrowserDirectory({ setLastBrowserDir }: UseRememberBrowserDirectoryOptions) {
  return useCallback((path: string) => {
    const dir = directoryFromPath(path);
    if (!dir) return;
    setLastBrowserDir(dir);
    writeStoredString(LAST_BROWSER_DIR_KEY, dir);
  }, [setLastBrowserDir]);
}
