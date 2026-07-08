import parityBatchCommands from '../parity_batch_commands.json';
import { AddBookmarkModal } from './AddBookmarkModal';
import { ApplyRedactionsModal } from './ApplyRedactionsModal';
import { BatesNumberModal } from './BatesNumberModal';
import { AddFormFieldModal } from './AddFormFieldModal';
import { BookmarkAllModal } from './BookmarkAllModal';
import { CropModal } from './CropModal';
import { CropRangeModal } from './CropRangeModal';
import { DecryptModal } from './DecryptModal';
import { DeleteNthModal } from './DeleteNthModal';
import { DeletePageModal } from './DeletePageModal';
import { DeleteRangeModal } from './DeleteRangeModal';
import { DocumentSummaryModal } from './DocumentSummaryModal';
import { DuplicateRangeModal } from './DuplicateRangeModal';
import { ExpandMarginsModal } from './ExpandMarginsModal';
import { ExportPagePdfModal } from './ExportPagePdfModal';
import { ExportPagesPdfModal } from './ExportPagesPdfModal';
import { ExportPngModal } from './ExportPngModal';
import { ExtractEvenPagesModal } from './ExtractEvenPagesModal';
import { ExtractOddPagesModal } from './ExtractOddPagesModal';
import { ExtractPagesModal } from './ExtractPagesModal';
import { FlattenModal } from './FlattenModal';
import { ImageInsertModal } from './ImageInsertModal';
import { InsertBlankPagesModal } from './InsertBlankPagesModal';
import { InsertImagePageModal } from './InsertImagePageModal';
import { InsertPdfModal } from './InsertPdfModal';
import { InterleaveModal } from './InterleaveModal';
import { KeepRangeModal } from './KeepRangeModal';
import { MarkdownSaveAsModal } from './MarkdownSaveAsModal';
import { MergePdfModal } from './MergePdfModal';
import { MetadataModal } from './MetadataModal';
import { MoveRangeModal } from './MoveRangeModal';
import { OpenPdfModal } from './OpenPdfModal';
import { PageBorderModal } from './PageBorderModal';
import { PageEditsModal } from './PageEditsModal';
import { PageFooterModal } from './PageFooterModal';
import { PageHeaderModal } from './PageHeaderModal';
import { PageNumbersModal } from './PageNumbersModal';
import { PageSizeModal } from './PageSizeModal';
import { PageTextModal } from './PageTextModal';
import { ParityRangeModal } from './ParityRangeModal';
import { PasswordModal } from './PasswordModal';
import { PdfBrowserModal } from './PdfBrowserModal';
import { PrependModal } from './PrependModal';
import { PrintDialog } from './PrintDialog';
import { ProtectPdfModal } from './ProtectPdfModal';
import { RenameBookmarkModal } from './RenameBookmarkModal';
import { ReplacePageModal } from './ReplacePageModal';
import { ReverseRangeModal } from './ReverseRangeModal';
import { RotateModal } from './RotateModal';
import { SaveAsModal } from './SaveAsModal';
import { SearchModal } from './SearchModal';
import { ShrinkMarginsModal } from './ShrinkMarginsModal';
import { SignPdfModal } from './SignPdfModal';
import { SplitAtModal } from './SplitAtModal';
import { SplitEveryModal } from './SplitEveryModal';
import { SplitPdfModal } from './SplitPdfModal';
import { StickyNoteModal } from './StickyNoteModal';
import { SwapPagesModal } from './SwapPagesModal';
import { TesseractReminderModal } from './TesseractReminderModal';
import { UnsavedChangesModal } from './UnsavedChangesModal';
import { UpdateModal } from './UpdateModal';
import { WatermarkModal } from './WatermarkModal';
import type { AppModalsContext } from './appModalsContext';

type AppModalsProps = {
  ctx: AppModalsContext;
};

