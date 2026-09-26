import type { TesseractInstallGuide } from '../modals/TesseractReminderModal';

export const MIN_ZOOM = 0.25;
export const MAX_ZOOM = 4;
export const ZOOM_STEP = 0.25;
export const WHEEL_NAV_COOLDOWN = 350;

/** Natural viewer page max box. Pages are aspect-fitted inside this. */
export const VIEWER_PAGE_W = 1600;
export const VIEWER_PAGE_H = 2264;

/**
 * Effective page height in PDF points along the viewer vertical axis. Pages
 * rotated 90/270 swap width and height, so the viewer's vertical axis maps to
 * the MediaBox width instead of the height. Returns `undefined` when the size
 * is unknown so callers can fall back to the legacy 1:1 assumption.
 *
 * Text styles carry `fontSize` in PDF points (it is written straight to the
 * PDF `Tf` operator and returned in points by `get_page_text_lines`), but the
 * edit overlays live in viewer pixel space. To preview text at the size it
 * will actually commit, the overlay scales points by
 * `viewerPageSize(size).h / pageHeightPtFor(size)` (about 2.69x on A4).
 */
export function pageHeightPtFor(
  size: { width: number; height: number; rotation: number } | undefined,
): number | undefined {
  if (!size || !(size.height > 0)) return undefined;
  return size.rotation % 180 === 0 ? size.height : size.width;
}

export function pageWidthPtFor(
  size: { width: number; height: number; rotation: number } | undefined,
): number | undefined {
  if (!size || !(size.width > 0)) return undefined;
  return size.rotation % 180 === 0 ? size.width : size.height;
}

/** Aspect-fit viewer pixel size for a page (matches Rust `viewer_size_for_media`). */
export function viewerPageSize(
  size: { width: number; height: number; rotation: number } | undefined,
): { w: number; h: number } {
  const pw = pageWidthPtFor(size);
  const ph = pageHeightPtFor(size);
  if (!pw || !ph || !(pw > 0) || !(ph > 0)) {
    return { w: VIEWER_PAGE_W, h: VIEWER_PAGE_H };
  }
  const scale = Math.min(VIEWER_PAGE_W / pw, VIEWER_PAGE_H / ph);
  return {
    w: Math.max(1, Math.round(pw * scale)),
    h: Math.max(1, Math.round(ph * scale)),
  };
}

export const RECENT_PDFS_KEY = 'kanoprii:recent-pdfs';
export const LAST_BROWSER_DIR_KEY = 'kanoprii:last-browser-dir';
export const TESSERACT_REMIND_DISMISSED_KEY =
  'kanoprii:tesseract-remind-dismissed';
export const RECENT_PDF_LIMIT = 8;

export type ShapeKind = 'square' | 'circle' | 'line' | 'arrow';
export type StampKind = 'text' | 'image';

export const STAMP_PRESETS = [
  { id: 'approved', label: 'APPROVED', color: '#228b22' },
  { id: 'draft', label: 'DRAFT', color: '#787878' },
  { id: 'confidential', label: 'CONFIDENTIAL', color: '#b22222' },
  { id: 'reviewed', label: 'REVIEWED', color: '#1e5aa0' },
] as const;

export const PDF_DIALOG_FILTER = [{ name: 'PDF', extensions: ['pdf'] }];
export const DOCUMENT_DIALOG_FILTER = [
  { name: 'Documents', extensions: ['pdf', 'md', 'markdown', 'html', 'htm'] },
];
export const PNG_DIALOG_FILTER = [{ name: 'PNG', extensions: ['png'] }];
export const JPEG_DIALOG_FILTER = [
  { name: 'JPEG', extensions: ['jpg', 'jpeg'] },
];
export const WEBP_DIALOG_FILTER = [{ name: 'WebP', extensions: ['webp'] }];
export const BMP_DIALOG_FILTER = [{ name: 'BMP', extensions: ['bmp'] }];
export const TIFF_DIALOG_FILTER = [
  { name: 'TIFF', extensions: ['tiff', 'tif'] },
];
export const GIF_DIALOG_FILTER = [{ name: 'GIF', extensions: ['gif'] }];
export const PPM_DIALOG_FILTER = [{ name: 'PPM', extensions: ['ppm', 'pnm'] }];
export const MARKDOWN_DIALOG_FILTER = [
  { name: 'Markdown', extensions: ['md', 'markdown'] },
];
export const CERT_DIALOG_FILTER = [
  { name: 'PKCS#12', extensions: ['p12', 'pfx'] },
];

export const DEFAULT_TESSERACT_GUIDE: TesseractInstallGuide = {
  platform: 'unknown',
  summary:
    'Tesseract lets Kanoprii read text from scanned PDF pages. Normal PDFs with selectable text work without it.',
  steps: [
    'Install Tesseract with English language support for your operating system.',
    'Restart Kanoprii.',
  ],
  installCommand: null,
  downloadUrl: 'https://github.com/tesseract-ocr/tesseract',
  licenseNote:
    'Tesseract is free, open-source software. You do not need to pay for it.',
};
