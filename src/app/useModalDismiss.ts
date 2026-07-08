import { useCallback, useMemo } from 'react';
import type { UnsavedChoice } from '../modals/UnsavedChangesModal';

type ModalDismissSetters = {
  setShowSaveAsModal: (show: boolean) => void;
  setShowMarkdownSaveAsModal: (show: boolean) => void;
  setShowProtectModal: (show: boolean) => void;
  setShowSignModal: (show: boolean) => void;
  setShowMetadataModal: (show: boolean) => void;
  setShowPasswordModal: (show: boolean) => void;
  setPendingEncryptedPath: (path: string) => void;
  setPdfPasswordDraft: (value: string) => void;
  setShowOpenModal: (show: boolean) => void;
  setShowBrowserModal: (show: boolean) => void;
  setShowDeleteModal: (show: boolean) => void;
  setShowSplitModal: (show: boolean) => void;
  setShowExtractModal: (show: boolean) => void;
  setShowExportPngModal: (show: boolean) => void;
  setShowDeleteRangeModal: (show: boolean) => void;
  setShowPageNumbersModal: (show: boolean) => void;
  setShowWatermarkModal: (show: boolean) => void;
  setShowCropModal: (show: boolean) => void;
  setShowFlattenModal: (show: boolean) => void;
  setShowAddBookmarkModal: (show: boolean) => void;
  setShowRenameBookmarkModal: (show: boolean) => void;
  setShowDuplicateRangeModal: (show: boolean) => void;
  setShowPageHeaderModal: (show: boolean) => void;
  setShowPageFooterModal: (show: boolean) => void;
  setShowSwapPagesModal: (show: boolean) => void;
  setShowReplacePageModal: (show: boolean) => void;
  setShowInterleaveModal: (show: boolean) => void;
  setShowPageSizeModal: (show: boolean) => void;
  setShowDecryptModal: (show: boolean) => void;
  setShowRotateModal: (show: boolean) => void;
  setShowKeepRangeModal: (show: boolean) => void;
  setShowMoveRangeModal: (show: boolean) => void;
  setShowPrependModal: (show: boolean) => void;
  setShowSplitEveryModal: (show: boolean) => void;
  setShowPageBorderModal: (show: boolean) => void;
  setShowBookmarkAllModal: (show: boolean) => void;
  setShowExpandMarginsModal: (show: boolean) => void;
  setShowShrinkMarginsModal: (show: boolean) => void;
  setShowDeleteNthModal: (show: boolean) => void;
  setShowExtractOddModal: (show: boolean) => void;
  setShowExtractEvenModal: (show: boolean) => void;
  setShowSplitAtModal: (show: boolean) => void;
  setShowReverseRangeModal: (show: boolean) => void;
  setShowInsertBlankPagesModal: (show: boolean) => void;
  setShowCropRangeModal: (show: boolean) => void;
  setShowParityRangeModal: (show: boolean) => void;
  setShowExportPagesPdfModal: (show: boolean) => void;
  setShowInsertImagePageModal: (show: boolean) => void;
  setShowExportPagePdfModal: (show: boolean) => void;
  setShowInsertModal: (show: boolean) => void;
  setInsertFilePath: (path: string) => void;
  setShowMergeModal: (show: boolean) => void;
  setMergeFilePath: (path: string) => void;
  setShowNoteModal: (show: boolean) => void;
  setShowImageInsertModal: (show: boolean) => void;
  setShowAddFormFieldModal: (show: boolean) => void;
  setShowSummaryModal: (show: boolean) => void;
  setShowPageTextModal: (show: boolean) => void;
  setEditingTextIndex: (index: number | null) => void;
  setPendingTextPos: (pos: { x: number; y: number } | null) => void;
  setShowPageEditsModal: (show: boolean) => void;
  setShowCommandPalette: (show: boolean) => void;
  setShowShortcutsHelp: (show: boolean) => void;
  setShowLicenses: (show: boolean) => void;
  setShowCredits: (show: boolean) => void;
  setShowAbout: (show: boolean) => void;
  setShowTesseractModal: (show: boolean) => void;
};