export function AppModals({ ctx: rawCtx }: AppModalsProps) {
  // Wide context object assembled in App.tsx; modal wiring is mechanical.
  const ctx = rawCtx;
  return (
    <>
      {ctx.showOpenModal && (
        <OpenPdfModal
          filePath={ctx.openFilePath}
          nativeDialogs={ctx.nativeDialogs}
          recentPdfs={ctx.recentPdfs}
          fileNameFromPath={ctx.fileNameFromPath}
          onFilePathChange={ctx.setOpenFilePath}
          onClose={() => ctx.setShowOpenModal(false)}
          onOpenPdfView={ctx.handleOpenPdfInPdfView}
          onOpenBirdsEye={ctx.handleOpenPdfInBirdsEye}
          openDirectlyInBirdsEye={ctx.openDirectlyInBirdsEye}
          onChooseNative={ctx.chooseOpenPdfNative}
          onBrowse={() => ctx.openPdfBrowser('open')}
        />
      )}

      {ctx.showNoteModal && (
        <StickyNoteModal
          noteDraft={ctx.noteDraft}
          onNoteDraftChange={ctx.setNoteDraft}
          onClose={ctx.exitNoteMode}
          onSubmit={ctx.submitTextNote}
        />
      )}

      {ctx.showDeleteModal && ctx.pageCount !== null && (
        <DeletePageModal
          deletePageInput={ctx.deletePageInput}
          currentPage={ctx.currentPage}
          pageCount={ctx.pageCount}
          onDeletePageInputChange={ctx.setDeletePageInput}
          onClose={() => ctx.setShowDeleteModal(false)}
          onDelete={ctx.handleDeletePage}
        />
      )}

      {ctx.showExportPngModal && (
        <ExportPngModal
          range={ctx.pngExportRange}
          pageCount={ctx.pageCount}
          currentPage={ctx.currentPage}
          format={ctx.imageExportFormat}
          outputPath={ctx.pngExportOutputPath}
          nativeDialogs={ctx.nativeDialogs}
          defaultOutputPath={ctx.defaultImageExportOutput}
          onFormatChange={ctx.setImageExportFormat}
          onOutputPathChange={ctx.setPngExportOutputPath}
          onClose={() => ctx.setShowExportPngModal(false)}
          onChooseOutputNative={ctx.chooseExportPngOutputNative}
          onExport={ctx.handleExportPng}
          onExportOdd={ctx.handleExportOddPagesImage}
          onExportEven={ctx.handleExportEvenPagesImage}
        />
      )}

      {ctx.showDeleteRangeModal && (
        <DeleteRangeModal
          startPage={ctx.deleteRange.startPage}
          endPage={ctx.deleteRange.endPage}
          pageCount={ctx.pageCount}
          onStartChange={ctx.deleteRange.setStartPage}
          onEndChange={ctx.deleteRange.setEndPage}
          onClose={() => ctx.setShowDeleteRangeModal(false)}
          onDelete={ctx.handleDeletePageRange}
        />
      )}

      {ctx.showPageNumbersModal && (
        <PageNumbersModal
          range={ctx.pageNumbersRange}
          pageCount={ctx.pageCount}
          prefix={ctx.pageNumbersPrefix}
          onPrefixChange={ctx.setPageNumbersPrefix}
          onClose={() => ctx.setShowPageNumbersModal(false)}
          onApply={ctx.handleAddPageNumbers}
          onApplyOdd={ctx.handleAddPageNumbersOddPages}
          onApplyEven={ctx.handleAddPageNumbersEvenPages}
        />
      )}

      {ctx.showBatesNumberModal && (
        <BatesNumberModal
          startPage={ctx.batesRange.startPage}
          endPage={ctx.batesRange.endPage}
          pageCount={ctx.pageCount}
          prefix={ctx.batesPrefix}
          startNumber={ctx.batesStartNumber}
          digits={ctx.batesDigits}
          position={ctx.batesPosition}
          onStartChange={ctx.batesRange.setStartPage}
          onEndChange={ctx.batesRange.setEndPage}
          onPrefixChange={ctx.setBatesPrefix}
          onStartNumberChange={ctx.setBatesStartNumber}
          onDigitsChange={ctx.setBatesDigits}
          onPositionChange={ctx.setBatesPosition}
          onClose={() => ctx.setShowBatesNumberModal(false)}
          onApply={ctx.handleAddBatesNumbers}
        />
      )}

      {ctx.showApplyRedactionsModal && (
        <ApplyRedactionsModal
          ocrAvailable={ctx.ocrAvailable}
          ocrAfter={ctx.applyRedactionsOcrAfter}
          onOcrAfterChange={ctx.setApplyRedactionsOcrAfter}
          onClose={() => ctx.setShowApplyRedactionsModal(false)}
          onApply={ctx.handleApplyRedactions}
          onOpenTesseractGuide={ctx.openTesseractGuide}
        />
      )}

      {ctx.showWatermarkModal && (
        <WatermarkModal
          range={ctx.watermarkRange}
          pageCount={ctx.pageCount}
          text={ctx.watermarkText}
          onTextChange={ctx.setWatermarkText}
          onClose={() => ctx.setShowWatermarkModal(false)}
          onApply={ctx.handleAddWatermark}
          onApplyOdd={ctx.handleAddWatermarkOddPages}
          onApplyEven={ctx.handleAddWatermarkEvenPages}
        />
      )}

      {ctx.showCropModal && (
        <CropModal
          currentPage={ctx.currentPage}
          applyAll={ctx.cropApplyAll}
          pageWidth={ctx.pageSizes[ctx.currentPage]?.width}
          pageHeight={ctx.pageSizes[ctx.currentPage]?.height}
          margins={{
            top: ctx.cropMarginTop,
            right: ctx.cropMarginRight,
            bottom: ctx.cropMarginBottom,
            left: ctx.cropMarginLeft,
          }}
          onApplyAllChange={ctx.setCropApplyAll}
          onMarginsChange={(m) => {
            ctx.setCropMarginTop(m.top);
            ctx.setCropMarginRight(m.right);
            ctx.setCropMarginBottom(m.bottom);
            ctx.setCropMarginLeft(m.left);
          }}
          onClose={() => ctx.setShowCropModal(false)}
          onClearPageCrop={ctx.handleClearPageCrop}
          onClearAllCrops={ctx.handleClearAllCrops}
          onClearOddCrops={ctx.handleClearCropOddPages}
          onClearEvenCrops={ctx.handleClearCropEvenPages}
          onCrop={ctx.handleCropPage}
        />
      )}

      {ctx.showDuplicateRangeModal && (
        <DuplicateRangeModal
          startPage={ctx.duplicateRange.startPage}
          endPage={ctx.duplicateRange.endPage}
          pageCount={ctx.pageCount}
          onStartChange={ctx.duplicateRange.setStartPage}
          onEndChange={ctx.duplicateRange.setEndPage}
          onClose={() => ctx.setShowDuplicateRangeModal(false)}
          onDuplicate={ctx.handleDuplicatePageRange}
          onDuplicateBefore={ctx.handleDuplicatePageRangeBefore}
          onDuplicateToStart={ctx.handleDuplicatePageRangeToStart}
          onDuplicateToEnd={ctx.handleDuplicatePageRangeToEnd}
        />
      )}

      {ctx.showFlattenModal && (
        <FlattenModal
          range={ctx.flattenRange}
          pageCount={ctx.pageCount}
          onClose={() => ctx.setShowFlattenModal(false)}
          onFlatten={ctx.handleFlattenAnnotations}
        />
      )}

      {ctx.showAddBookmarkModal && (
        <AddBookmarkModal
          currentPage={ctx.currentPage}
          title={ctx.bookmarkTitle}
          onTitleChange={ctx.setBookmarkTitle}
          onClose={() => ctx.setShowAddBookmarkModal(false)}
          onAdd={ctx.handleAddBookmark}
        />
      )}

      {ctx.showPageHeaderModal && (
        <PageHeaderModal
          range={ctx.pageHeaderRange}
          pageCount={ctx.pageCount}
          text={ctx.pageHeaderText}
          onTextChange={ctx.setPageHeaderText}
          onClose={() => ctx.setShowPageHeaderModal(false)}
          onApply={ctx.handleAddPageHeader}
          onApplyOdd={ctx.handleAddPageHeaderOddPages}
          onApplyEven={ctx.handleAddPageHeaderEvenPages}
        />
      )}

      {ctx.showPageFooterModal && (
        <PageFooterModal
          range={ctx.pageFooterRange}
          pageCount={ctx.pageCount}
          text={ctx.pageFooterText}
          onTextChange={ctx.setPageFooterText}
          onClose={() => ctx.setShowPageFooterModal(false)}
          onApply={ctx.handleAddPageFooter}
          onApplyOdd={ctx.handleAddPageFooterOddPages}
          onApplyEven={ctx.handleAddPageFooterEvenPages}
        />
      )}

      {ctx.showSwapPagesModal && (
        <SwapPagesModal
          pageA={ctx.swapPageA}
          pageB={ctx.swapPageB}
          pageCount={ctx.pageCount}
          onPageAChange={ctx.setSwapPageA}
          onPageBChange={ctx.setSwapPageB}
          onClose={() => ctx.setShowSwapPagesModal(false)}
          onSwap={ctx.handleSwapPages}
        />
      )}

      {ctx.showReplacePageModal && (
        <ReplacePageModal
          currentPage={ctx.currentPage}
          sourcePath={ctx.replaceSourcePath}
          sourcePage={ctx.replaceSourcePage}
          sourcePageCount={ctx.replaceSourcePageCount}
          onSourcePathChange={(value) =>
            void ctx.handleReplaceSourcePathChange(value)
          }
          onSourcePageChange={ctx.setReplaceSourcePage}
          onBrowse={() => ctx.openPdfBrowser('replace')}
          onClose={() => ctx.setShowReplacePageModal(false)}
          onReplace={ctx.handleReplacePage}
        />
      )}

      {ctx.showInterleaveModal && (
        <InterleaveModal
          sourcePath={ctx.interleaveFilePath}
          sourcePageCount={ctx.interleaveSourcePageCount}
          startPage={ctx.interleaveRange.startPage}
          endPage={ctx.interleaveRange.endPage}
          onSourcePathChange={(value) =>
            void ctx.handleInterleaveSourcePathChange(value)
          }
          onStartChange={ctx.interleaveRange.setStartPage}
          onEndChange={ctx.interleaveRange.setEndPage}
          onBrowse={() => ctx.openPdfBrowser('interleave')}
          onClose={() => ctx.setShowInterleaveModal(false)}
          onInterleave={ctx.handleInterleavePdf}
        />
      )}

      {ctx.showPageSizeModal && (
        <PageSizeModal
          range={ctx.pageSizeRange}
          pageCount={ctx.pageCount}
          preset={ctx.pageSizePreset}
          onPresetChange={ctx.setPageSizePreset}
          onClose={() => ctx.setShowPageSizeModal(false)}
          onApply={ctx.handleSetPageSize}
          onApplyOdd={ctx.handleSetPageSizeOddPages}
          onApplyEven={ctx.handleSetPageSizeEvenPages}
        />
      )}

      {ctx.showExportPagesPdfModal && (
        <ExportPagesPdfModal
          range={ctx.exportPagesPdfRange}
          pageCount={ctx.pageCount}
          outputDir={ctx.exportPagesPdfOutputDir}
          onOutputDirChange={ctx.setExportPagesPdfOutputDir}
          onClose={() => ctx.setShowExportPagesPdfModal(false)}
          onExport={ctx.handleExportPagesPdf}
          onExportOdd={ctx.handleExportOddPagesAsPdf}
          onExportEven={ctx.handleExportEvenPagesAsPdf}
        />
      )}

      {ctx.showRotateModal && (
        <RotateModal
          scope={ctx.rotateRange.scope}
          startPage={ctx.rotateRange.startPage}
          endPage={ctx.rotateRange.endPage}
          pageCount={ctx.pageCount}
          direction={ctx.rotateDirection}
          onClose={() => ctx.setShowRotateModal(false)}
          onScopeChange={ctx.rotateRange.setScope}
          onStartChange={ctx.rotateRange.setStartPage}
          onEndChange={ctx.rotateRange.setEndPage}
          onDirectionChange={ctx.setRotateDirection}
          onApply={ctx.handleApplyRotateModal}
        />
      )}

      {ctx.showKeepRangeModal && (
        <KeepRangeModal
          startPage={ctx.keepRange.startPage}
          endPage={ctx.keepRange.endPage}
          pageCount={ctx.pageCount}
          onStartChange={ctx.keepRange.setStartPage}
          onEndChange={ctx.keepRange.setEndPage}
          onClose={() => ctx.setShowKeepRangeModal(false)}
          onKeep={ctx.handleKeepPageRange}
        />
      )}

      {ctx.showMoveRangeModal && (
        <MoveRangeModal
          startPage={ctx.moveRange.startPage}
          endPage={ctx.moveRange.endPage}
          targetIndex={ctx.moveRangeToIndex}
          pageCount={ctx.pageCount}
          onStartChange={ctx.moveRange.setStartPage}
          onEndChange={ctx.moveRange.setEndPage}
          onTargetChange={ctx.setMoveRangeToIndex}
          onClose={() => ctx.setShowMoveRangeModal(false)}
          onMoveToStart={ctx.handleMovePageRangeToStart}
          onMoveToEnd={ctx.handleMovePageRangeToEnd}
          onMove={ctx.handleMovePageRange}
        />
      )}

      {ctx.showPrependModal && (
        <PrependModal
          sourcePath={ctx.prependFilePath}
          sourcePageCount={ctx.prependSourcePageCount}
          startPage={ctx.prependRange.startPage}
          endPage={ctx.prependRange.endPage}
          onSourcePathChange={(value) =>
            void ctx.handlePrependSourcePathChange(value)
          }
          onStartChange={ctx.prependRange.setStartPage}
          onEndChange={ctx.prependRange.setEndPage}
          onBrowse={() => ctx.openPdfBrowser('prepend')}
          onClose={() => ctx.setShowPrependModal(false)}
          onPrepend={ctx.handlePrependPdf}
        />
      )}

      {ctx.showSplitEveryModal && (
        <SplitEveryModal
          everyN={ctx.splitEveryN}
          onEveryNChange={ctx.setSplitEveryN}
          onClose={() => ctx.setShowSplitEveryModal(false)}
          onSplit={ctx.handleSplitEveryN}
        />
      )}

      {ctx.showPageBorderModal && (
        <PageBorderModal
          range={ctx.pageBorderRange}
          pageCount={ctx.pageCount}
          inset={ctx.pageBorderInset}
          onInsetChange={ctx.setPageBorderInset}
          onClose={() => ctx.setShowPageBorderModal(false)}
          onApply={ctx.handleAddPageBorder}
          onApplyOdd={ctx.handleAddPageBorderOddPages}
          onApplyEven={ctx.handleAddPageBorderEvenPages}
        />
      )}

      {ctx.showBookmarkAllModal && (
        <BookmarkAllModal
          prefix={ctx.bookmarkAllPrefix}
          onPrefixChange={ctx.setBookmarkAllPrefix}
          onClose={() => ctx.setShowBookmarkAllModal(false)}
          onBookmarkOdd={ctx.handleBookmarkOddPages}
          onBookmarkEven={ctx.handleBookmarkEvenPages}
          onBookmarkAll={ctx.handleBookmarkAllPages}
        />
      )}

      {ctx.showShrinkMarginsModal && (
        <ShrinkMarginsModal
          range={ctx.shrinkMarginsRange}
          pageCount={ctx.pageCount}
          margins={{
            top: ctx.shrinkMarginTop,
            right: ctx.shrinkMarginRight,
            bottom: ctx.shrinkMarginBottom,
            left: ctx.shrinkMarginLeft,
          }}
          onMarginsChange={(m) => {
            ctx.setShrinkMarginTop(m.top);
            ctx.setShrinkMarginRight(m.right);
            ctx.setShrinkMarginBottom(m.bottom);
            ctx.setShrinkMarginLeft(m.left);
          }}
          onClose={() => ctx.setShowShrinkMarginsModal(false)}
          onShrink={ctx.handleShrinkPageMargins}
          onShrinkOdd={ctx.handleShrinkOddPages}
          onShrinkEven={ctx.handleShrinkEvenPages}
        />
      )}

      {ctx.showSplitAtModal && (
        <SplitAtModal
          splitAtPage={ctx.splitAtPage}
          pageCount={ctx.pageCount}
          onSplitAtPageChange={ctx.setSplitAtPage}
          onClose={() => ctx.setShowSplitAtModal(false)}
          onSplit={ctx.handleSplitPdfAtPage}
        />
      )}

      {ctx.showDeleteNthModal && (
        <DeleteNthModal
          nth={ctx.deleteNthValue}
          onNthChange={ctx.setDeleteNthValue}
          onClose={() => ctx.setShowDeleteNthModal(false)}
          onDelete={ctx.handleDeleteEveryNthPage}
        />
      )}

      {ctx.showExtractOddModal && (
        <ExtractOddPagesModal
          outputPath={ctx.extractOddOutputPath}
          onOutputPathChange={ctx.setExtractOddOutputPath}
          onClose={() => ctx.setShowExtractOddModal(false)}
          onExtract={ctx.handleExtractOddPages}
        />
      )}

      {ctx.showExtractEvenModal && (
        <ExtractEvenPagesModal
          outputPath={ctx.extractEvenOutputPath}
          onOutputPathChange={ctx.setExtractEvenOutputPath}
          onClose={() => ctx.setShowExtractEvenModal(false)}
          onExtract={ctx.handleExtractEvenPages}
        />
      )}

      {ctx.showExpandMarginsModal && (
        <ExpandMarginsModal
          range={ctx.expandMarginsRange}
          pageCount={ctx.pageCount}
          margins={{
            top: ctx.expandMarginTop,
            right: ctx.expandMarginRight,
            bottom: ctx.expandMarginBottom,
            left: ctx.expandMarginLeft,
          }}
          onMarginsChange={(m) => {
            ctx.setExpandMarginTop(m.top);
            ctx.setExpandMarginRight(m.right);
            ctx.setExpandMarginBottom(m.bottom);
            ctx.setExpandMarginLeft(m.left);
          }}
          onClose={() => ctx.setShowExpandMarginsModal(false)}
          onExpand={ctx.handleExpandPageMargins}
          onExpandOdd={ctx.handleExpandOddPages}
          onExpandEven={ctx.handleExpandEvenPages}
        />
      )}

      {ctx.showReverseRangeModal && (
        <ReverseRangeModal
          startPage={ctx.reverseRange.startPage}
          endPage={ctx.reverseRange.endPage}
          pageCount={ctx.pageCount}
          onStartChange={ctx.reverseRange.setStartPage}
          onEndChange={ctx.reverseRange.setEndPage}
          onClose={() => ctx.setShowReverseRangeModal(false)}
          onReverse={ctx.handleReversePageRange}
        />
      )}

      {ctx.showInsertBlankPagesModal && (
        <InsertBlankPagesModal
          atIndex={ctx.insertBlankAtIndex}
          count={ctx.insertBlankCount}
          pageCount={ctx.pageCount}
          onAtIndexChange={ctx.setInsertBlankAtIndex}
          onCountChange={ctx.setInsertBlankCount}
          onClose={() => ctx.setShowInsertBlankPagesModal(false)}
          onInsert={ctx.handleInsertBlankPages}
        />
      )}

      {ctx.showParityRangeModal && (
        <ParityRangeModal
          commands={parityBatchCommands as string[]}
          command={ctx.parityRangeCommand}
          outputPath={ctx.parityRangeOutputPath}
          startPage={ctx.parityRange.startPage}
          endPage={ctx.parityRange.endPage}
          pageCount={ctx.pageCount}
          onCommandChange={ctx.setParityRangeCommand}
          onOutputPathChange={ctx.setParityRangeOutputPath}
          onStartChange={ctx.parityRange.setStartPage}
          onEndChange={ctx.parityRange.setEndPage}
          onClose={() => ctx.setShowParityRangeModal(false)}
          onRun={ctx.handleParityRangeAction}
        />
      )}

      {ctx.showCropRangeModal && (
        <CropRangeModal
          startPage={ctx.cropRange.startPage}
          endPage={ctx.cropRange.endPage}
          pageCount={ctx.pageCount}
          margins={{
            top: ctx.cropMarginTop,
            right: ctx.cropMarginRight,
            bottom: ctx.cropMarginBottom,
            left: ctx.cropMarginLeft,
          }}
          onStartChange={ctx.cropRange.setStartPage}
          onEndChange={ctx.cropRange.setEndPage}
          onMarginsChange={(m) => {
            ctx.setCropMarginTop(m.top);
            ctx.setCropMarginRight(m.right);
            ctx.setCropMarginBottom(m.bottom);
            ctx.setCropMarginLeft(m.left);
          }}
          onClose={() => ctx.setShowCropRangeModal(false)}
          onCropOdd={ctx.handleCropOddPages}
          onCropEven={ctx.handleCropEvenPages}
          onCrop={ctx.handleCropPageRange}
        />
      )}

      {ctx.showDecryptModal && (
        <DecryptModal
          password={ctx.decryptPassword}
          onPasswordChange={ctx.setDecryptPassword}
          onClose={() => ctx.setShowDecryptModal(false)}
          onDecrypt={ctx.handleRemovePdfPassword}
        />
      )}

      {ctx.showInsertImagePageModal && (
        <InsertImagePageModal
          atIndex={ctx.insertImageAtIndex}
          imagePath={ctx.insertImagePagePath}
          pageCount={ctx.pageCount}
          onAtIndexChange={ctx.setInsertImageAtIndex}
          onImagePathChange={ctx.setInsertImagePagePath}
          onClose={() => ctx.setShowInsertImagePageModal(false)}
          onInsert={ctx.handleInsertImagePage}
        />
      )}

      {ctx.showExportPagePdfModal && (
        <ExportPagePdfModal
          currentPage={ctx.currentPage}
          outputPath={ctx.exportPagePdfPath}
          onOutputPathChange={ctx.setExportPagePdfPath}
          onClose={() => ctx.setShowExportPagePdfModal(false)}
          onExport={ctx.handleExportPagePdf}
        />
      )}

      {ctx.showRenameBookmarkModal && (
        <RenameBookmarkModal
          title={ctx.renameBookmarkTitle}
          onTitleChange={ctx.setRenameBookmarkTitle}
          onClose={() => ctx.setShowRenameBookmarkModal(false)}
          onRename={ctx.handleRenameBookmark}
        />
      )}

      {ctx.showExtractModal && (
        <ExtractPagesModal
          startPage={ctx.extractRange.startPage}
          endPage={ctx.extractRange.endPage}
          pageCount={ctx.pageCount}
          outputPath={ctx.extractOutputPath}
          nativeDialogs={ctx.nativeDialogs}
          onStartChange={(start) => {
            ctx.extractRange.setStartPage(start);
            ctx.setExtractOutputPath(
              ctx.defaultExtractOutputPath(start, ctx.extractRange.endPage)
            );
          }}
          onEndChange={(end) => {
            ctx.extractRange.setEndPage(end);
            ctx.setExtractOutputPath(
              ctx.defaultExtractOutputPath(ctx.extractRange.startPage, end)
            );
          }}
          onOutputPathChange={ctx.setExtractOutputPath}
          onClose={() => ctx.setShowExtractModal(false)}
          onChooseOutputNative={ctx.chooseExtractOutputNative}
          onExtract={ctx.handleExtractPdf}
        />
      )}

      {ctx.showSplitModal && (
        <SplitPdfModal
          splitRanges={ctx.splitRanges}
          pageCount={ctx.pageCount}
          onSplitRangesChange={ctx.setSplitRanges}
          onClose={() => ctx.setShowSplitModal(false)}
          onSplit={ctx.handleSplitPdf}
        />
      )}

      {ctx.showAddFormFieldModal && (
        <AddFormFieldModal
          fieldKind={ctx.newFormFieldKind}
          fieldName={ctx.newFormFieldName}
          fieldOptions={ctx.newFormFieldOptions}
          checkboxChecked={ctx.newFormCheckboxChecked}
          radioGroup={ctx.newFormRadioGroup}
          radioOption={ctx.newFormRadioOption}
          onFieldKindChange={ctx.setNewFormFieldKind}
          onFieldNameChange={ctx.setNewFormFieldName}
          onFieldOptionsChange={ctx.setNewFormFieldOptions}
          onCheckboxCheckedChange={ctx.setNewFormCheckboxChecked}
          onRadioGroupChange={ctx.setNewFormRadioGroup}
          onRadioOptionChange={ctx.setNewFormRadioOption}
          onClose={() => ctx.setShowAddFormFieldModal(false)}
          onConfirm={ctx.confirmAddFormField}
        />
      )}

      {ctx.showImageInsertModal && (
        <ImageInsertModal
          imagePath={ctx.imageSourceDraft}
          onImagePathChange={ctx.setImageSourceDraft}
          onClose={() => ctx.setShowImageInsertModal(false)}
          onConfirm={ctx.confirmImageSource}
        />
      )}

      {ctx.showSearchModal && (
        <SearchModal
          inputRef={ctx.searchInputRef}
          query={ctx.searchQuery}
          matchCase={ctx.searchMatchCase}
          wholeWord={ctx.searchWholeWord}
          results={ctx.searchResults}
          resultIndex={ctx.searchResultIndex}
          onQueryChange={ctx.setSearchQuery}
          onMatchCaseChange={ctx.setSearchMatchCase}
          onWholeWordChange={ctx.setSearchWholeWord}
          onClose={ctx.closeSearchModal}
          onFind={ctx.runPdfSearch}
          onStepMatch={ctx.stepSearchMatch}
        />
      )}

      {ctx.showMergeModal && (
        <MergePdfModal
          sourcePath={ctx.mergeFilePath}
          sourcePageCount={ctx.mergeSourcePageCount}
          pageCount={ctx.pageCount}
          startPage={ctx.mergeRange.startPage}
          endPage={ctx.mergeRange.endPage}
          nativeDialogs={ctx.nativeDialogs}
          onSourcePathChange={ctx.setMergeFilePath}
          onStartChange={ctx.mergeRange.setStartPage}
          onEndChange={ctx.mergeRange.setEndPage}
          onClose={() => {
            ctx.setShowMergeModal(false);
            ctx.setMergeFilePath('');
          }}
          onChooseNative={ctx.chooseMergePdfNative}
          onBrowse={() => ctx.openPdfBrowser('merge')}
          onMerge={ctx.handleMergePdf}
        />
      )}

      {ctx.showInsertModal && (
        <InsertPdfModal
          sourcePath={ctx.insertFilePath}
          sourcePageCount={ctx.insertSourcePageCount}
          pageCount={ctx.pageCount}
          insertAtPage={ctx.insertAtPage}
          startPage={ctx.insertRange.startPage}
          endPage={ctx.insertRange.endPage}
          nativeDialogs={ctx.nativeDialogs}
          onSourcePathChange={ctx.setInsertFilePath}
          onInsertAtPageChange={ctx.setInsertAtPage}
          onStartChange={ctx.insertRange.setStartPage}
          onEndChange={ctx.insertRange.setEndPage}
          onClose={() => {
            ctx.setShowInsertModal(false);
            ctx.setInsertFilePath('');
          }}
          onChooseNative={ctx.chooseInsertPdfNative}
          onBrowse={() => ctx.openPdfBrowser('insert')}
          onInsert={ctx.handleInsertPdf}
        />
      )}

      {ctx.showPageTextModal && (
        <PageTextModal
          editing={ctx.editingTextIndex !== null}
          text={ctx.pageTextDraft}
          fontSize={ctx.pageTextFontSize}
          onTextChange={ctx.setPageTextDraft}
          onFontSizeChange={ctx.setPageTextFontSize}
          onClose={ctx.closePageTextModal}
          onSave={ctx.submitPageText}
        />
      )}

      {ctx.showPageEditsModal && (
        <PageEditsModal
          currentPage={ctx.currentPage}
          textEdits={ctx.pageTextEdits}
          vectorEdits={ctx.pageVectorEdits}
          onClose={() => ctx.setShowPageEditsModal(false)}
          onEditText={ctx.startEditPageText}
          onRemoveText={ctx.removePageTextEdit}
          onRemoveVector={ctx.removePageVectorEdit}
        />
      )}

      {ctx.showSummaryModal && ctx.pdfSummary && (
        <DocumentSummaryModal
          summary={ctx.pdfSummary}
          onClose={() => ctx.setShowSummaryModal(false)}
          onCopy={ctx.handleCopySummary}
          onSave={ctx.handleSaveSummary}
        />
      )}

      {ctx.showUpdateModal && (
        <UpdateModal
          onClose={() => ctx.setShowUpdateModal(false)}
          updaterSupported={ctx.updaterSupported}
        />
      )}

      {ctx.showTesseractModal && (
        <TesseractReminderModal
          guide={ctx.tesseractInstallGuide}
          doNotRemind={ctx.tesseractDoNotRemind}
          onDoNotRemindChange={ctx.setTesseractDoNotRemind}
          onClose={ctx.closeTesseractReminderModal}
          onCopyInstallCommand={() => {
            void navigator.clipboard.writeText(
              ctx.tesseractInstallGuide.installCommand ?? ''
            );
            ctx.showToast('Install command copied');
          }}
        />
      )}

      {ctx.showMarkdownSaveAsModal && (
        <MarkdownSaveAsModal
          outputPath={ctx.markdownSaveAsPath}
          nativeDialogs={ctx.nativeDialogs}
          onOutputPathChange={ctx.setMarkdownSaveAsPath}
          onClose={() => ctx.setShowMarkdownSaveAsModal(false)}
          onChooseNative={ctx.chooseMarkdownSaveAsNative}
          onSave={ctx.handleMarkdownSaveAs}
        />
      )}

      {ctx.showPasswordModal && (
        <PasswordModal
          password={ctx.pdfPasswordDraft}
          onPasswordChange={ctx.setPdfPasswordDraft}
          onClose={ctx.closePasswordModal}
          onOpen={ctx.handleOpenEncryptedPdf}
        />
      )}

      {ctx.showMetadataModal && (
        <MetadataModal
          title={ctx.metadataTitle}
          author={ctx.metadataAuthor}
          subject={ctx.metadataSubject}
          keywords={ctx.metadataKeywords}
          creator={ctx.metadataCreator}
          producer={ctx.metadataProducer}
          creationDate={ctx.metadataCreationDate}
          modDate={ctx.metadataModDate}
          onTitleChange={ctx.setMetadataTitle}
          onAuthorChange={ctx.setMetadataAuthor}
          onSubjectChange={ctx.setMetadataSubject}
          onKeywordsChange={ctx.setMetadataKeywords}
          onCreatorChange={ctx.setMetadataCreator}
          onProducerChange={ctx.setMetadataProducer}
          onClose={() => ctx.setShowMetadataModal(false)}
          onClear={ctx.handleClearPdfMetadata}
          onApply={ctx.handleSaveMetadata}
        />
      )}

      {ctx.showSignModal && (
        <SignPdfModal
          certPath={ctx.signCertPath}
          certPassword={ctx.signCertPassword}
          reason={ctx.signReason}
          location={ctx.signLocation}
          nativeDialogs={ctx.nativeDialogs}
          onCertPathChange={ctx.setSignCertPath}
          onCertPasswordChange={ctx.setSignCertPassword}
          onReasonChange={ctx.setSignReason}
          onLocationChange={ctx.setSignLocation}
          onClose={() => ctx.setShowSignModal(false)}
          onChooseCertNative={ctx.chooseSignCertNative}
          onSign={ctx.handleSignPdf}
        />
      )}

      {ctx.showProtectModal && (
        <ProtectPdfModal
          userPassword={ctx.protectUserPassword}
          userPasswordConfirm={ctx.protectUserPasswordConfirm}
          ownerPassword={ctx.protectOwnerPassword}
          onUserPasswordChange={ctx.setProtectUserPassword}
          onUserPasswordConfirmChange={ctx.setProtectUserPasswordConfirm}
          onOwnerPasswordChange={ctx.setProtectOwnerPassword}
          onClose={() => ctx.setShowProtectModal(false)}
          onProtect={ctx.handleProtectPdf}
        />
      )}

      {ctx.showSaveAsModal && (
        <SaveAsModal
          outputPath={ctx.saveAsPath}
          nativeDialogs={ctx.nativeDialogs}
          onOutputPathChange={ctx.setSaveAsPath}
          onClose={() => ctx.setShowSaveAsModal(false)}
          onChooseNative={ctx.chooseSaveAsNative}
          onSave={ctx.handleSaveAs}
        />
      )}

      {ctx.showPrintDialog && (
        <PrintDialog
          filePath={ctx.activeFilePath}
          pageCount={ctx.pageCount ?? 0}
          currentPage={ctx.currentPage}
          onClose={() => ctx.setShowPrintDialog(false)}
          onUseSystemPrint={ctx.handleUseSystemPrint}
        />
      )}

      {ctx.showUnsavedModal && (
        <UnsavedChangesModal
          onClose={() => ctx.resolveUnsaved('cancel')}
          onChoose={ctx.resolveUnsaved}
        />
      )}

      {ctx.showBrowserModal && (
        <PdfBrowserModal
          pathInput={ctx.browserPathInput}
          listing={ctx.browserListing}
          onPathInputChange={ctx.setBrowserPathInput}
          onClose={() => ctx.setShowBrowserModal(false)}
          onCommitPath={ctx.commitBrowserPath}
          onNavigateParent={(parentDir) => void ctx.loadPdfBrowser(parentDir)}
          onEntryClick={(entry) => void ctx.handleBrowserEntryClick(entry)}
        />
      )}
    </>
  );
}
