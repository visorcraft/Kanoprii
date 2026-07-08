import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useEscapeClose } from '../legal/useEscapeClose';
import { FocusTrap } from '../ui/FocusTrap';
import { LicenseTextDialog } from './LicenseTextDialog';

type LicenseTab = 'gpl' | 'third-party' | 'acknowledgments' | 'runtime';

type LicenseDocuments = {
  gpl: string;
  third_party: string;
  credits: string;
  runtime: string;
};

type TabConfig = {
  id: LicenseTab;
  label: string;
  title: string;
  subtitle: string;
  body: string;
};

const TAB_ORDER: LicenseTab[] = [
  'gpl',
  'third-party',
  'acknowledgments',
  'runtime',
];

function decodeEntities(text: string): string {
  return text
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&apos;/g, "'")
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&amp;/g, '&');
}

function lineCount(text: string): number {
  if (!text) return 0;
  return text.split('\n').length;
}

function lineNumber(value: number): string {
  return String(value).padStart(5, ' ');
}

function countMatchingLines(text: string, query: string): number {
  const needle = query.trim().toLowerCase();
  if (!needle) return 0;
  return text.split('\n').filter((line) => line.toLowerCase().includes(needle))
    .length;
}

function filteredBody(text: string, query: string): string {
  const needle = query.trim().toLowerCase();
  if (!needle) return text;
  const matches = text
    .split('\n')
    .map((line, index) => ({ line, index }))
    .filter(({ line }) => line.toLowerCase().includes(needle))
    .map(({ line, index }) => `${lineNumber(index + 1)}  ${line}`);
  if (matches.length === 0) return `No matches for "${query.trim()}".`;
  return matches.join('\n');
}

