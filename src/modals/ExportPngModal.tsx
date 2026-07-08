import type { PageRangeScope } from '../pageRange/types';
import { resolvePageRange } from '../pageRange/resolvePageRange';
import { PageRangePairInputs } from '../pageRange/PageRangeFields';
import type { PageRangeController } from '../pageRange/usePageRange';
import { type ImageExportFormat, imageExportExtension } from '../pdf/imageExportCommands';
import { Modal } from '../ui/Modal';

type ExportPngModalProps = {
  range: PageRangeController;
  pageCount: number | null;
  currentPage: number;
  format: ImageExportFormat;
  outputPath: string;
  nativeDialogs: boolean;
  defaultOutputPath: (format: ImageExportFormat, scope: PageRangeScope, start: number, end: number) => string;
  onFormatChange: (format: ImageExportFormat) => void;
  onOutputPathChange: (path: string) => void;
  onClose: () => void;
  onChooseOutputNative: () => void;
  onExport: () => void;
  onExportOdd: () => void;
  onExportEven: () => void;
};

export function ExportPngModal({
  range,
  pageCount,
  currentPage,
  format,
  outputPath,
  nativeDialogs,
  defaultOutputPath,
  onFormatChange,
  onOutputPathChange,
  onClose,
  onChooseOutputNative,
  onExport,
  onExportOdd,
  onExportEven,
}: ExportPngModalProps) {
  const ext = imageExportExtension(format);

  return (
    <Modal onClose={onClose}>
      <h3>Export Image</h3>
      <p className="modal-help">Render PDF pages to PNG, JPEG, WebP, BMP, TIFF, GIF, PPM, TGA, or ICO images (1600×2264). The open PDF is not modified.</p>
      <label>Format:</label>
      <select
        className="modal-input"
        value={format}
        onChange={(e) => {
          const next = e.target.value as ImageExportFormat;
          onFormatChange(next);
          const start = range.scope === 'current' ? currentPage : range.startPage;
          const end = range.scope === 'all' ? (pageCount ?? 1) - 1 : range.scope === 'current' ? currentPage : range.endPage;
          onOutputPathChange(defaultOutputPath(next, range.scope, start, end));
        }}
      >
        <option value="png">PNG</option>
        <option value="jpeg">JPEG</option>
        <option value="webp">WebP</option>
        <option value="bmp">BMP</option>
        <option value="tiff">TIFF</option>
        <option value="gif">GIF</option>
        <option value="ppm">PPM</option>
        <option value="tga">TGA</option>
        <option value="ico">ICO</option>
      </select>
      <label>Pages to export:</label>
      <select
        className="modal-input"
        value={range.scope}
        onChange={(e) => {
          const scope = e.target.value as PageRangeScope;
          range.setScope(scope);
          const resolved = resolvePageRange(scope, range.startPage, range.endPage, currentPage, pageCount);
          onOutputPathChange(defaultOutputPath(format, scope, resolved.start, resolved.end));
        }}
      >
        <option value="current">Current page only</option>
        <option value="range">Page range</option>
        <option value="all">All pages</option>
      </select>
      {range.scope === 'range' && (
        <PageRangePairInputs
          startPage={range.startPage}
          endPage={range.endPage}
          onStartChange={(start) => {
            range.setStartPage(start);
            onOutputPathChange(defaultOutputPath(format, 'range', start, range.endPage));
          }}
          onEndChange={(end) => {
            range.setEndPage(end);
            onOutputPathChange(defaultOutputPath(format, 'range', range.startPage, end));
          }}
          maxPage={pageCount ?? undefined}
        />
      )}
      <label>{range.scope === 'current' ? 'Output file path:' : 'Output directory:'}</label>
      <div className="modal-path-row">
        <input
          type="text"
          value={outputPath}
          onChange={(e) => onOutputPathChange(e.target.value)}
          className="modal-input"
          placeholder={range.scope === 'current' ? '/path/to/page.png' : '/path/to/output_dir'}
        />
        {nativeDialogs && (
          <button onClick={() => void onChooseOutputNative()} className="btn">Choose…</button>
        )}
      </div>
      {range.scope !== 'current' && (
        <p className="modal-help">
          Files are written as page-001.{ext}, page-002.{ext}, … inside the directory.
        </p>
      )}
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        {range.scope !== 'current' && (
          <>
            <button onClick={() => void onExportOdd()} className="btn" disabled={!outputPath.trim()}>Export Odd</button>
            <button onClick={() => void onExportEven()} className="btn" disabled={!outputPath.trim()}>Export Even</button>
          </>
        )}
        <button onClick={() => void onExport()} className="btn" disabled={!outputPath.trim()}>Export</button>
      </div>
    </Modal>
  );
}
