import { useEffect, useRef, useState } from 'react';

import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { Modal } from '../ui/Modal';

type PrinterInfo = {
  systemName: string;
  displayName: string;
  isDefault: boolean;
  driverName: string;
};

type PrintMargins =
  | { kind: 'default' }
  | { kind: 'none' }
  | { kind: 'custom'; top: number; right: number; bottom: number; left: number };

type PrintOptions = {
  pageRange: string;
  orientation: 'portrait' | 'landscape';
  paperSize: string;
  scaling: string;
  margins: PrintMargins;
  colorMode: string;
  includeWatermarks: boolean;
  copies: number;
  duplex: string;
  printerName?: string;
};

type PrintDialogProps = {
  filePath: string;
  pageCount: number;
  currentPage: number;
  onClose: () => void;
  onUseSystemPrint: () => void;
};

const DEFAULT_OPTS: PrintOptions = {
  pageRange: 'all',
  orientation: 'portrait',
  paperSize: 'Letter',
  scaling: 'none',
  margins: { kind: 'default' },
  colorMode: 'color',
  includeWatermarks: true,
  copies: 1,
  duplex: 'simplex',
};

const marginsToBackend = (m: PrintMargins): unknown => {
  switch (m.kind) {
    case 'default':
      return 'default';
    case 'none':
      return 'none';
    case 'custom':
      return { custom: { top: m.top, right: m.right, bottom: m.bottom, left: m.left } };
  }
};

const toBackendOpts = (o: PrintOptions) => ({
  pageRange: o.pageRange === 'all' ? null : o.pageRange,
  orientation: o.orientation,
  paperSize: o.paperSize,
  scaling: o.scaling,
  margins: marginsToBackend(o.margins),
  colorMode: o.colorMode,
  includeWatermarks: o.includeWatermarks,
  printerName: o.printerName ?? null,
  copies: o.copies,
  duplex: o.duplex,
});

const pageRangeMode = (range: string, current: number): 'all' | 'current' | 'custom' => {
  if (range === 'all') return 'all';
  if (range === String(current + 1)) return 'current';
  return 'custom';
};

