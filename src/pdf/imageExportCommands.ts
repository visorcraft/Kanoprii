export type ImageExportFormat = 'png' | 'jpeg' | 'webp' | 'bmp' | 'tiff' | 'gif' | 'ppm' | 'tga' | 'ico';

type ImageExportMeta = {
  ext: string;
  label: string;
  singleCommand: string;
  multiCommand: string;
};

const IMAGE_EXPORT_META: Record<ImageExportFormat, ImageExportMeta> = {
  png: { ext: 'png', label: 'PNG', singleCommand: 'export_pdf_page_png', multiCommand: 'export_pdf_pages_png' },
  jpeg: { ext: 'jpg', label: 'JPEG', singleCommand: 'export_pdf_page_jpeg', multiCommand: 'export_pdf_pages_jpeg' },
  webp: { ext: 'webp', label: 'WebP', singleCommand: 'export_pdf_page_webp', multiCommand: 'export_pdf_pages_webp' },
  bmp: { ext: 'bmp', label: 'BMP', singleCommand: 'export_pdf_page_bmp', multiCommand: 'export_pdf_pages_bmp' },
  tiff: { ext: 'tiff', label: 'TIFF', singleCommand: 'export_pdf_page_tiff', multiCommand: 'export_pdf_pages_tiff' },
  gif: { ext: 'gif', label: 'GIF', singleCommand: 'export_pdf_page_gif', multiCommand: 'export_pdf_pages_gif' },
  ppm: { ext: 'ppm', label: 'PPM', singleCommand: 'export_pdf_page_ppm', multiCommand: 'export_pdf_pages_ppm' },
  tga: { ext: 'tga', label: 'TGA', singleCommand: 'export_pdf_page_tga', multiCommand: 'export_pdf_pages_tga' },
  ico: { ext: 'ico', label: 'ICO', singleCommand: 'export_pdf_page_ico', multiCommand: 'export_pdf_pages_ico' },
};

export function imageExportExtension(format: ImageExportFormat): string {
  return IMAGE_EXPORT_META[format].ext;
}

export function imageExportLabel(format: ImageExportFormat): string {
  return IMAGE_EXPORT_META[format].label;
}

export function imageExportCommand(format: ImageExportFormat, multi: boolean): string {
  const meta = IMAGE_EXPORT_META[format];
  return multi ? meta.multiCommand : meta.singleCommand;
}

export function parityImageExportCommand(format: ImageExportFormat, odd: boolean): string {
  const side = odd ? 'odd' : 'even';
  return `export_${side}_pages_${format === 'jpeg' ? 'jpeg' : format}`;
}
