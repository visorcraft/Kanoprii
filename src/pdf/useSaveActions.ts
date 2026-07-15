import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import { useAnnouncer } from '../ui/useAnnouncer';

type UseSaveActionsOptions = {
  filePath: string;
  originalPath: string;
  /**
   * Auto-materialized sibling PDF path (markdown/HTML -> PDF view). When set,
   * "Save" writes here so the source document is not overwritten. Empty for
   * plain PDF sessions, in which case `originalPath` is used.
   */
  generatedPdfPath: string;
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
  // Save target: prefer the auto-generated sibling PDF (so "Save" on a
  // markdown-derived PDF view writes the .pdf sibling, not the .md source);
  // fall back to the plain-PDF original path.
  const saveTarget = opts.generatedPdfPath || opts.originalPath;

  const handleSave = useCallback(async () => {
    if (!opts.filePath || !saveTarget) return;
    await opts.withLoading(async () => {
      await invoke('save_working_copy', { working: opts.filePath, target: saveTarget });
      opts.markSaved();
      opts.showToast('Saved');
      announce('Saved');
    });
  }, [opts, saveTarget, announce]);

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
    opts.setSaveAsPath(saveTarget);
    opts.setShowSaveAsModal(true);
  }, [opts, saveAsViaNativeDialog, saveTarget]);

  return { handleSave, handleSaveAs, openSaveAs };
}
