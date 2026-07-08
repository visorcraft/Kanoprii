import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import type { PdfUaReport } from '../app/types';

export function PdfUaPanel({ filePath, pdfRevision }: { filePath: string; pdfRevision: number }) {
  const [report, setReport] = useState<PdfUaReport | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!filePath) {
      setReport(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    invoke<PdfUaReport>('inspect_pdfua', { path: filePath })
      .then((r) => { if (!cancelled) setReport(r); })
      .catch(() => { if (!cancelled) setReport(null); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [filePath, pdfRevision]);

  if (loading) {
    return (
      <div className="pdfua-panel" role="region" aria-label="PDF/UA inspection">
        <h3>PDF/UA Check</h3>
        <p className="muted">Inspecting PDF/UA…</p>
      </div>
    );
  }
  if (!report) {
    return (
      <div className="pdfua-panel" role="region" aria-label="PDF/UA inspection">
        <h3>PDF/UA Check</h3>
        <p className="muted">Unable to inspect PDF/UA.</p>
      </div>
    );
  }
  if (report.encrypted) {
    return (
      <div className="pdfua-panel" role="region" aria-label="PDF/UA inspection">
        <h3>PDF/UA Check</h3>
        <p className="muted">Document is encrypted.</p>
      </div>
    );
  }

  return (
    <div className="pdfua-panel" role="region" aria-label="PDF/UA inspection">
      <h3>PDF/UA Check</h3>
      <div className={`pdfua-card ${report.tagged ? 'pdfua-pass' : 'pdfua-warn'}`}>
        <strong>{report.tagged ? 'Tagged' : 'Not tagged'}</strong>
        <span className="muted">{report.tagged ? 'Document has a structure tree.' : 'No tagged structure tree found.'}</span>
      </div>
      <div className={`pdfua-card ${report.hasTitle ? 'pdfua-pass' : 'pdfua-warn'}`}>
        <strong>{report.hasTitle ? 'Title present' : 'No title'}</strong>
        <span className="muted">{report.hasTitle ? 'The Info dictionary contains a title.' : 'Document title is missing.'}</span>
      </div>
      <div className="pdfua-card pdfua-info">
        <strong>Language</strong>
        <span className="muted">{report.language ?? 'Not set'}</span>
      </div>
      <div className="pdfua-card pdfua-info">
        <strong>Figures with alt text</strong>
        <span className="muted">{report.figuresWithAlt} of {report.figuresTotal}</span>
      </div>
      <div className="pdfua-card pdfua-info">
        <strong>Image XObjects</strong>
        <span className="muted">{report.imageXobjects}</span>
      </div>
      <div className="pdfua-card pdfua-info">
        <strong>Pages</strong>
        <span className="muted">{report.pageCount}</span>
      </div>
    </div>
  );
}