type ModalDismissFlags = {
  showUnsavedModal: boolean;
  showSaveAsModal: boolean;
  showMarkdownSaveAsModal: boolean;
  showProtectModal: boolean;
  showSignModal: boolean;
  showMetadataModal: boolean;
  showPasswordModal: boolean;
  showOpenModal: boolean;
  showBrowserModal: boolean;
  showDeleteModal: boolean;
  showSplitModal: boolean;
  showExtractModal: boolean;
  showExportPngModal: boolean;
  showDeleteRangeModal: boolean;
  showPageNumbersModal: boolean;
  showWatermarkModal: boolean;
  showCropModal: boolean;
  showFlattenModal: boolean;
  showAddBookmarkModal: boolean;
  showRenameBookmarkModal: boolean;
  showDuplicateRangeModal: boolean;
  showPageHeaderModal: boolean;
  showPageFooterModal: boolean;
  showSwapPagesModal: boolean;
  showReplacePageModal: boolean;
  showInterleaveModal: boolean;
  showPageSizeModal: boolean;
  showDecryptModal: boolean;
  showRotateModal: boolean;
  showKeepRangeModal: boolean;
  showMoveRangeModal: boolean;
  showPrependModal: boolean;
  showSplitEveryModal: boolean;
  showPageBorderModal: boolean;
  showBookmarkAllModal: boolean;
  showExpandMarginsModal: boolean;
  showShrinkMarginsModal: boolean;
  showDeleteNthModal: boolean;
  showExtractOddModal: boolean;
  showExtractEvenModal: boolean;
  showSplitAtModal: boolean;
  showReverseRangeModal: boolean;
  showInsertBlankPagesModal: boolean;
  showCropRangeModal: boolean;
  showParityRangeModal: boolean;
  showExportPagesPdfModal: boolean;
  showInsertImagePageModal: boolean;
  showExportPagePdfModal: boolean;
  showInsertModal: boolean;
  showMergeModal: boolean;
  showSearchModal: boolean;
  showNoteModal: boolean;
  showImageInsertModal: boolean;
  showAddFormFieldModal: boolean;
  showSummaryModal: boolean;
  showPageTextModal: boolean;
  showPageEditsModal: boolean;
  showCommandPalette: boolean;
  showShortcutsHelp: boolean;
  showLicenses: boolean;
  showCredits: boolean;
  showAbout: boolean;
  showTesseractModal: boolean;
};

export type UseModalDismissOptions = ModalDismissSetters &
  ModalDismissFlags & {
    closeSearchModal: () => void;
    resolveUnsaved: (choice: UnsavedChoice) => void | Promise<void>;
  };

