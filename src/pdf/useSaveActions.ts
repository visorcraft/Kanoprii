import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import { useAnnouncer } from '../ui/useAnnouncer';

type UseSaveActionsOptions = {
  filePath: string;
  originalPath: string;
  nativeDialogs: boolean;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markSaved: () => void;
  showToast: (msg: string, kind?: 'error') => void;
  saveAsViaNativeDialog: () => Promise<boolean>;
  saveAsPath: string;
  rememberOpenedPdf: (path: string) => void;
  setOriginalPath: (path: string) => void;
  setSaveAsPath: (path: string) => void;
  setShowSaveAsModal: (open: boolean) => void;
};

export function useSaveActions(opts: UseSaveActionsOptions) {
  const { announce } = useAnnouncer();

  const handleSave = useCallback(async () => {
    if (!opts.filePath || !opts.originalPath) return;
    await opts.withLoading(async () => {
      await invoke('save_working_copy', { working: opts.filePath, target: opts.originalPath });
      opts.markSaved();
      opts.showToast('Saved');
      announce('Saved');
    });
  }, [opts, announce]);

  const handleSaveAs = useCallback(async () => {
    const target = opts.saveAsPath.trim();
    if (!opts.filePath || !target) return;
    let saved = false;
    await opts.withLoading(async () => {
      await invoke('save_working_copy', { working: opts.filePath, target });
      opts.setOriginalPath(target);
      opts.rememberOpenedPdf(target);
      opts.markSaved();
      opts.setShowSaveAsModal(false);
      opts.showToast(`Saved to ${target}`);
      saved = true;
    });
    if (saved) {
      announce('Saved as new file');
    }
  }, [opts, announce]);

  const saveAsViaNativeDialog = useCallback(async () => {
    const saved = await opts.saveAsViaNativeDialog();
    if (saved) {
      announce('Saved as new file');
    }
  }, [opts, announce]);

  const openSaveAs = useCallback(() => {
    if (opts.nativeDialogs) {
      void saveAsViaNativeDialog();
      return;
    }
    opts.setSaveAsPath(opts.originalPath);
    opts.setShowSaveAsModal(true);
  }, [opts, saveAsViaNativeDialog]);

  return { handleSave, handleSaveAs, openSaveAs };
}