export function LicensesModal({ onClose }: { onClose: () => void }) {
  const [documents, setDocuments] = useState<LicenseDocuments | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<LicenseTab>('gpl');
  const [filterText, setFilterText] = useState('');
  const [wrapText, setWrapText] = useState(false);
  const [showGplDialog, setShowGplDialog] = useState(false);
  const [copyStatus, setCopyStatus] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void invoke<LicenseDocuments>('license_documents')
      .then((docs) => {
        if (!cancelled) setDocuments(docs);
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setLoadError(err instanceof Error ? err.message : String(err));
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const tabs = useMemo<TabConfig[]>(() => {
    if (!documents) return [];
    return [
      {
        id: 'gpl',
        label: 'Kanoprii License',
        title: 'Kanoprii License',
        subtitle:
          'The complete GPL-3.0-only license text bundled into the application.',
        body: documents.gpl,
      },
      {
        id: 'third-party',
        label: 'Third-party',
        title: 'Third-party licenses',
        subtitle:
          'The cargo-about-generated Rust crate bundle plus shipped npm packages, grouped by license text.',
        body: decodeEntities(documents.third_party),
      },
      {
        id: 'acknowledgments',
        label: 'Acknowledgments',
        title: 'Acknowledgments',
        subtitle:
          'Narrative attribution for Kanoprii, runtime components, and direct dependencies.',
        body: decodeEntities(documents.credits),
      },
      {
        id: 'runtime',
        label: 'Runtime components',
        title: 'Runtime components',
        subtitle:
          'Full license texts for PDFium, WebKitGTK, GTK, CUPS, and optional Tesseract OCR runtimes.',
        body: documents.runtime,
      },
    ];
  }, [documents]);

  const currentTab = tabs.find((tab) => tab.id === activeTab) ?? tabs[0];
  const currentBody = currentTab?.body ?? '';
  const visibleBody = filteredBody(currentBody, filterText);
  const matchingLineCount = countMatchingLines(currentBody, filterText);
  const currentLineCount = lineCount(currentBody);

  const switchTab = useCallback((tab: LicenseTab) => {
    setActiveTab(tab);
    setFilterText('');
  }, []);

  const handleCopy = useCallback(async () => {
    if (!currentBody) return;
    try {
      await navigator.clipboard.writeText(currentBody);
      setCopyStatus('Copied');
      window.setTimeout(() => setCopyStatus(null), 2000);
    } catch {
      setCopyStatus('Copy failed');
      window.setTimeout(() => setCopyStatus(null), 2500);
    }
  }, [currentBody]);

  useEscapeClose(() => {
    if (showGplDialog) setShowGplDialog(false);
    else onClose();
  });

  return (
    <>
      <FocusTrap active={!showGplDialog}>
        <div
          className="modal-backdrop legal-backdrop licenses-backdrop"
          onClick={() => {
            if (!showGplDialog) onClose();
          }}
        >
          <div
            className="legal-page licenses-panel"
            onClick={(e) => e.stopPropagation()}
            data-testid="licenses-panel"
          >
            <header className="legal-header licenses-header">
              <div>
                <h2>Licenses</h2>
                <p className="legal-tagline licenses-tagline">
                  Bundled license and attribution documents, available without
                  opening a browser.
                </p>
              </div>
              <button
                type="button"
                className="btn btn-secondary legal-close-btn"
                onClick={onClose}
                aria-label="Close licenses"
              >
                Close
              </button>
            </header>
            <div className="legal-body licenses-body">
              <div className="licenses-toolbar">
                <div
                  className="licenses-tabs"
                  role="tablist"
                  aria-label="License documents"
                >
                  {TAB_ORDER.map((tabId) => {
                    const tab = tabs.find((entry) => entry.id === tabId);
                    const label =
                      tab?.label ??
                      (tabId === 'gpl'
                        ? 'Kanoprii License'
                        : tabId === 'third-party'
                          ? 'Third-party'
                          : tabId === 'acknowledgments'
                            ? 'Acknowledgments'
                            : 'Runtime components');
                    return (
                      <button
                        key={tabId}
                        type="button"
                        role="tab"
                        aria-selected={activeTab === tabId}
                        className={`licenses-tab${activeTab === tabId ? ' licenses-tab-active' : ''}`}
                        onClick={() => switchTab(tabId)}
                        disabled={!documents}
                      >
                        {label}
                      </button>
                    );
                  })}
                </div>

                <div className="licenses-toolbar-actions">
                  <button
                    type="button"
                    className="btn btn-secondary legal-action-btn"
                    onClick={() => void handleCopy()}
                    disabled={!currentBody}
                    data-testid="licenses-copy"
                  >
                    {copyStatus ?? 'Copy'}
                  </button>
                  {activeTab === 'gpl' && (
                    <button
                      type="button"
                      className="btn btn-secondary legal-action-btn"
                      onClick={() => setShowGplDialog(true)}
                      disabled={!documents?.gpl}
                      data-testid="licenses-gpl-dialog"
                    >
                      Dialog
                    </button>
                  )}
                </div>
              </div>

              <div className="licenses-doc-header">
                <div>
                  <h3>{currentTab?.title ?? 'Licenses'}</h3>
                  <p className="legal-subtitle licenses-doc-subtitle">
                    {currentTab?.subtitle ?? ''}
                  </p>
                </div>
                <span className="legal-line-count licenses-line-count">
                  {filterText.trim()
                    ? `${matchingLineCount} matches`
                    : `${currentLineCount} lines`}
                </span>
              </div>

              <div className="licenses-filter-row">
                <input
                  className="modal-input legal-input licenses-filter-input"
                  type="search"
                  placeholder="Find by crate, package, license, or phrase..."
                  value={filterText}
                  onChange={(e) => setFilterText(e.target.value)}
                  aria-label="Find in license document"
                />
                <label className="licenses-wrap-toggle">
                  <input
                    type="checkbox"
                    checked={wrapText}
                    onChange={(e) => setWrapText(e.target.checked)}
                  />
                  Wrap
                </label>
                <button
                  type="button"
                  className="btn btn-secondary legal-action-btn"
                  onClick={() => setFilterText('')}
                  disabled={!filterText}
                >
                  Clear
                </button>
              </div>

              <div className="licenses-document-shell">
                {loadError ? (
                  <p className="legal-load-error licenses-load-error">
                    {loadError}
                  </p>
                ) : !documents ? (
                  <p className="legal-loading licenses-loading">
                    Loading license documents…
                  </p>
                ) : (
                  <textarea
                    className={`licenses-document${wrapText ? ' licenses-document-wrap' : ''}`}
                    readOnly
                    value={visibleBody}
                    spellCheck={false}
                    aria-label={currentTab?.title ?? 'License document'}
                  />
                )}
              </div>
            </div>
          </div>
        </div>
      </FocusTrap>

      {showGplDialog && documents?.gpl && (
        <LicenseTextDialog
          title="GNU General Public License v3"
          detail="GPL-3.0-only license text bundled with Kanoprii."
          body={documents.gpl}
          onClose={() => setShowGplDialog(false)}
        />
      )}
    </>
  );
}
