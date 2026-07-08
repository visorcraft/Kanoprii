import { useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { useEscapeClose } from '../legal/useEscapeClose';
import { FocusTrap } from '../ui/FocusTrap';
import { openExternalUrl } from '../legal/openExternalUrl';

const REPO_URL = 'https://github.com/visorcraft/Kanoprii';

export function AboutModal({ onClose }: { onClose: () => void }) {
  const [version, setVersion] = useState<string | null>(null);

  useEscapeClose(onClose, true);

  useEffect(() => {
    let cancelled = false;
    void getVersion()
      .then((value) => {
        if (!cancelled) setVersion(value);
      })
      .catch(() => {
        if (!cancelled) setVersion(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const openRepo = () => {
    openExternalUrl(REPO_URL);
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <FocusTrap initialFocus={false}>
        <div
          className="modal about-modal"
          onClick={(e) => e.stopPropagation()}
          data-testid="about-modal"
        >
          <h3>About Kanoprii</h3>
          <p className="about-version">
            Version <span data-testid="about-version">{version ?? '…'}</span>
          </p>
          <p className="modal-help">
            A cross-platform desktop PDF editor. GPL-3.0-only - source and
            releases on GitHub.
          </p>
          <p className="about-repo">
            <button
              type="button"
              className="about-repo-link"
              onClick={openRepo}
              data-testid="about-repo-link"
            >
              github.com/visorcraft/Kanoprii
            </button>
          </p>
          <div className="modal-actions">
            <button type="button" className="btn btn-active" onClick={onClose}>
              Close
            </button>
          </div>
        </div>
      </FocusTrap>
    </div>
  );
}