export function useModalDismiss(opts: UseModalDismissOptions) {
  const { showUnsavedModal, closeSearchModal, resolveUnsaved, ...rest } = opts;

  const dismissModals = useCallback(() => {
    if (showUnsavedModal) {
      void resolveUnsaved('cancel');
      return;
    }
    rest.setShowSaveAsModal(false);
    rest.setShowMarkdownSaveAsModal(false);
    rest.setShowProtectModal(false);
    rest.setShowSignModal(false);
    rest.setShowMetadataModal(false);
    rest.setShowPasswordModal(false);
    rest.setPendingEncryptedPath('');
    rest.setPdfPasswordDraft('');
    rest.setShowOpenModal(false);
    rest.setShowBrowserModal(false);
    rest.setShowDeleteModal(false);
    rest.setShowSplitModal(false);
    rest.setShowExtractModal(false);
    rest.setShowExportPngModal(false);
    rest.setShowDeleteRangeModal(false);
    rest.setShowPageNumbersModal(false);
    rest.setShowWatermarkModal(false);
    rest.setShowCropModal(false);
    rest.setShowFlattenModal(false);
    rest.setShowAddBookmarkModal(false);
    rest.setShowRenameBookmarkModal(false);
    rest.setShowDuplicateRangeModal(false);
    rest.setShowPageHeaderModal(false);
    rest.setShowPageFooterModal(false);
    rest.setShowSwapPagesModal(false);
    rest.setShowReplacePageModal(false);
    rest.setShowInterleaveModal(false);
    rest.setShowPageSizeModal(false);
    rest.setShowDecryptModal(false);
    rest.setShowRotateModal(false);
    rest.setShowKeepRangeModal(false);
    rest.setShowMoveRangeModal(false);
    rest.setShowPrependModal(false);
    rest.setShowSplitEveryModal(false);
    rest.setShowPageBorderModal(false);
    rest.setShowBookmarkAllModal(false);
    rest.setShowExpandMarginsModal(false);
    rest.setShowShrinkMarginsModal(false);
    rest.setShowDeleteNthModal(false);
    rest.setShowExtractOddModal(false);
    rest.setShowExtractEvenModal(false);
    rest.setShowSplitAtModal(false);
    rest.setShowReverseRangeModal(false);
    rest.setShowInsertBlankPagesModal(false);
    rest.setShowCropRangeModal(false);
    rest.setShowParityRangeModal(false);
    rest.setShowExportPagesPdfModal(false);
    rest.setShowInsertImagePageModal(false);
    rest.setShowExportPagePdfModal(false);
    rest.setShowInsertModal(false);
    rest.setInsertFilePath('');
    rest.setShowMergeModal(false);
    rest.setMergeFilePath('');
    closeSearchModal();
    rest.setShowNoteModal(false);
    rest.setShowImageInsertModal(false);
    rest.setShowAddFormFieldModal(false);
    rest.setShowSummaryModal(false);
    rest.setShowPageTextModal(false);
    rest.setEditingTextIndex(null);
    rest.setPendingTextPos(null);
    rest.setShowPageEditsModal(false);
    rest.setShowCommandPalette(false);
    rest.setShowShortcutsHelp(false);
    rest.setShowLicenses(false);
    rest.setShowCredits(false);
    rest.setShowAbout(false);
    rest.setShowTesseractModal(false);
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: stable option object / destructured deps
  }, [showUnsavedModal, closeSearchModal, resolveUnsaved]);

  const anyModalOpen = useMemo(
    () =>
      showUnsavedModal ||
      rest.showSaveAsModal ||
      rest.showMarkdownSaveAsModal ||
      rest.showProtectModal ||
      rest.showSignModal ||
      rest.showMetadataModal ||
      rest.showPasswordModal ||
      rest.showOpenModal ||
      rest.showBrowserModal ||
      rest.showDeleteModal ||
      rest.showSplitModal ||
      rest.showExtractModal ||
      rest.showExportPngModal ||
      rest.showDeleteRangeModal ||
      rest.showPageNumbersModal ||
      rest.showWatermarkModal ||
      rest.showCropModal ||
      rest.showFlattenModal ||
      rest.showAddBookmarkModal ||
      rest.showRenameBookmarkModal ||
      rest.showDuplicateRangeModal ||
      rest.showPageHeaderModal ||
      rest.showPageFooterModal ||
      rest.showSwapPagesModal ||
      rest.showReplacePageModal ||
      rest.showInterleaveModal ||
      rest.showPageSizeModal ||
      rest.showDecryptModal ||
      rest.showRotateModal ||
      rest.showKeepRangeModal ||
      rest.showMoveRangeModal ||
      rest.showPrependModal ||
      rest.showSplitEveryModal ||
      rest.showPageBorderModal ||
      rest.showBookmarkAllModal ||
      rest.showExpandMarginsModal ||
      rest.showShrinkMarginsModal ||
      rest.showDeleteNthModal ||
      rest.showExtractOddModal ||
      rest.showExtractEvenModal ||
      rest.showSplitAtModal ||
      rest.showReverseRangeModal ||
      rest.showInsertBlankPagesModal ||
      rest.showCropRangeModal ||
      rest.showParityRangeModal ||
      rest.showExportPagesPdfModal ||
      rest.showInsertImagePageModal ||
      rest.showExportPagePdfModal ||
      rest.showInsertModal ||
      rest.showMergeModal ||
      rest.showSearchModal ||
      rest.showNoteModal ||
      rest.showImageInsertModal ||
      rest.showAddFormFieldModal ||
      rest.showSummaryModal ||
      rest.showPageTextModal ||
      rest.showPageEditsModal ||
      rest.showCommandPalette ||
      rest.showShortcutsHelp ||
      rest.showLicenses ||
      rest.showCredits ||
      rest.showAbout ||
      rest.showTesseractModal,
    [
      showUnsavedModal,
      rest.showSaveAsModal,
      rest.showMarkdownSaveAsModal,
      rest.showProtectModal,
      rest.showSignModal,
      rest.showMetadataModal,
      rest.showPasswordModal,
      rest.showOpenModal,
      rest.showBrowserModal,
      rest.showDeleteModal,
      rest.showSplitModal,
      rest.showExtractModal,
      rest.showExportPngModal,
      rest.showDeleteRangeModal,
      rest.showPageNumbersModal,
      rest.showWatermarkModal,
      rest.showCropModal,
      rest.showFlattenModal,
      rest.showAddBookmarkModal,
      rest.showRenameBookmarkModal,
      rest.showDuplicateRangeModal,
      rest.showPageHeaderModal,
      rest.showPageFooterModal,
      rest.showSwapPagesModal,
      rest.showReplacePageModal,
      rest.showInterleaveModal,
      rest.showPageSizeModal,
      rest.showDecryptModal,
      rest.showRotateModal,
      rest.showKeepRangeModal,
      rest.showMoveRangeModal,
      rest.showPrependModal,
      rest.showSplitEveryModal,
      rest.showPageBorderModal,
      rest.showBookmarkAllModal,
      rest.showExpandMarginsModal,
      rest.showShrinkMarginsModal,
      rest.showDeleteNthModal,
      rest.showExtractOddModal,
      rest.showExtractEvenModal,
      rest.showSplitAtModal,
      rest.showReverseRangeModal,
      rest.showInsertBlankPagesModal,
      rest.showCropRangeModal,
      rest.showParityRangeModal,
      rest.showExportPagesPdfModal,
      rest.showInsertImagePageModal,
      rest.showExportPagePdfModal,
      rest.showInsertModal,
      rest.showMergeModal,
      rest.showSearchModal,
      rest.showNoteModal,
      rest.showImageInsertModal,
      rest.showAddFormFieldModal,
      rest.showSummaryModal,
      rest.showPageTextModal,
      rest.showPageEditsModal,
      rest.showCommandPalette,
      rest.showShortcutsHelp,
      rest.showLicenses,
      rest.showCredits,
      rest.showAbout,
      rest.showTesseractModal,
    ]
  );

  return { dismissModals, anyModalOpen };
}
