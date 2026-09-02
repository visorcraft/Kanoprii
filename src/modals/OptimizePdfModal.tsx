import { invoke } from '@tauri-apps/api/core';
import { useEffect, useId, useState } from 'react';
import { Modal } from '../ui/Modal';

type OptimizeEstimate = {
  originalBytes: number;
  estimatedBytes: number;
  imagesRecompressed: number;
};

type OptimizePdfModalProps = {
  originalFileName: string;
  filePath: string;
  onClose: () => void;
  onReplaceOriginal: (jpegQuality: number, maxDpi: number) => void;
  onSaveAsNew: (jpegQuality: number, maxDpi: number) => void;
};

function formatBytes(n: number): string {
  if (n >= 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(2)} MB`;
  if (n >= 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${n} B`;
}

function sizeLine(original: number, next: number): string {
  if (original > 0 && next < original) {
    const percent = Math.round(((original - next) / original) * 100);
    return `${formatBytes(next)} (${percent}% smaller)`;
  }
  return `${formatBytes(next)} (no smaller)`;
}

export function OptimizePdfModal({
  originalFileName,
  filePath,
  onClose,
  onReplaceOriginal,
  onSaveAsNew,
}: OptimizePdfModalProps) {
  const name = originalFileName || 'this PDF';
  const siblingName = name.toLowerCase().endsWith('.pdf')
    ? `${name.slice(0, -4)}_optimized.pdf`
    : `${name}_optimized.pdf`;
  const baseId = useId();
  const qualityId = `${baseId}-quality`;
  const dpiId = `${baseId}-dpi`;
  const [quality, setQuality] = useState(50);
  const [maxDpi, setMaxDpi] = useState(150);
  const [currentBytes, setCurrentBytes] = useState<number | null>(null);
  const [estimate, setEstimate] = useState<OptimizeEstimate | null>(null);
  const [estimating, setEstimating] = useState(false);
  const [estimateError, setEstimateError] = useState<string | null>(null);

  useEffect(() => {
    if (!filePath) return;
    let cancelled = false;
    void invoke<number>('optimize_source_size', { path: filePath })
      .then((bytes) => {
        if (!cancelled) setCurrentBytes(bytes);
      })
      .catch(() => {
        if (!cancelled) setCurrentBytes(null);
      });
    return () => {
      cancelled = true;
    };
  }, [filePath]);

  const changeQuality = (value: number) => {
    setQuality(value);
    setEstimate(null);
    setEstimateError(null);
  };
  const changeDpi = (value: number) => {
    setMaxDpi(value);
    setEstimate(null);
    setEstimateError(null);
  };

  const runEstimate = async () => {
    if (!filePath) return;
    setEstimating(true);
    setEstimateError(null);
    try {
      const result = await invoke<OptimizeEstimate>('estimate_optimize_pdf', {
        path: filePath,
        jpegQuality: quality,
        maxDpi,
      });
      setEstimate(result);
      setCurrentBytes(result.originalBytes);
    } catch (err) {
      setEstimate(null);
      setEstimateError(err instanceof Error ? err.message : String(err));
    } finally {
      setEstimating(false);
    }
  };

  return (
    <Modal onClose={onClose} aria-label="Optimize PDF" data-testid="optimize-pdf-modal">
      <h3>Optimize PDF</h3>
      <p className="modal-help">
        Recompress images at the quality and max DPI you choose, then strip metadata, prune unused objects, and compress streams.
      </p>
      <p className="optimize-size-line" data-testid="optimize-current-size">
        Current size: {currentBytes == null ? '…' : formatBytes(currentBytes)}
      </p>
      <label htmlFor={qualityId}>JPEG quality ({quality})</label>
      <input
        id={qualityId}
        type="range"
        min={10}
        max={90}
        step={1}
        value={quality}
        onChange={(e) => changeQuality(Number(e.target.value))}
        className="modal-range"
        data-testid="optimize-quality"
      />
      <p className="muted">Lower quality makes a smaller file.</p>
      <label htmlFor={dpiId}>Max image DPI</label>
      <select
        id={dpiId}
        className="modal-input"
        value={maxDpi}
        onChange={(e) => changeDpi(Number(e.target.value))}
        data-testid="optimize-dpi"
      >
        <option value={72}>72 DPI (screen)</option>
        <option value={150}>150 DPI (balanced)</option>
        <option value={300}>300 DPI (print)</option>
        <option value={0}>Original (no downsample)</option>
      </select>
      <div className="optimize-estimate-row">
        <button
          type="button"
          className="btn btn-secondary"
          onClick={() => void runEstimate()}
          disabled={estimating || !filePath}
          data-testid="optimize-estimate"
        >
          {estimating ? 'Estimating…' : 'Estimate size'}
        </button>
        <p className="optimize-size-line" data-testid="optimize-estimate-result">
          {estimateError
            ? estimateError
            : estimate
              ? `Estimated ${sizeLine(estimate.originalBytes, estimate.estimatedBytes)}`
              : ''}
        </p>
      </div>
      <p className="modal-help">
        Replace overwrites <code>{name}</code> in place. Save as new writes{' '}
        <code>{siblingName}</code> next to it and leaves the original alone.
        Password-protected originals cannot be replaced; save as a new file instead.
      </p>
      <div className="modal-actions">
        <button type="button" onClick={onClose} className="btn btn-secondary">Cancel</button>
        <button
          type="button"
          onClick={() => void onSaveAsNew(quality, maxDpi)}
          className="btn btn-secondary"
          data-testid="optimize-save-as"
        >
          Save as new file
        </button>
        <button
          type="button"
          onClick={() => void onReplaceOriginal(quality, maxDpi)}
          className="btn"
          data-testid="optimize-replace"
        >
          Replace original
        </button>
      </div>
    </Modal>
  );
}
