import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';

type UseNotePasswordActionsOptions = {
  filePath: string;
  currentPage: number;
  noteDraft: string;
  pendingNotePos: { x: number; y: number } | null;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  refreshAnnotations: () => Promise<void>;
  exitNoteMode: () => void;
  showToast: (msg: string, kind?: 'error') => void;
  setShowPasswordModal: (open: boolean) => void;
  setPendingEncryptedPath: (path: string) => void;
  setPdfPasswordDraft: (password: string) => void;
};

export function useNotePasswordActions(opts: UseNotePasswordActionsOptions) {
  const closePasswordModal = useCallback(() => {
    opts.setShowPasswordModal(false);
    opts.setPendingEncryptedPath('');
    opts.setPdfPasswordDraft('');
  }, [opts]);

  const submitTextNote = useCallback(() => {
    const text = opts.noteDraft.trim();
    const pos = opts.pendingNotePos;
    if (!text || !pos) return;
    void opts.withLoading(async () => {
      await invoke('add_text_note', {
        path: opts.filePath,
        pageIndex: opts.currentPage,
        x: pos.x,
        y: pos.y,
        content: text,
      });
      opts.markPdfEdited();
      await opts.refreshAnnotations();
      opts.showToast('Note added');
      opts.exitNoteMode();
    });
  }, [opts]);

  return { closePasswordModal, submitTextNote };
}
