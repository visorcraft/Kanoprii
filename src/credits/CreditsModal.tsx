import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useEscapeClose } from '../legal/useEscapeClose';
import { openExternalUrl } from '../legal/openExternalUrl';
import { FocusTrap } from '../ui/FocusTrap';
import { LicenseTextDialog } from '../licenses/LicenseTextDialog';

type ThirdPartyCreditRow = {
  name: string;
  version: string;
  license: string;
  url: string;
};

type RuntimeComponentRow = {
  name: string;
  licenses: string;
  url: string;
  spdx: string[];
};

type CreditsCatalog = {
  crates: ThirdPartyCreditRow[];
  npm_packages: ThirdPartyCreditRow[];
  runtime_components: RuntimeComponentRow[];
};

type LicenseDialogState = {
  title: string;
  detail: string;
  body: string;
};

function CreditsTable({
  rows,
  emptyLabel,
  onOpenUrl,
}: {
  rows: ThirdPartyCreditRow[];
  emptyLabel: string;
  onOpenUrl: (url: string) => void;
}) {
  if (rows.length === 0) {
    return <p className="credits-table-empty">{emptyLabel}</p>;
  }

  return (
    <div className="credits-table-shell">
      <div className="credits-table-header">
        <span className="credits-col-name">Package</span>
        <span className="credits-col-version">Version</span>
        <span className="credits-col-license">License expression</span>
        <span className="credits-col-link" aria-hidden="true" />
      </div>
      <div className="credits-table-body" role="list">
        {rows.map((row, index) => (
          <div
            key={`${row.name}-${row.version}-${index}`}
            className={`credits-table-row${index % 2 === 1 ? ' credits-table-row-alt' : ''}`}
            role="listitem"
          >
            <span className="credits-col-name credits-mono">{row.name}</span>
            <span className="credits-col-version credits-mono">
              {row.version}
            </span>
            <span className="credits-col-license">
              <span className="credits-license-pill">{row.license}</span>
            </span>
            <span className="credits-col-link">
              <button
                type="button"
                className="btn btn-secondary credits-icon-btn"
                title="Open project page"
                aria-label={`Open project page for ${row.name}`}
                onClick={() => onOpenUrl(row.url)}
              >
                ↗
              </button>
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

export function CreditsModal({ onClose }: { onClose: () => void }) {
  const [catalog, setCatalog] = useState<CreditsCatalog | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [crateFilter, setCrateFilter] = useState('');
  const [npmFilter, setNpmFilter] = useState('');
  const [licenseDialog, setLicenseDialog] = useState<LicenseDialogState | null>(
    null
  );

  useEffect(() => {
    let cancelled = false;
    void invoke<CreditsCatalog>('credits_catalog')
      .then((data) => {
        if (!cancelled) setCatalog(data);
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

  const filterRows = useCallback(
    (rows: ThirdPartyCreditRow[], needle: string) => {
      const q = needle.trim().toLowerCase();
      if (!q) return rows;
      return rows.filter(
        (row) =>
          row.name.toLowerCase().includes(q) ||
          row.version.toLowerCase().includes(q) ||
          row.license.toLowerCase().includes(q)
      );
    },
    []
  );

  const filteredCrates = useMemo(
    () => (catalog ? filterRows(catalog.crates, crateFilter) : []),
    [catalog, crateFilter, filterRows]
  );
  const filteredNpm = useMemo(
    () => (catalog ? filterRows(catalog.npm_packages, npmFilter) : []),
    [catalog, npmFilter, filterRows]
  );

  const openUrl = useCallback((url: string) => {
    openExternalUrl(url);
  }, []);

  const openComponentLicense = useCallback(
    async (component: RuntimeComponentRow) => {
      const sections: string[] = [];
      for (const spdx of component.spdx) {
        const body = await invoke<string>('runtime_license_text', {
          spdxId: spdx,
        });
        if (body.trim()) {
          sections.push(`===== ${spdx} =====\n\n${body}`);
        }
      }
      setLicenseDialog({
        title: component.name,
        detail: component.licenses,
        body:
          sections.length > 0
            ? sections.join('\n\n\n')
            : 'No bundled license text is available.',
      });
    },
    []
  );

  const crateCount = catalog?.crates.length ?? 0;
  const npmCount = catalog?.npm_packages.length ?? 0;
  const runtimeCount = catalog?.runtime_components.length ?? 0;

  const tagline = loadError
    ? 'Unable to load credits catalog.'
    : `${crateCount} Cargo crates · ${npmCount} npm packages · ${runtimeCount} runtime components`;

  useEscapeClose(() => {
    if (licenseDialog) setLicenseDialog(null);
    else onClose();
  });

  return (
    <>
      <FocusTrap active={!licenseDialog}>
        <div
          className="modal-backdrop legal-backdrop credits-backdrop"
          onClick={() => {
            if (!licenseDialog) onClose();
          }}
        >
          <div
            className="legal-page credits-panel"
            onClick={(e) => e.stopPropagation()}
            data-testid="credits-panel"
          >
            <header className="legal-header credits-header">
              <div>
                <h2>Credits</h2>
                <p className="legal-tagline credits-tagline">{tagline}</p>
              </div>
              <button
                type="button"
                className="btn btn-secondary legal-close-btn"
                onClick={onClose}
                aria-label="Close credits"
              >
                Close
              </button>
            </header>
            <div className="legal-body credits-body">
              {loadError ? (
                <p className="legal-load-error credits-load-error">
                  {loadError}
                </p>
              ) : !catalog ? (
                <p className="legal-loading credits-loading">
                  Loading credits catalog…
                </p>
              ) : (
                <>
                  <section className="credits-runtime-card">
                    <h3>Runtime components</h3>
                    <p className="credits-runtime-help">
                      System libraries and bundled runtimes Kanoprii uses at
                      execution time. Packaged builds ship PDFium; Linux desktop
                      builds use the system WebKitGTK and GTK stacks. CUPS is
                      used for direct printing on Linux and macOS. Tesseract is
                      optional for scan OCR.
                    </p>
                    <ul className="credits-runtime-list">
                      {catalog.runtime_components.map((component) => (
                        <li
                          key={component.name}
                          className="credits-runtime-row"
                        >
                          <span className="credits-runtime-name">
                            {component.name}
                          </span>
                          <span className="credits-runtime-license">
                            {component.licenses}
                          </span>
                          <div className="credits-row-actions">
                            <button
                              type="button"
                              className="btn btn-secondary credits-icon-btn"
                              title="View license text"
                              aria-label={`View license text for ${component.name}`}
                              onClick={() =>
                                void openComponentLicense(component)
                              }
                            >
                              License
                            </button>
                            <button
                              type="button"
                              className="btn btn-secondary credits-icon-btn"
                              title="Open project website"
                              aria-label={`Open website for ${component.name}`}
                              onClick={() => openUrl(component.url)}
                            >
                              ↗
                            </button>
                          </div>
                        </li>
                      ))}
                    </ul>
                  </section>

                  <div className="credits-section-heading">Cargo crates</div>
                  <div className="credits-filter-row">
                    <input
                      className="modal-input legal-input credits-filter-input"
                      type="search"
                      placeholder="Filter by crate name or license..."
                      value={crateFilter}
                      onChange={(e) => setCrateFilter(e.target.value)}
                      aria-label="Filter Cargo crate credits"
                    />
                    <span className="credits-filter-count">
                      {filteredCrates.length} / {crateCount}
                    </span>
                  </div>
                  <CreditsTable
                    rows={filteredCrates}
                    emptyLabel={
                      crateFilter.trim()
                        ? 'No Cargo crates match the current filter.'
                        : 'No Cargo crates listed.'
                    }
                    onOpenUrl={openUrl}
                  />

                  <div className="credits-section-heading">npm packages</div>
                  <div className="credits-filter-row">
                    <input
                      className="modal-input legal-input credits-filter-input"
                      type="search"
                      placeholder="Filter by package name or license..."
                      value={npmFilter}
                      onChange={(e) => setNpmFilter(e.target.value)}
                      aria-label="Filter npm package credits"
                    />
                    <span className="credits-filter-count">
                      {filteredNpm.length} / {npmCount}
                    </span>
                  </div>
                  <CreditsTable
                    rows={filteredNpm}
                    emptyLabel={
                      npmFilter.trim()
                        ? 'No npm packages match the current filter.'
                        : 'No npm packages listed.'
                    }
                    onOpenUrl={openUrl}
                  />
                </>
              )}
            </div>
          </div>
        </div>
      </FocusTrap>

      {licenseDialog && (
        <LicenseTextDialog
          title={licenseDialog.title}
          detail={licenseDialog.detail}
          body={licenseDialog.body}
          onClose={() => setLicenseDialog(null)}
        />
      )}
    </>
  );
}
