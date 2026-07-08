import type { AppMenuContext, MenuRoot } from './types';
import { act, canDeletePage, multiPage, sep, sub } from './menuBuilders';

export function buildPagesMenu(ctx: AppMenuContext): MenuRoot {
  const { pageCount, currentPage } = ctx;
  const mp = multiPage(pageCount);
  const del = canDeletePage(pageCount);
  const atFirst = currentPage === 0;
  const atLast = pageCount !== null && currentPage >= pageCount - 1;

  return {
    id: 'pages',
    label: 'Pages',
    disabled: !ctx.hasPdf,
    items: [
      act('rot-modal', 'Rotate…', ctx.openRotateModal),
      sub('Rotate', [
        act('rot-cw', 'Current page 90° clockwise', ctx.handleRotatePage, {
          shortcutCommandId: 'rotate-page',
        }),
        act(
          'rot-ccw',
          'Current page 90° counter-clockwise',
          ctx.handleRotatePageCcw
        ),
        act('rot-180', 'Current page 180°', ctx.handleRotatePage180),
        act(
          'rot-reset',
          'Reset current page rotation',
          ctx.handleResetPageRotation
        ),
        sep(),
        act('rot-all', 'All pages 90° clockwise', ctx.handleRotateAllPages),
        act(
          'rot-all-ccw',
          'All pages 90° counter-clockwise',
          ctx.handleRotateAllPagesCcw
        ),
        act('rot-all-180', 'All pages 180°', ctx.handleRotateAllPages180),
        act(
          'rot-reset-all',
          'Reset all page rotations',
          ctx.handleResetAllRotations
        ),
        sep(),
        sub('Odd pages', [
          act('rot-odd', '90° clockwise', ctx.handleRotateOddPages),
          act(
            'rot-odd-ccw',
            '90° counter-clockwise',
            ctx.handleRotateOddPagesCcw
          ),
          act('rot-odd-180', '180°', ctx.handleRotate180OddPages),
          act(
            'rot-odd-reset',
            'Reset rotation',
            ctx.handleResetRotationOddPages
          ),
        ]),
        sub('Even pages', [
          act('rot-even', '90° clockwise', ctx.handleRotateEvenPages),
          act(
            'rot-even-ccw',
            '90° counter-clockwise',
            ctx.handleRotateEvenPagesCcw
          ),
          act('rot-even-180', '180°', ctx.handleRotate180EvenPages),
          act(
            'rot-even-reset',
            'Reset rotation',
            ctx.handleResetRotationEvenPages
          ),
        ]),
        sep(),
        act('rot-range', 'Page range…', ctx.openRotateRangeModal),
      ]),
      sub('Duplicate', [
        act('dup', 'Duplicate current page', ctx.handleDuplicatePage, {
          shortcutCommandId: 'duplicate-page',
        }),
        act(
          'dup-before',
          'Duplicate before current page',
          ctx.handleDuplicatePageBefore
        ),
        act('dup-range', 'Duplicate page range…', ctx.openDuplicateRangeModal),
        act(
          'dup-all',
          'Duplicate all pages (append)',
          ctx.handleDuplicateAllPages
        ),
        act(
          'dup-end',
          'Duplicate current page to end',
          ctx.handleDuplicatePageToEnd
        ),
        sep(),
        sub('Odd pages', [
          act(
            'dup-odd',
            'Append copies of odd pages',
            ctx.handleDuplicateOddPages
          ),
          act(
            'dup-odd-before',
            'Insert copy before each odd page',
            ctx.handleDuplicateOddPagesBefore
          ),
          act(
            'dup-odd-end',
            'Copy each odd page to end',
            ctx.handleDuplicateOddPagesToEnd
          ),
          act(
            'dup-odd-start',
            'Copy each odd page to start',
            ctx.handleDuplicateOddPagesToStart
          ),
        ]),
        sub('Even pages', [
          act(
            'dup-even',
            'Append copies of even pages',
            ctx.handleDuplicateEvenPages
          ),
          act(
            'dup-even-before',
            'Insert copy before each even page',
            ctx.handleDuplicateEvenPagesBefore
          ),
          act(
            'dup-even-end',
            'Copy each even page to end',
            ctx.handleDuplicateEvenPagesToEnd
          ),
          act(
            'dup-even-start',
            'Copy each even page to start',
            ctx.handleDuplicateEvenPagesToStart
          ),
        ]),
      ]),
      sub('Move & order', [
        act('move-up', 'Move current page up', ctx.handleMovePageUp, {
          disabled: atFirst,
        }),
        act('move-down', 'Move current page down', ctx.handleMovePageDown, {
          disabled: atLast,
        }),
        act(
          'move-first',
          'Move current page to first',
          ctx.handleMovePageToFirst,
          { disabled: atFirst }
        ),
        act(
          'move-last',
          'Move current page to last',
          ctx.handleMovePageToLast,
          { disabled: atLast }
        ),
        act('swap', 'Swap two pages…', ctx.openSwapPagesModal),
        act('move-range', 'Move page range…', ctx.openMoveRangeModal),
        sep(),
        act('reverse', 'Reverse all pages', ctx.handleReversePages, {
          shortcutCommandId: 'reverse-pages',
        }),
        act('reverse-range', 'Reverse page range…', ctx.openReverseRangeModal),
        act('reverse-odd', 'Reverse odd pages', ctx.handleReverseOddPages, {
          disabled: !mp,
        }),
        act('reverse-even', 'Reverse even pages', ctx.handleReverseEvenPages, {
          disabled: !mp,
        }),
        sep(),
        act(
          'odd-start',
          'Move odd pages to start',
          ctx.handleMoveOddPagesToStart,
          { disabled: !mp }
        ),
        act(
          'even-start',
          'Move even pages to start',
          ctx.handleMoveEvenPagesToStart,
          { disabled: !mp }
        ),
        act('odd-end', 'Move odd pages to end', ctx.handleMoveOddPagesToEnd, {
          disabled: !mp,
        }),
        act(
          'even-end',
          'Move even pages to end',
          ctx.handleMoveEvenPagesToEnd,
          { disabled: !mp }
        ),
      ]),
      sub('Insert', [
        act('blank-after', 'Blank page after current', ctx.handleAddBlankPage, {
          shortcutCommandId: 'blank-page-after',
        }),
        act(
          'blank-before',
          'Blank page before current',
          ctx.handleAddBlankPageBefore
        ),
        act(
          'blank-multi',
          'Multiple blank pages…',
          ctx.openInsertBlankPagesModal
        ),
        act(
          'blank-between',
          'Blank page between each pair',
          ctx.handleInsertBlankBetweenPages,
          { disabled: !mp }
        ),
        sep(),
        act(
          'blank-before-odd',
          'Blank before each odd page',
          ctx.handleInsertBlankBeforeOddPages
        ),
        act(
          'blank-before-even',
          'Blank before each even page',
          ctx.handleInsertBlankBeforeEvenPages
        ),
        act(
          'blank-after-odd',
          'Blank after each odd page',
          ctx.handleInsertBlankAfterOddPages
        ),
        act(
          'blank-after-even',
          'Blank after each even page',
          ctx.handleInsertBlankAfterEvenPages
        ),
        sep(),
        act('insert-pdf', 'Pages from another PDF…', ctx.openInsertModal, {
          shortcutCommandId: 'insert-pdf',
        }),
        act('image-page', 'Image as new page…', ctx.openInsertImagePageModal),
      ]),
      sub('Delete', [
        act('delete', 'Delete current page', ctx.openDeleteModal, {
          shortcutCommandId: 'delete-page',
          disabled: !del,
          danger: true,
        }),
        act('delete-range', 'Delete page range…', ctx.openDeleteRangeModal, {
          disabled: !del,
          danger: true,
        }),
        act('delete-nth', 'Delete every Nth page…', ctx.openDeleteNthModal, {
          disabled: !mp,
          danger: true,
        }),
        sep(),
        act('delete-odd', 'Delete odd pages', ctx.handleDeleteOddPages, {
          disabled: !mp,
          danger: true,
        }),
        act('delete-even', 'Delete even pages', ctx.handleDeleteEvenPages, {
          disabled: !mp,
          danger: true,
        }),
      ]),
      sub('Split & extract', [
        act('split', 'Split into parts…', ctx.openSplitModal, {
          shortcutCommandId: 'split-pdf',
        }),
        act('split-at', 'Split at page…', ctx.openSplitAtModal, {
          disabled: !mp,
        }),
        act('split-n', 'Split every N pages…', ctx.openSplitEveryModal),
        sep(),
        act('extract', 'Extract pages…', ctx.openExtractModal, {
          shortcutCommandId: 'extract-pages',
        }),
        act('extract-odd', 'Extract odd pages…', ctx.openExtractOddModal, {
          disabled: !mp,
        }),
        act('extract-even', 'Extract even pages…', ctx.openExtractEvenModal, {
          disabled: !mp,
        }),
        act(
          'split-odd-even',
          'Split into odd/even PDFs',
          ctx.handleSplitOddEven,
          { disabled: !mp }
        ),
      ]),
      sub('Combine', [
        act('merge', 'Merge PDF (append)…', ctx.openMergeModal, {
          shortcutCommandId: 'merge-pdf',
        }),
        act('prepend', 'Prepend pages…', ctx.openPrependModal),
        act('interleave', 'Interleave pages…', ctx.openInterleaveModal),
        act('replace', 'Replace current page…', ctx.openReplacePageModal),
      ]),
      sub('Keep & filter', [
        act('keep-range', 'Keep page range only…', ctx.openKeepRangeModal),
        act('keep-odd', 'Keep odd pages only', ctx.handleKeepOddPages, {
          disabled: !mp,
        }),
        act('keep-even', 'Keep even pages only', ctx.handleKeepEvenPages, {
          disabled: !mp,
        }),
      ]),
      sub('Sort', [
        act('sort-size-up', 'By size (smallest first)', () =>
          ctx.handleSortPagesBySize(false)
        ),
        act('sort-size-down', 'By size (largest first)', () =>
          ctx.handleSortPagesBySize(true)
        ),
        act('sort-rot-up', 'By rotation (0° first)', () =>
          ctx.handleSortPagesByRotation(false)
        ),
        act('sort-rot-down', 'By rotation (270° first)', () =>
          ctx.handleSortPagesByRotation(true)
        ),
        sep(),
        act(
          'sort-odd-size-up',
          'Odd pages by size ↑',
          () => ctx.handleSortOddPagesBySize(false),
          { disabled: !mp }
        ),
        act(
          'sort-odd-size-down',
          'Odd pages by size ↓',
          () => ctx.handleSortOddPagesBySize(true),
          { disabled: !mp }
        ),
        act(
          'sort-even-size-up',
          'Even pages by size ↑',
          () => ctx.handleSortEvenPagesBySize(false),
          { disabled: !mp }
        ),
        act(
          'sort-even-size-down',
          'Even pages by size ↓',
          () => ctx.handleSortEvenPagesBySize(true),
          { disabled: !mp }
        ),
        sep(),
        act(
          'sort-odd-rot-up',
          'Odd pages by rotation ↑',
          () => ctx.handleSortOddPagesByRotation(false),
          { disabled: !mp }
        ),
        act(
          'sort-odd-rot-down',
          'Odd pages by rotation ↓',
          () => ctx.handleSortOddPagesByRotation(true),
          { disabled: !mp }
        ),
        act(
          'sort-even-rot-up',
          'Even pages by rotation ↑',
          () => ctx.handleSortEvenPagesByRotation(false),
          { disabled: !mp }
        ),
        act(
          'sort-even-rot-down',
          'Even pages by rotation ↓',
          () => ctx.handleSortEvenPagesByRotation(true),
          { disabled: !mp }
        ),
      ]),
      sep(),
      act(
        'parity-range',
        'Parity tools for page range…',
        ctx.openParityRangeModal
      ),
    ],
  };
}