export function PrintDialog({
  filePath,
  pageCount,
  currentPage,
  onClose,
  onUseSystemPrint,
}: PrintDialogProps) {
  const [printers, setPrinters] = useState<PrinterInfo[]>([]);
  const [opts, setOpts] = useState<PrintOptions>(DEFAULT_OPTS);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [previewLoading, setPreviewLoading] = useState(true);
  const [deferredWorkReady, setDeferredWorkReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [working, setWorking] = useState(false);
  const previewUrlRef = useRef<string | null>(null);
  const previewDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const setPreviewObjectUrl = (url: string | null) => {
    if (previewUrlRef.current) {
      URL.revokeObjectURL(previewUrlRef.current);
    }
    previewUrlRef.current = url;
    setPreviewUrl(url);
  };

  useEffect(() => {
    // Let the modal/menu paint before sync Tauri print commands can block WebKit.
    const timeout = setTimeout(() => setDeferredWorkReady(true), 500);
    return () => {
      clearTimeout(timeout);
    };
  }, []);

  useEffect(() => {
    if (!deferredWorkReady) return;
    let mounted = true;
    void (async () => {
      try {
        const list = await invoke<PrinterInfo[]>('list_printers');
        if (!mounted) return;
        setPrinters(list);
        const defaultPrinter = list.find((p) => p.isDefault) ?? list[0];
        if (defaultPrinter) {
          setOpts((prev) => ({ ...prev, printerName: defaultPrinter.systemName }));
        }
      } catch (e) {
        if (!mounted) return;
        setError(`Failed to load printers: ${e instanceof Error ? e.message : String(e)}`);
      }
    })();
    return () => {
      mounted = false;
    };
  }, [deferredWorkReady]);

  useEffect(() => {
    if (!deferredWorkReady) return;
    let cancelled = false;
    if (previewDebounceRef.current) {
      clearTimeout(previewDebounceRef.current);
      previewDebounceRef.current = null;
    }
    const run = async () => {
      setPreviewLoading(true);
      try {
        const bytes = await invoke<number[]>('render_print_preview', {
          sourcePath: filePath,
          pageIndex: currentPage,
          opts: toBackendOpts(opts),
          width: 400,
          height: 600,
        });
        if (cancelled) return;
        const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' });
        const url = URL.createObjectURL(blob);
        setPreviewObjectUrl(url);
      } catch (e) {
        if (cancelled) return;
        setError(`Preview failed: ${e instanceof Error ? e.message : String(e)}`);
      } finally {
        if (!cancelled) setPreviewLoading(false);
      }
    };
    previewDebounceRef.current = setTimeout(() => {
      previewDebounceRef.current = null;
      void run();
    }, 250);
    return () => {
      cancelled = true;
      if (previewDebounceRef.current) {
        clearTimeout(previewDebounceRef.current);
        previewDebounceRef.current = null;
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: preview reacts to specific option fields only
  }, [deferredWorkReady, filePath, currentPage, opts.orientation, opts.paperSize, opts.scaling, opts.margins, opts.colorMode, opts.includeWatermarks]);

  useEffect(() => {
    return () => {
      if (previewUrlRef.current) {
        URL.revokeObjectURL(previewUrlRef.current);
        previewUrlRef.current = null;
      }
    };
  }, []);

  const handlePrint = async () => {
    if (!opts.printerName) {
      setError('Please select a printer.');
      return;
    }
    setWorking(true);
    setError(null);
    try {
      await invoke('print_document', { sourcePath: filePath, opts: toBackendOpts(opts) });
      onClose();
    } catch (e) {
      setError(`Print failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setWorking(false);
    }
  };

  const handleSaveAsPdf = async () => {
    setError(null);
    let outputPath: string | null;
    try {
      outputPath = await save({
        defaultPath: 'print-output.pdf',
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });
    } catch (e) {
      setError(`Save dialog failed: ${e instanceof Error ? e.message : String(e)}`);
      return;
    }
    if (!outputPath) return;
    setWorking(true);
    try {
      await invoke('print_to_pdf', {
        sourcePath: filePath,
        opts: toBackendOpts(opts),
        outputPath,
      });
      onClose();
    } catch (e) {
      setError(`Save as PDF failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setWorking(false);
    }
  };

  const updatePageRangeMode = (mode: 'all' | 'current' | 'custom') => {
    setOpts((prev) => {
      if (mode === 'all') return { ...prev, pageRange: 'all' };
      if (mode === 'current') return { ...prev, pageRange: String(currentPage + 1) };
      return { ...prev, pageRange: `1-${pageCount}` };
    });
  };

  const updateMarginsKind = (kind: PrintMargins['kind']) => {
    setOpts((prev) => {
      if (prev.margins.kind === kind) return prev;
      if (kind === 'custom') {
        return { ...prev, margins: { kind: 'custom', top: 10, right: 10, bottom: 10, left: 10 } };
      }
      return { ...prev, margins: { kind } };
    });
  };

  const updateMarginValue = (key: 'top' | 'right' | 'bottom' | 'left', value: number) => {
    setOpts((prev) => {
      if (prev.margins.kind !== 'custom') return prev;
      return { ...prev, margins: { ...prev.margins, [key]: value } };
    });
  };

  const rangeMode = pageRangeMode(opts.pageRange, currentPage);
  const canPrint = !working && opts.printerName;
  const customMargins = opts.margins.kind === 'custom' ? opts.margins : null;

  return (
    <Modal onClose={onClose} data-testid="print-dialog" aria-label="Print">
      <div className="print-dialog-layout">
        <div className="print-dialog-controls">
          <h3>Print</h3>
          {error && <p className="modal-error">{error}</p>}

          <div className="print-dialog-field">
            <label htmlFor="print-printer">Printer</label>
            <select
              id="print-printer"
              className="modal-input"
              value={opts.printerName ?? ''}
              onChange={(e) => setOpts((prev) => ({ ...prev, printerName: e.target.value }))}
            >
              {printers.length === 0 && <option value="">No printers found</option>}
              {printers.map((p) => (
                <option key={p.systemName} value={p.systemName}>
                  {p.displayName}
                  {p.isDefault ? ' (default)' : ''}
                </option>
              ))}
            </select>
          </div>

          <div className="print-dialog-row">
            <div className="print-dialog-field">
              <label htmlFor="print-pages">Pages</label>
              <select
                id="print-pages"
                className="modal-input"
                value={rangeMode}
                onChange={(e) => updatePageRangeMode(e.target.value as 'all' | 'current' | 'custom')}
              >
                <option value="all">All</option>
                <option value="current">Current page ({currentPage + 1})</option>
                <option value="custom">Custom range</option>
              </select>
            </div>
            {rangeMode === 'custom' && (
              <div className="print-dialog-field">
                <label htmlFor="print-range">Range</label>
                <input
                  id="print-range"
                  type="text"
                  className="modal-input"
                  value={opts.pageRange === 'all' ? '' : opts.pageRange}
                  onChange={(e) => setOpts((prev) => ({ ...prev, pageRange: e.target.value }))}
                  placeholder="e.g. 1-3, 5"
                />
              </div>
            )}
          </div>

          <div className="print-dialog-row">
            <div className="print-dialog-field">
              <label htmlFor="print-orientation">Orientation</label>
              <select
                id="print-orientation"
                className="modal-input"
                value={opts.orientation}
                onChange={(e) =>
                  setOpts((prev) => ({ ...prev, orientation: e.target.value as 'portrait' | 'landscape' }))
                }
              >
                <option value="portrait">Portrait</option>
                <option value="landscape">Landscape</option>
              </select>
            </div>
            <div className="print-dialog-field">
              <label htmlFor="print-paper">Paper size</label>
              <select
                id="print-paper"
                className="modal-input"
                value={opts.paperSize}
                onChange={(e) => setOpts((prev) => ({ ...prev, paperSize: e.target.value }))}
              >
                <option value="A4">A4</option>
                <option value="Letter">Letter</option>
                <option value="Legal">Legal</option>
              </select>
            </div>
          </div>

          <div className="print-dialog-row">
            <div className="print-dialog-field">
              <label htmlFor="print-scaling">Scaling</label>
              <select
                id="print-scaling"
                className="modal-input"
                value={opts.scaling}
                onChange={(e) => setOpts((prev) => ({ ...prev, scaling: e.target.value }))}
              >
                <option value="fitToPage">Fit to page</option>
                <option value="shrinkToFit">Shrink to fit</option>
                <option value="fill">Fill page</option>
                <option value="none">None</option>
              </select>
            </div>
            <div className="print-dialog-field">
              <label htmlFor="print-color">Color mode</label>
              <select
                id="print-color"
                className="modal-input"
                value={opts.colorMode}
                onChange={(e) => setOpts((prev) => ({ ...prev, colorMode: e.target.value }))}
              >
                <option value="color">Color</option>
                <option value="grayscale">Grayscale</option>
              </select>
            </div>
          </div>

          <div className="print-dialog-field">
            <label className="print-dialog-checkbox">
              <input
                type="checkbox"
                checked={opts.includeWatermarks}
                onChange={(e) => setOpts((prev) => ({ ...prev, includeWatermarks: e.target.checked }))}
              />
              <span>Include watermarks</span>
            </label>
          </div>

          <div className="print-dialog-field">
            <label htmlFor="print-margins">Margins</label>
            <select
              id="print-margins"
              className="modal-input"
              value={opts.margins.kind}
              onChange={(e) => updateMarginsKind(e.target.value as PrintMargins['kind'])}
            >
              <option value="default">Default</option>
              <option value="none">None</option>
              <option value="custom">Custom</option>
            </select>
            {customMargins && (
              <div className="print-dialog-margin-grid">
                {(['top', 'right', 'bottom', 'left'] as const).map((key) => (
                  <div key={key} className="print-dialog-field">
                    <label htmlFor={`print-margin-${key}`}>{key}</label>
                    <input
                      id={`print-margin-${key}`}
                      type="number"
                      min={0}
                      step={1}
                      className="modal-input"
                      value={customMargins[key]}
                      onChange={(e) => updateMarginValue(key, parseFloat(e.target.value) || 0)}
                    />
                  </div>
                ))}
              </div>
            )}
          </div>

          <div className="print-dialog-row">
            <div className="print-dialog-field">
              <label htmlFor="print-copies">Copies</label>
              <input
                id="print-copies"
                type="number"
                min={1}
                max={99}
                className="modal-input"
                value={opts.copies}
                onChange={(e) =>
                  setOpts((prev) => ({ ...prev, copies: Math.max(1, parseInt(e.target.value, 10) || 1) }))
                }
              />
            </div>
            <div className="print-dialog-field">
              <label htmlFor="print-duplex">Duplex</label>
              <select
                id="print-duplex"
                className="modal-input"
                value={opts.duplex}
                onChange={(e) => setOpts((prev) => ({ ...prev, duplex: e.target.value }))}
              >
                <option value="simplex">One-sided</option>
                <option value="longEdge">Two-sided (long edge)</option>
                <option value="shortEdge">Two-sided (short edge)</option>
              </select>
            </div>
          </div>
        </div>

        <div className="print-dialog-preview">
          {previewUrl ? (
            <img src={previewUrl} alt="Print preview" />
          ) : previewLoading ? (
            <div className="print-dialog-preview-placeholder">Loading preview…</div>
          ) : (
            <div className="print-dialog-preview-placeholder">No preview available</div>
          )}
        </div>
      </div>

      <div className="modal-actions">
        <button type="button" className="btn btn-secondary" onClick={onUseSystemPrint} disabled={working}>
          Use System Print Dialog
        </button>
        <button type="button" className="btn btn-secondary" onClick={onClose} disabled={working}>
          Cancel
        </button>
        <button type="button" className="btn" onClick={() => void handleSaveAsPdf()} disabled={working}>
          Save as PDF
        </button>
        <button type="button" className="btn" onClick={() => void handlePrint()} disabled={!canPrint}>
          Print
        </button>
      </div>
    </Modal>
  );
}
