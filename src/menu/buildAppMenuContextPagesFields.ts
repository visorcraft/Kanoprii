import type { AppMenuContext } from './types';
import type { AppMenuContextSource } from './types';
import { voidRun, voidSort } from './menuBuilders';

export function menuContextPagesFields(
  source: AppMenuContextSource
): Pick<
  AppMenuContext,
  | 'handleRotatePage'
  | 'handleRotatePageCcw'
  | 'handleResetPageRotation'
  | 'handleRotatePage180'
  | 'handleRotateAllPages'
  | 'handleRotateAllPagesCcw'
  | 'handleRotateAllPages180'
  | 'handleRotateOddPages'
  | 'handleRotateEvenPages'
  | 'handleRotateOddPagesCcw'
  | 'handleRotateEvenPagesCcw'
  | 'handleRotate180OddPages'
  | 'handleRotate180EvenPages'
  | 'handleResetRotationOddPages'
  | 'handleResetRotationEvenPages'
  | 'handleResetAllRotations'
  | 'openRotateModal'
  | 'openRotateRangeModal'
  | 'handleDuplicatePage'
  | 'handleDuplicatePageBefore'
  | 'openDuplicateRangeModal'
  | 'openParityRangeModal'
  | 'openMoveRangeModal'
  | 'openKeepRangeModal'
  | 'handleKeepOddPages'
  | 'handleKeepEvenPages'
  | 'handleDeleteOddPages'
  | 'handleDeleteEvenPages'
  | 'handleAddBlankPage'
  | 'handleAddBlankPageBefore'
  | 'openInsertBlankPagesModal'
  | 'handleInsertBlankBetweenPages'
  | 'handleInsertBlankBeforeOddPages'
  | 'handleInsertBlankBeforeEvenPages'
  | 'handleInsertBlankAfterOddPages'
  | 'handleInsertBlankAfterEvenPages'
  | 'handleMovePageToFirst'
  | 'handleMovePageToLast'
  | 'handleMovePageUp'
  | 'handleMovePageDown'
  | 'openSwapPagesModal'
  | 'handleReversePages'
  | 'openReverseRangeModal'
  | 'handleReverseOddPages'
  | 'handleReverseEvenPages'
  | 'handleMoveOddPagesToStart'
  | 'handleMoveEvenPagesToStart'
  | 'handleMoveOddPagesToEnd'
  | 'handleMoveEvenPagesToEnd'
  | 'handleSplitOddEven'
  | 'handleDuplicateAllPages'
  | 'handleDuplicatePageToEnd'
  | 'handleDuplicateOddPages'
  | 'handleDuplicateEvenPages'
  | 'handleDuplicateOddPagesBefore'
  | 'handleDuplicateEvenPagesBefore'
  | 'handleDuplicateOddPagesToEnd'
  | 'handleDuplicateEvenPagesToEnd'
  | 'handleDuplicateOddPagesToStart'
  | 'handleDuplicateEvenPagesToStart'
  | 'openDeleteModal'
  | 'openDeleteRangeModal'
  | 'openDeleteNthModal'
  | 'openInsertModal'
  | 'openMergeModal'
  | 'openInterleaveModal'
  | 'openPrependModal'
  | 'openReplacePageModal'
  | 'openSplitModal'
  | 'openSplitAtModal'
  | 'openSplitEveryModal'
  | 'openExtractModal'
  | 'openExtractOddModal'
  | 'openExtractEvenModal'
  | 'handleSortPagesBySize'
  | 'handleSortOddPagesBySize'
  | 'handleSortEvenPagesBySize'
  | 'handleSortPagesByRotation'
  | 'handleSortOddPagesByRotation'
  | 'handleSortEvenPagesByRotation'
