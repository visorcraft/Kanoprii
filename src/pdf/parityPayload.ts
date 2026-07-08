export type ParityBatchContext = {
  filePath: string;
  startPage: number;
  endPage: number;
  outputPath: string;
  marginTop: number;
  marginRight: number;
  marginBottom: number;
  marginLeft: number;
  watermarkText: string;
  pageHeaderText: string;
  pageFooterText: string;
  pageBorderInset: number;
  pageSizePreset: string;
  pageNumbersPrefix: string;
};

export function isParityDocModCommand(command: string): boolean {
  if (command.includes('_in_range')) return false;
  return /_mod3_[0-2]_/.test(command)
    || /_mod4_[0-3]_/.test(command)
    || /_mod5_[0-4]_/.test(command)
    || /_mod6_[0-5]_/.test(command);
}

export function parityBatchNeedsRange(command: string): boolean {
  return !isParityDocModCommand(command)
    && command !== 'export_odd_pages_ico'
    && command !== 'export_even_pages_ico';
}

export function parityBatchMutatesPdf(command: string): boolean {
  return !command.startsWith('export_') && !command.startsWith('extract_');
}

function applyParityCommandFields(
  command: string,
  base: Record<string, unknown>,
  ctx: ParityBatchContext,
): Record<string, unknown> {
  if (command.startsWith('extract_')) {
    return { ...base, outputPath: ctx.outputPath.trim() };
  }
  if (command.startsWith('export_')) {
    return { ...base, outputDir: ctx.outputPath.trim() };
  }
  if (command.includes('crop_') || command.includes('expand_') || command.includes('shrink_')) {
    return {
      ...base,
      marginTop: ctx.marginTop,
      marginRight: ctx.marginRight,
      marginBottom: ctx.marginBottom,
      marginLeft: ctx.marginLeft,
    };
  }
  if (command.includes('watermark')) {
    return { ...base, text: ctx.watermarkText.trim() };
  }
  if (command.includes('header')) {
    return { ...base, text: ctx.pageHeaderText.trim() };
  }
  if (command.includes('footer')) {
    return { ...base, text: ctx.pageFooterText.trim() };
  }
  if (command.includes('border')) {
    return { ...base, inset: ctx.pageBorderInset };
  }
  if (command.includes('page_size')) {
    return { ...base, preset: ctx.pageSizePreset };
  }
  if (command.includes('bookmark') || command.includes('page_numbers')) {
    return { ...base, prefix: ctx.pageNumbersPrefix.trim() || null };
  }
  if (command.includes('_by_rotation') || command.includes('_by_size')) {
    return { ...base, descending: false };
  }
  return base;
}

export function buildParityBatchPayload(
  command: string,
  ctx: ParityBatchContext,
): Record<string, unknown> {
  const docWide = isParityDocModCommand(command)
    || command === 'export_odd_pages_ico'
    || command === 'export_even_pages_ico';

  const base = docWide
    ? { path: ctx.filePath }
    : { path: ctx.filePath, startPage: ctx.startPage, endPage: ctx.endPage };

  return applyParityCommandFields(command, base, ctx);
}
