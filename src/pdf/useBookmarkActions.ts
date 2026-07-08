import { useCallback } from 'react';
import type { RunEdit } from './runEditTypes';

type UseBookmarkActionsOptions = {
  filePath: string;
  currentPage: number;
  bookmarkTitle: string;
  bookmarkAllPrefix: string;
  renameBookmarkIndex: number;
  renameBookmarkTitle: string;
  runEdit: RunEdit;
  loadPdfBookmarks: (path: string) => Promise<void>;
  setBookmarkTitle: (title: string) => void;
  setBookmarkAllPrefix: (prefix: string) => void;
  setRenameBookmarkIndex: (index: number) => void;
  setRenameBookmarkTitle: (title: string) => void;
  setShowAddBookmarkModal: (open: boolean) => void;
  setShowRenameBookmarkModal: (open: boolean) => void;
  setShowBookmarkAllModal: (open: boolean) => void;
};

export function useBookmarkActions(opts: UseBookmarkActionsOptions) {
  const openAddBookmarkModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setBookmarkTitle(`Page ${opts.currentPage + 1}`);
    opts.setShowAddBookmarkModal(true);
  }, [opts]);

  const handleAddBookmark = useCallback(async () => {
    if (!opts.filePath || !opts.bookmarkTitle.trim()) return;
    await opts.runEdit({
      command: 'add_pdf_bookmark',
      args: { title: opts.bookmarkTitle.trim(), pageIndex: opts.currentPage },
      afterEdit: async () => { await opts.loadPdfBookmarks(opts.filePath); },
      toast: 'Bookmark added',
      onSuccess: () => opts.setShowAddBookmarkModal(false),
    });
  }, [opts]);

  const openRenameBookmarkModal = useCallback((index: number, title: string) => {
    opts.setRenameBookmarkIndex(index);
    opts.setRenameBookmarkTitle(title);
    opts.setShowRenameBookmarkModal(true);
  }, [opts]);

  const handleRenameBookmark = useCallback(async () => {
    if (!opts.filePath || !opts.renameBookmarkTitle.trim()) return;
    await opts.runEdit({
      command: 'rename_pdf_bookmark',
      args: { bookmarkIndex: opts.renameBookmarkIndex, title: opts.renameBookmarkTitle.trim() },
      afterEdit: async () => { await opts.loadPdfBookmarks(opts.filePath); },
      toast: 'Bookmark renamed',
      onSuccess: () => opts.setShowRenameBookmarkModal(false),
    });
  }, [opts]);

  const handleRemoveBookmark = useCallback(async (index: number) => {
    await opts.runEdit({
      command: 'remove_pdf_bookmark',
      args: { bookmarkIndex: index },
      afterEdit: async () => { await opts.loadPdfBookmarks(opts.filePath); },
      toast: 'Bookmark removed',
    });
  }, [opts]);

  const openBookmarkAllModal = useCallback(() => {
    if (!opts.filePath) return;
    opts.setBookmarkAllPrefix('Page ');
    opts.setShowBookmarkAllModal(true);
  }, [opts]);

  const handleBookmarkAllPages = useCallback(async () => {
    await opts.runEdit({
      command: 'bookmark_all_pages',
      args: { prefix: opts.bookmarkAllPrefix.trim() || 'Page ' },
      afterEdit: async () => { await opts.loadPdfBookmarks(opts.filePath); },
      toast: (n) => `Added ${n} bookmark${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowBookmarkAllModal(false),
    });
  }, [opts]);

  const handleBookmarkOddPages = useCallback(async () => {
    await opts.runEdit({
      command: 'bookmark_odd_pages',
      args: { prefix: opts.bookmarkAllPrefix.trim() || 'Page ' },
      afterEdit: async () => { await opts.loadPdfBookmarks(opts.filePath); },
      toast: (n) => `Added ${n} odd bookmark${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowBookmarkAllModal(false),
    });
  }, [opts]);

  const handleBookmarkEvenPages = useCallback(async () => {
    await opts.runEdit({
      command: 'bookmark_even_pages',
      args: { prefix: opts.bookmarkAllPrefix.trim() || 'Page ' },
      afterEdit: async () => { await opts.loadPdfBookmarks(opts.filePath); },
      toast: (n) => `Added ${n} even bookmark${n === 1 ? '' : 's'}`,
      onSuccess: () => opts.setShowBookmarkAllModal(false),
    });
  }, [opts]);

  return {
    openAddBookmarkModal,
    handleAddBookmark,
    openRenameBookmarkModal,
    handleRenameBookmark,
    handleRemoveBookmark,
    openBookmarkAllModal,
    handleBookmarkAllPages,
    handleBookmarkOddPages,
    handleBookmarkEvenPages,
  };
}