> {
  return {
    handleRotatePage: source.handleRotatePage,
    handleRotatePageCcw: voidRun(source.handleRotatePageCcw),
    handleResetPageRotation: voidRun(source.handleResetPageRotation),
    handleRotatePage180: voidRun(source.handleRotatePage180),
    handleRotateAllPages: voidRun(source.handleRotateAllPages),
    handleRotateAllPagesCcw: voidRun(source.handleRotateAllPagesCcw),
    handleRotateAllPages180: voidRun(source.handleRotateAllPages180),
    handleRotateOddPages: voidRun(source.handleRotateOddPages),
    handleRotateEvenPages: voidRun(source.handleRotateEvenPages),
    handleRotateOddPagesCcw: voidRun(source.handleRotateOddPagesCcw),
    handleRotateEvenPagesCcw: voidRun(source.handleRotateEvenPagesCcw),
    handleRotate180OddPages: voidRun(source.handleRotate180OddPages),
    handleRotate180EvenPages: voidRun(source.handleRotate180EvenPages),
    handleResetRotationOddPages: voidRun(source.handleResetRotationOddPages),
    handleResetRotationEvenPages: voidRun(source.handleResetRotationEvenPages),
    handleResetAllRotations: voidRun(source.handleResetAllRotations),
    openRotateModal: source.openRotateModal,
    openRotateRangeModal: source.openRotateRangeModal,
    handleDuplicatePage: source.handleDuplicatePage,
    handleDuplicatePageBefore: voidRun(source.handleDuplicatePageBefore),
    openDuplicateRangeModal: source.openDuplicateRangeModal,
    openParityRangeModal: source.openParityRangeModal,
    openMoveRangeModal: source.openMoveRangeModal,
    openKeepRangeModal: source.openKeepRangeModal,
    handleKeepOddPages: voidRun(source.handleKeepOddPages),
    handleKeepEvenPages: voidRun(source.handleKeepEvenPages),
    handleDeleteOddPages: voidRun(source.handleDeleteOddPages),
    handleDeleteEvenPages: voidRun(source.handleDeleteEvenPages),
    handleAddBlankPage: voidRun(source.handleAddBlankPage),
    handleAddBlankPageBefore: voidRun(source.handleAddBlankPageBefore),
    openInsertBlankPagesModal: source.openInsertBlankPagesModal,
    handleInsertBlankBetweenPages: voidRun(
      source.handleInsertBlankBetweenPages
    ),
    handleInsertBlankBeforeOddPages: voidRun(
      source.handleInsertBlankBeforeOddPages
    ),
    handleInsertBlankBeforeEvenPages: voidRun(
      source.handleInsertBlankBeforeEvenPages
    ),
    handleInsertBlankAfterOddPages: voidRun(
      source.handleInsertBlankAfterOddPages
    ),
    handleInsertBlankAfterEvenPages: voidRun(
      source.handleInsertBlankAfterEvenPages
    ),
    handleMovePageToFirst: voidRun(source.handleMovePageToFirst),
    handleMovePageToLast: voidRun(source.handleMovePageToLast),
    handleMovePageUp: voidRun(source.handleMovePageUp),
    handleMovePageDown: voidRun(source.handleMovePageDown),
    openSwapPagesModal: source.openSwapPagesModal,
    handleReversePages: voidRun(source.handleReversePages),
    openReverseRangeModal: source.openReverseRangeModal,
    handleReverseOddPages: voidRun(source.handleReverseOddPages),
    handleReverseEvenPages: voidRun(source.handleReverseEvenPages),
    handleMoveOddPagesToStart: voidRun(source.handleMoveOddPagesToStart),
    handleMoveEvenPagesToStart: voidRun(source.handleMoveEvenPagesToStart),
    handleMoveOddPagesToEnd: voidRun(source.handleMoveOddPagesToEnd),
    handleMoveEvenPagesToEnd: voidRun(source.handleMoveEvenPagesToEnd),
    handleSplitOddEven: voidRun(source.handleSplitOddEven),
    handleDuplicateAllPages: voidRun(source.handleDuplicateAllPages),
    handleDuplicatePageToEnd: voidRun(source.handleDuplicatePageToEnd),
    handleDuplicateOddPages: voidRun(source.handleDuplicateOddPages),
    handleDuplicateEvenPages: voidRun(source.handleDuplicateEvenPages),
    handleDuplicateOddPagesBefore: voidRun(
      source.handleDuplicateOddPagesBefore
    ),
    handleDuplicateEvenPagesBefore: voidRun(
      source.handleDuplicateEvenPagesBefore
    ),
    handleDuplicateOddPagesToEnd: voidRun(source.handleDuplicateOddPagesToEnd),
    handleDuplicateEvenPagesToEnd: voidRun(
      source.handleDuplicateEvenPagesToEnd
    ),
    handleDuplicateOddPagesToStart: voidRun(
      source.handleDuplicateOddPagesToStart
    ),
    handleDuplicateEvenPagesToStart: voidRun(
      source.handleDuplicateEvenPagesToStart
    ),
    openDeleteModal: source.openDeleteModal,
    openDeleteRangeModal: source.openDeleteRangeModal,
    openDeleteNthModal: source.openDeleteNthModal,
    openInsertModal: source.openInsertModal,
    openMergeModal: source.openMergeModal,
    openInterleaveModal: source.openInterleaveModal,
    openPrependModal: source.openPrependModal,
    openReplacePageModal: source.openReplacePageModal,
    openSplitModal: source.openSplitModal,
    openSplitAtModal: source.openSplitAtModal,
    openSplitEveryModal: source.openSplitEveryModal,
    openExtractModal: source.openExtractModal,
    openExtractOddModal: source.openExtractOddModal,
    openExtractEvenModal: source.openExtractEvenModal,
    handleSortPagesBySize: voidSort(source.handleSortPagesBySize),
    handleSortOddPagesBySize: voidSort(source.handleSortOddPagesBySize),
    handleSortEvenPagesBySize: voidSort(source.handleSortEvenPagesBySize),
    handleSortPagesByRotation: voidSort(source.handleSortPagesByRotation),
    handleSortOddPagesByRotation: voidSort(source.handleSortOddPagesByRotation),
    handleSortEvenPagesByRotation: voidSort(
      source.handleSortEvenPagesByRotation
    ),
  };
}
