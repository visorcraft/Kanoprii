import { open as openNativeDialog, save as saveNativeDialog } from '@tauri-apps/plugin-dialog';
import {
  BMP_DIALOG_FILTER,
  CERT_DIALOG_FILTER,
  DEFAULT_TESSERACT_GUIDE,
  GIF_DIALOG_FILTER,
  JPEG_DIALOG_FILTER,
  MARKDOWN_DIALOG_FILTER,
  MAX_ZOOM,
  MIN_ZOOM,
  PDF_DIALOG_FILTER,
  PNG_DIALOG_FILTER,
  PPM_DIALOG_FILTER,
  RECENT_PDFS_KEY,
  STAMP_PRESETS,
  TESSERACT_REMIND_DISMISSED_KEY,
  TIFF_DIALOG_FILTER,
  WEBP_DIALOG_FILTER,
} from './constants';
import type { MarkdownSaveResult, MarkdownOcrNotice, PdfSummaryResult } from './types';

export { DEFAULT_TESSERACT_GUIDE };

export const clampZoom = (z: number) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z));

export const siblingMarkdownPath = (pdfPath: string) => pdfPath.replace(/\.pdf$/i, '.md');

export const formatSummaryMarkdown = (summary: PdfSummaryResult): string => {
  const lines: string[] = ['# Document Summary', ''];
  if (summary.titleGuess) lines.push(`**Title guess:** ${summary.titleGuess}`, '');
  lines.push(
    `**Pages:** ${summary.pageCount} · **Words:** ${summary.wordCount} · **Scanned/image-only pages:** ${summary.scannedPages}`,
    '',
    '## Overview',
    '',
    summary.overview,
    '',
    '## Key points',
    '',
  );
  if (summary.keyPoints.length === 0) lines.push('_(none)_', '');
  else summary.keyPoints.forEach((point) => lines.push(`- ${point}`));
  lines.push('', '## Extracted headings', '');
  if (summary.extraction.headings.length === 0) lines.push('_(none)_', '');
  else summary.extraction.headings.forEach((heading) => lines.push(`- ${heading}`));
  lines.push('', '## Emails', '');
  if (summary.extraction.emails.length === 0) lines.push('_(none)_', '');
  else summary.extraction.emails.forEach((email) => lines.push(`- ${email}`));
  lines.push('', '## URLs', '');
  if (summary.extraction.urls.length === 0) lines.push('_(none)_', '');
  else summary.extraction.urls.forEach((url) => lines.push(`- ${url}`));
  lines.push('', '## Dates', '');
  if (summary.extraction.dates.length === 0) lines.push('_(none)_');
  else summary.extraction.dates.forEach((date) => lines.push(`- ${date}`));
  return lines.join('\n');
};

export const readStoredString = (key: string): string => {
  try {
    return window.localStorage.getItem(key) ?? '';
  } catch {
    return '';
  }
};

export const readStoredStringArray = (key: string): string[] => {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(key) ?? '[]');
    return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === 'string') : [];
  } catch {
    return [];
  }
};

export const writeStoredString = (key: string, value: string) => {
  try {
    if (value) window.localStorage.setItem(key, value);
  } catch {
    // localStorage can be unavailable in restricted webviews; persistence is optional.
  }
};

export const writeStoredStringArray = (key: string, value: string[]) => {
  try {
    window.localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // localStorage can be unavailable in restricted webviews; persistence is optional.
  }
};

export const isTesseractReminderDismissed = () => readStoredString(TESSERACT_REMIND_DISMISSED_KEY) === '1';

export const dismissTesseractReminder = () => writeStoredString(TESSERACT_REMIND_DISMISSED_KEY, '1');

export const directoryFromPath = (path: string): string => {
  const trimmed = path.trim();
  const slash = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'));
  return slash > 0 ? trimmed.slice(0, slash) : '';
};

export const fileNameFromPath = (path: string): string => {
  const slash = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  return slash >= 0 ? path.slice(slash + 1) : path;
};

export const stampPresetMeta = (preset: string | null | undefined) => {
  return STAMP_PRESETS.find((entry) => entry.id === preset);
};

export const shapeStrokeColor = (color: [number, number, number] | null): string => {
  if (!color) return 'rgb(255,0,0)';
  return `rgb(${color[0] * 255},${color[1] * 255},${color[2] * 255})`;
};

export const inkPointsToPolyline = (points: number[] | null | undefined): string => {
  if (!points || points.length < 2) return '';
  const pairs: string[] = [];
  for (let i = 0; i + 1 < points.length; i += 2) {
    pairs.push(`${points[i]},${points[i + 1]}`);
  }
  return pairs.join(' ');
};

export const markdownOcrNoticeFromResult = (result: MarkdownSaveResult): MarkdownOcrNotice | null => {
  if (result.pagesNeedingOcr === 0) return null;
  if (result.ocrMissingHints > 0 || result.ocrTextBlocks === 0) {
    return {
      tone: 'warning',
      message: 'Scanned pages - pictures saved, text not read',
    };
  }
  return {
    tone: 'success',
    message: 'Text read from scanned pages',
  };
};

export const markdownSaveToastMessage = (result: MarkdownSaveResult): string => {
  const base = result.written
    ? `Markdown saved to ${result.markdownPath}`
    : 'Markdown file is already up to date';
  if (result.pagesNeedingOcr === 0) return base;
  if (result.ocrMissingHints > 0 || result.ocrTextBlocks === 0) {
    return `${base}. Some pages are scans - pictures were saved, but their text couldn't be read.`;
  }
  return `${base}. Text was read from scanned pages.`;
};

export const signatureStatusLabel = (status: string): string => {
  switch (status) {
    case 'valid':
      return 'Valid (trusted)';
    case 'valid_untrusted':
      return 'Valid (untrusted CA)';
    case 'invalid':
      return 'Invalid';
    case 'indeterminate':
      return 'Indeterminate';
    default:
      return status;
  }
};

export const ensureExtension = (path: string, extension: string): string => {
  const lower = path.toLowerCase();
  const suffix = `.${extension}`;
  return lower.endsWith(suffix) ? path : `${path}${suffix}`;
};

export const pickPdfWithNativeDialog = async (defaultPath?: string): Promise<string | null> => {
  const selected = await openNativeDialog({
    multiple: false,
    directory: false,
    defaultPath: defaultPath?.trim() || undefined,
    filters: PDF_DIALOG_FILTER,
  });
  if (selected === null) return null;
  return typeof selected === 'string' ? selected : selected[0] ?? null;
};

export const pickSaveWithNativeDialog = async (
  defaultPath: string,
  filters: { name: string; extensions: string[] }[],
): Promise<string | null> => saveNativeDialog({
  defaultPath: defaultPath.trim() || undefined,
  filters,
});

export {
  BMP_DIALOG_FILTER,
  CERT_DIALOG_FILTER,
  GIF_DIALOG_FILTER,
  JPEG_DIALOG_FILTER,
  MARKDOWN_DIALOG_FILTER,
  PDF_DIALOG_FILTER,
  PNG_DIALOG_FILTER,
  PPM_DIALOG_FILTER,
  RECENT_PDFS_KEY,
  TIFF_DIALOG_FILTER,
  WEBP_DIALOG_FILTER,
};
