import type { AppMenuContext, MenuRoot } from './types';
import { act, sep, sub } from './menuBuilders';

export function buildDocumentMenu(ctx: AppMenuContext): MenuRoot {
  return {
    id: 'document',
    label: 'Document',
    disabled: !ctx.hasPdf,
    items: [
      act('optimize', 'Optimize PDF', ctx.handleOptimizePdf, { shortcutCommandId: 'optimize-pdf' }),
      act('make-searchable', 'Make Searchable (OCR)', () => void ctx.handleMakePdfSearchable()),
      act(
        'apply-redactions',
        'Apply Redactions…',
        ctx.openApplyRedactionsModal,
        { disabled: !ctx.hasRedactions, danger: true },
      ),
      act('metadata', 'Edit metadata…', ctx.openMetadataModal),
      act('summarize', 'Summarize & extract…', ctx.handleSummarizePdf, { shortcutCommandId: 'summarize' }),
      sep(),
      act('page-numbers', 'Add page numbers…', ctx.openPageNumbersModal),
      act('bates-numbers', 'Bates Numbering…', ctx.openBatesNumberModal),
      act('page-header', 'Add page header…', ctx.openPageHeaderModal),
      act('page-footer', 'Add page footer…', ctx.openPageFooterModal),
      act('page-size', 'Set page size…', ctx.openPageSizeModal),
      act('watermark', 'Add watermark…', ctx.openWatermarkModal),
      act('border', 'Draw page border…', ctx.openPageBorderModal),
      sep(),
      sub('Crop', [
        act('crop', 'Crop current page…', ctx.openCropModal),
        act('crop-range', 'Crop page range…', ctx.openCropRangeModal),
        act('crop-odd', 'Crop odd pages', ctx.handleCropOddPages),
        act('crop-even', 'Crop even pages', ctx.handleCropEvenPages),
      ]),
      act('expand', 'Expand margins…', ctx.openExpandMarginsModal),
      act('shrink', 'Shrink margins…', ctx.openShrinkMarginsModal),
      sep(),
      sub('Flatten annotations', [
        act('flatten', 'Flatten current page…', ctx.openFlattenModal),
        act('flatten-all', 'Flatten all pages', ctx.handleFlattenAllAnnotations),
        act('flatten-odd', 'Flatten odd pages', ctx.handleFlattenOddPages),
        act('flatten-even', 'Flatten even pages', ctx.handleFlattenEvenPages),
      ]),
    ],
  };
}
