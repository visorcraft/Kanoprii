import { useState } from 'react';
import { LAST_BROWSER_DIR_KEY, RECENT_PDFS_KEY } from './constants';
import type { PdfPageSize, PdfSummaryResult } from './types';
import { readStoredString, readStoredStringArray } from './utils';
import type { ImageExportFormat } from '../pdf/imageExportCommands';
import type { PageSizePreset } from '../modals/PageSizeModal';

export type RotateDirection = 'cw' | 'ccw';

export function useAppModalState() {
  const [showSaveAsModal, setShowSaveAsModal] = useState(false);
  const [saveAsPath, setSaveAsPath] = useState<string>('');
  const [showMarkdownSaveAsModal, setShowMarkdownSaveAsModal] = useState(false);
  const [markdownSaveAsPath, setMarkdownSaveAsPath] = useState('');
  const [nativeDialogs, setNativeDialogs] = useState(false);
  const [showSummaryModal, setShowSummaryModal] = useState(false);
  const [pdfSummary, setPdfSummary] = useState<PdfSummaryResult | null>(null);
  const [showOpenModal, setShowOpenModal] = useState(false);
  const [openFilePath, setOpenFilePath] = useState<string>('');
  const [showPrintDialog, setShowPrintDialog] = useState(false);
  const [recentPdfs, setRecentPdfs] = useState<string[]>(() => readStoredStringArray(RECENT_PDFS_KEY));
  const [lastBrowserDir, setLastBrowserDir] = useState<string>(() => readStoredString(LAST_BROWSER_DIR_KEY));

  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [deletePageInput, setDeletePageInput] = useState('1');
  const [showPageNumbersModal, setShowPageNumbersModal] = useState(false);
  const [pageNumbersPrefix, setPageNumbersPrefix] = useState('Page ');
  const [showWatermarkModal, setShowWatermarkModal] = useState(false);
  const [watermarkText, setWatermarkText] = useState('DRAFT');
  const [showCropModal, setShowCropModal] = useState(false);
  const [cropMarginTop, setCropMarginTop] = useState(50);
  const [cropMarginRight, setCropMarginRight] = useState(50);
  const [cropMarginBottom, setCropMarginBottom] = useState(50);
  const [cropMarginLeft, setCropMarginLeft] = useState(50);
  const [showFlattenModal, setShowFlattenModal] = useState(false);
  const [showAddBookmarkModal, setShowAddBookmarkModal] = useState(false);
  const [bookmarkTitle, setBookmarkTitle] = useState('');
  const [showRenameBookmarkModal, setShowRenameBookmarkModal] = useState(false);
  const [renameBookmarkIndex, setRenameBookmarkIndex] = useState(0);
  const [renameBookmarkTitle, setRenameBookmarkTitle] = useState('');
  const [cropApplyAll, setCropApplyAll] = useState(false);
  const [pageSizes, setPageSizes] = useState<PdfPageSize[]>([]);
  const [showPageHeaderModal, setShowPageHeaderModal] = useState(false);
  const [pageHeaderText, setPageHeaderText] = useState('DRAFT');
  const [showInsertImagePageModal, setShowInsertImagePageModal] = useState(false);
  const [insertImagePagePath, setInsertImagePagePath] = useState('');
  const [insertImageAtIndex, setInsertImageAtIndex] = useState(0);
  const [showExportPagePdfModal, setShowExportPagePdfModal] = useState(false);
  const [exportPagePdfPath, setExportPagePdfPath] = useState('');
  const [showExportPagesPdfModal, setShowExportPagesPdfModal] = useState(false);
  const [exportPagesPdfOutputDir, setExportPagesPdfOutputDir] = useState('');
  const [showPageFooterModal, setShowPageFooterModal] = useState(false);
  const [pageFooterText, setPageFooterText] = useState('Confidential');
  const [showSwapPagesModal, setShowSwapPagesModal] = useState(false);
  const [swapPageA, setSwapPageA] = useState(0);
  const [swapPageB, setSwapPageB] = useState(1);
  const [showBatesNumberModal, setShowBatesNumberModal] = useState(false);
  const [batesPrefix, setBatesPrefix] = useState('');
  const [batesStartNumber, setBatesStartNumber] = useState(1);
  const [batesDigits, setBatesDigits] = useState(6);
  const [batesPosition, setBatesPosition] = useState('footer-right');
  const [showApplyRedactionsModal, setShowApplyRedactionsModal] = useState(false);
  const [applyRedactionsOcrAfter, setApplyRedactionsOcrAfter] = useState(false);

  const [showDeleteRangeModal, setShowDeleteRangeModal] = useState(false);
  const [showDuplicateRangeModal, setShowDuplicateRangeModal] = useState(false);
  const [showKeepRangeModal, setShowKeepRangeModal] = useState(false);
  const [showMoveRangeModal, setShowMoveRangeModal] = useState(false);
  const [moveRangeToIndex, setMoveRangeToIndex] = useState(0);
  const [showExpandMarginsModal, setShowExpandMarginsModal] = useState(false);
  const [expandMarginTop, setExpandMarginTop] = useState(20);
  const [expandMarginRight, setExpandMarginRight] = useState(20);
  const [expandMarginBottom, setExpandMarginBottom] = useState(20);
  const [expandMarginLeft, setExpandMarginLeft] = useState(20);
  const [showShrinkMarginsModal, setShowShrinkMarginsModal] = useState(false);
  const [shrinkMarginTop, setShrinkMarginTop] = useState(20);
  const [shrinkMarginRight, setShrinkMarginRight] = useState(20);
  const [shrinkMarginBottom, setShrinkMarginBottom] = useState(20);
  const [shrinkMarginLeft, setShrinkMarginLeft] = useState(20);
  const [showDeleteNthModal, setShowDeleteNthModal] = useState(false);
  const [deleteNthValue, setDeleteNthValue] = useState(2);
  const [showReverseRangeModal, setShowReverseRangeModal] = useState(false);
  const [showInsertBlankPagesModal, setShowInsertBlankPagesModal] = useState(false);
  const [insertBlankCount, setInsertBlankCount] = useState(1);
  const [insertBlankAtIndex, setInsertBlankAtIndex] = useState(0);
  const [showCropRangeModal, setShowCropRangeModal] = useState(false);
  const [showParityRangeModal, setShowParityRangeModal] = useState(false);
  const [parityRangeCommand, setParityRangeCommand] = useState('rotate_odd_pages_in_range');
  const [parityRangeOutputPath, setParityRangeOutputPath] = useState('');

  const [showSplitModal, setShowSplitModal] = useState(false);
  const [splitRanges, setSplitRanges] = useState<string>('');
  const [showExtractModal, setShowExtractModal] = useState(false);
  const [extractOutputPath, setExtractOutputPath] = useState('');
  const [showExportPngModal, setShowExportPngModal] = useState(false);
  const [pngExportOutputPath, setPngExportOutputPath] = useState('');
  const [imageExportFormat, setImageExportFormat] = useState<ImageExportFormat>('png');
  const [showReplacePageModal, setShowReplacePageModal] = useState(false);
  const [replaceSourcePath, setReplaceSourcePath] = useState('');
  const [replaceSourcePage, setReplaceSourcePage] = useState(0);
  const [replaceSourcePageCount, setReplaceSourcePageCount] = useState<number | null>(null);
  const [showInterleaveModal, setShowInterleaveModal] = useState(false);
  const [interleaveFilePath, setInterleaveFilePath] = useState('');
  const [interleaveSourcePageCount, setInterleaveSourcePageCount] = useState<number | null>(null);
  const [showPageSizeModal, setShowPageSizeModal] = useState(false);
  const [pageSizePreset, setPageSizePreset] = useState<PageSizePreset>('letter');
  const [showPrependModal, setShowPrependModal] = useState(false);
  const [prependFilePath, setPrependFilePath] = useState('');
  const [prependSourcePageCount, setPrependSourcePageCount] = useState<number | null>(null);
  const [showSplitEveryModal, setShowSplitEveryModal] = useState(false);
  const [splitEveryN, setSplitEveryN] = useState(2);
  const [showPageBorderModal, setShowPageBorderModal] = useState(false);
  const [pageBorderInset, setPageBorderInset] = useState(20);
  const [showBookmarkAllModal, setShowBookmarkAllModal] = useState(false);
  const [bookmarkAllPrefix, setBookmarkAllPrefix] = useState('Page ');
  const [showExtractOddModal, setShowExtractOddModal] = useState(false);
  const [extractOddOutputPath, setExtractOddOutputPath] = useState('');
  const [showExtractEvenModal, setShowExtractEvenModal] = useState(false);
  const [extractEvenOutputPath, setExtractEvenOutputPath] = useState('');
  const [showSplitAtModal, setShowSplitAtModal] = useState(false);
  const [splitAtPage, setSplitAtPage] = useState(1);
  const [showInsertModal, setShowInsertModal] = useState(false);
  const [insertFilePath, setInsertFilePath] = useState<string>('');
  const [insertAtPage, setInsertAtPage] = useState<number>(0);
  const [insertSourcePageCount, setInsertSourcePageCount] = useState<number | null>(null);
  const [showMergeModal, setShowMergeModal] = useState(false);
  const [mergeFilePath, setMergeFilePath] = useState('');
  const [mergeSourcePageCount, setMergeSourcePageCount] = useState<number | null>(null);

  const [showRotateModal, setShowRotateModal] = useState(false);
  const [rotateDirection, setRotateDirection] = useState<RotateDirection>('cw');

  return {
    showSaveAsModal, setShowSaveAsModal,
    saveAsPath, setSaveAsPath,
    showMarkdownSaveAsModal, setShowMarkdownSaveAsModal,
    markdownSaveAsPath, setMarkdownSaveAsPath,
    nativeDialogs, setNativeDialogs,
    showSummaryModal, setShowSummaryModal,
    pdfSummary, setPdfSummary,
    showOpenModal, setShowOpenModal,
    openFilePath, setOpenFilePath,
    showPrintDialog, setShowPrintDialog,
    recentPdfs, setRecentPdfs,
    lastBrowserDir, setLastBrowserDir,
    showDeleteModal, setShowDeleteModal,
    deletePageInput, setDeletePageInput,
    showPageNumbersModal, setShowPageNumbersModal,
    pageNumbersPrefix, setPageNumbersPrefix,
    showWatermarkModal, setShowWatermarkModal,
    watermarkText, setWatermarkText,
    showCropModal, setShowCropModal,
    cropMarginTop, setCropMarginTop,
    cropMarginRight, setCropMarginRight,
    cropMarginBottom, setCropMarginBottom,
    cropMarginLeft, setCropMarginLeft,
    showFlattenModal, setShowFlattenModal,
    showAddBookmarkModal, setShowAddBookmarkModal,
    bookmarkTitle, setBookmarkTitle,
    showRenameBookmarkModal, setShowRenameBookmarkModal,
    renameBookmarkIndex, setRenameBookmarkIndex,
    renameBookmarkTitle, setRenameBookmarkTitle,
    cropApplyAll, setCropApplyAll,
    pageSizes, setPageSizes,
    showPageHeaderModal, setShowPageHeaderModal,
    pageHeaderText, setPageHeaderText,
    showInsertImagePageModal, setShowInsertImagePageModal,
    insertImagePagePath, setInsertImagePagePath,
    insertImageAtIndex, setInsertImageAtIndex,
    showExportPagePdfModal, setShowExportPagePdfModal,
    exportPagePdfPath, setExportPagePdfPath,
    showExportPagesPdfModal, setShowExportPagesPdfModal,
    exportPagesPdfOutputDir, setExportPagesPdfOutputDir,
    showPageFooterModal, setShowPageFooterModal,
    pageFooterText, setPageFooterText,
    showSwapPagesModal, setShowSwapPagesModal,
    swapPageA, setSwapPageA,
    swapPageB, setSwapPageB,
    showBatesNumberModal, setShowBatesNumberModal,
    batesPrefix, setBatesPrefix,
    batesStartNumber, setBatesStartNumber,
    batesDigits, setBatesDigits,
    batesPosition, setBatesPosition,
    showApplyRedactionsModal, setShowApplyRedactionsModal,
    applyRedactionsOcrAfter, setApplyRedactionsOcrAfter,
    showDeleteRangeModal, setShowDeleteRangeModal,
    showDuplicateRangeModal, setShowDuplicateRangeModal,
    showKeepRangeModal, setShowKeepRangeModal,
    showMoveRangeModal, setShowMoveRangeModal,
    moveRangeToIndex, setMoveRangeToIndex,
    showExpandMarginsModal, setShowExpandMarginsModal,
    expandMarginTop, setExpandMarginTop,
    expandMarginRight, setExpandMarginRight,
    expandMarginBottom, setExpandMarginBottom,
    expandMarginLeft, setExpandMarginLeft,
    showShrinkMarginsModal, setShowShrinkMarginsModal,
    shrinkMarginTop, setShrinkMarginTop,
    shrinkMarginRight, setShrinkMarginRight,
    shrinkMarginBottom, setShrinkMarginBottom,
    shrinkMarginLeft, setShrinkMarginLeft,
    showDeleteNthModal, setShowDeleteNthModal,
    deleteNthValue, setDeleteNthValue,
    showReverseRangeModal, setShowReverseRangeModal,
    showInsertBlankPagesModal, setShowInsertBlankPagesModal,
    insertBlankCount, setInsertBlankCount,
    insertBlankAtIndex, setInsertBlankAtIndex,
    showCropRangeModal, setShowCropRangeModal,
    showParityRangeModal, setShowParityRangeModal,
    parityRangeCommand, setParityRangeCommand,
    parityRangeOutputPath, setParityRangeOutputPath,
    showSplitModal, setShowSplitModal,
    splitRanges, setSplitRanges,
    showExtractModal, setShowExtractModal,
    extractOutputPath, setExtractOutputPath,
    showExportPngModal, setShowExportPngModal,
    pngExportOutputPath, setPngExportOutputPath,
    imageExportFormat, setImageExportFormat,
    showReplacePageModal, setShowReplacePageModal,
    replaceSourcePath, setReplaceSourcePath,
    replaceSourcePage, setReplaceSourcePage,
    replaceSourcePageCount, setReplaceSourcePageCount,
    showInterleaveModal, setShowInterleaveModal,
    interleaveFilePath, setInterleaveFilePath,
    interleaveSourcePageCount, setInterleaveSourcePageCount,
    showPageSizeModal, setShowPageSizeModal,
    pageSizePreset, setPageSizePreset,
    showPrependModal, setShowPrependModal,
    prependFilePath, setPrependFilePath,
    prependSourcePageCount, setPrependSourcePageCount,
    showSplitEveryModal, setShowSplitEveryModal,
    splitEveryN, setSplitEveryN,
    showPageBorderModal, setShowPageBorderModal,
    pageBorderInset, setPageBorderInset,
    showBookmarkAllModal, setShowBookmarkAllModal,
    bookmarkAllPrefix, setBookmarkAllPrefix,
    showExtractOddModal, setShowExtractOddModal,
    extractOddOutputPath, setExtractOddOutputPath,
    showExtractEvenModal, setShowExtractEvenModal,
    extractEvenOutputPath, setExtractEvenOutputPath,
    showSplitAtModal, setShowSplitAtModal,
    splitAtPage, setSplitAtPage,
    showInsertModal, setShowInsertModal,
    insertFilePath, setInsertFilePath,
    insertAtPage, setInsertAtPage,
    insertSourcePageCount, setInsertSourcePageCount,
    showMergeModal, setShowMergeModal,
    mergeFilePath, setMergeFilePath,
    mergeSourcePageCount, setMergeSourcePageCount,
    showRotateModal, setShowRotateModal,
    rotateDirection, setRotateDirection,
  };
}

/** Canonical alias for this hook's state shape. */
export type ModalState = ReturnType<typeof useAppModalState>;
