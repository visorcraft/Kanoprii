import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { Modal } from '../ui/Modal';

type UpdatePhase = 'idle' | 'checking' | 'current' | 'available' | 'downloading' | 'ready' | 'handoff' | 'error';

type LinuxPackageRef = { url: string; sha256: string };

type LatestVersion = {
  version: string;
  notes?: string;
  current: string;
  newer: boolean;
  linuxPackages?: { deb?: LinuxPackageRef; rpm?: LinuxPackageRef };
};

type UpdateModalProps = {
  onClose: () => void;
  updaterSupported: boolean;
};

export function UpdateModal({ onClose, updaterSupported }: UpdateModalProps) {
  const [phase, setPhase] = useState<UpdatePhase>('checking');
  const [update, setUpdate] = useState<Update | null>(null);
  const [latest, setLatest] = useState<LatestVersion | null>(null);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState('');
  const [channel, setChannel] = useState<string>('manual');

  const runCheck = useCallback(async () => {
    setPhase('checking');
    setError('');
    setUpdate(null);
    setLatest(null);

    if (updaterSupported) {
      try {
        const result = await check();
        if (!result) {
          setPhase('current');
          return;
        }
        setUpdate(result);
        setPhase('available');
      } catch (err) {
        setError(String(err));
        setPhase('error');
      }
      return;
    }

    // Check-only path for unsupported platforms (deb/rpm/raw binary)
    try {
      const [result, ch] = await Promise.all([
        invoke<LatestVersion>('fetch_latest_version'),
        invoke<string>('update_channel').catch(() => 'manual'),
      ]);
      setChannel(ch);
      setLatest(result);
      setPhase(result.newer ? 'available' : 'current');
    } catch (err) {
      setError(String(err));
      setPhase('error');
    }
  }, [updaterSupported]);

  useEffect(() => {
    void runCheck();
  }, [runCheck]);

  const download = async () => {
    if (!update) return;
    setPhase('downloading');
    setProgress(0);
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === 'Progress') {
          setProgress((prev) => (event.data.chunkLength > 0 ? Math.min(99, prev + 1) : prev));
        }
      });
      setPhase('ready');
    } catch (err) {
      setError(String(err));
      setPhase('error');
    }
  };

  const restart = async () => {
    await relaunch();
  };

  const openReleasePage = () => {
    const url = latest
      ? `https://github.com/visorcraft/Kanoprii/releases/tag/v${latest.version}`
      : 'https://github.com/visorcraft/Kanoprii/releases';
    void invoke('open_url', { url });
  };

  const packageForChannel = (): LinuxPackageRef | undefined => {
    if (!latest?.linuxPackages) return undefined;
    if (channel === 'deb') return latest.linuxPackages.deb;
    if (channel === 'rpm') return latest.linuxPackages.rpm;
    return undefined;
  };

  const installPackage = async () => {
    const pkg = packageForChannel();
    if (!pkg) return;
    setPhase('downloading');
    setProgress(0);
    try {
      await invoke<string>('download_and_open_package', { url: pkg.url, sha256: pkg.sha256 });
      setPhase('handoff');
    } catch (err) {
      setError(String(err));
      setPhase('error');
    }
  };

  return (
    <Modal onClose={onClose}>
      <div data-testid="update-modal">
        <h3>Check for Updates</h3>
      <div className="update-modal-body">
        {phase === 'checking' && <p>Checking for updates…</p>}
        {phase === 'current' && <p>Kanoprii is up to date.</p>}
        {phase === 'available' && (
          <>
            {update && (
              <>
                <p>
                  Version <strong>{update.version}</strong> is available.
                </p>
                {update.body && <pre className="update-notes">{update.body}</pre>}
                <div className="modal-actions">
                  <button type="button" className="btn btn-active" onClick={() => void download()}>
                    Download
                  </button>
                  <button type="button" className="btn btn-secondary" onClick={onClose}>
                    Later
                  </button>
                </div>
              </>
            )}
            {latest && (
              <>
                <p>
                  Version <strong>{latest.version}</strong> is available.
                </p>
                {latest.notes && <pre className="update-notes">{latest.notes}</pre>}
                {packageForChannel() ? (
                  <>
                    <p className="muted">
                      Download the {channel === 'deb' ? 'Debian' : 'RPM'} package and install it with your
                      system package manager.
                    </p>
                    <div className="modal-actions">
                      <button type="button" className="btn btn-active" onClick={() => void installPackage()}>
                        Download &amp; Install
                      </button>
                      <button type="button" className="btn btn-secondary" onClick={onClose}>
                        Later
                      </button>
                    </div>
                  </>
                ) : (
                  <>
                    <p className="muted">
                      Your platform does not support in-app updates. Open the release page to download manually.
                    </p>
                    <div className="modal-actions">
                      <button type="button" className="btn btn-active" onClick={() => void openReleasePage()}>
                        Open Release Page
                      </button>
                      <button type="button" className="btn btn-secondary" onClick={onClose}>
                        Later
                      </button>
                    </div>
                  </>
                )}
              </>
            )}
          </>
        )}
        {phase === 'downloading' && <p>Downloading… {progress > 0 ? `${progress}%` : ''}</p>}
        {phase === 'ready' && (
          <div className="modal-actions">
            <button type="button" className="btn btn-active" onClick={() => void restart()}>
              Restart Now
            </button>
            <button type="button" className="btn btn-secondary" onClick={onClose}>
              Later
            </button>
          </div>
        )}
        {phase === 'handoff' && (
          <>
            <p>The system installer has been launched. Restart Kanoprii after the update finishes.</p>
            <div className="modal-actions">
              <button type="button" className="btn btn-secondary" onClick={onClose}>
                Close
              </button>
            </div>
          </>
        )}
        {phase === 'error' && (
          <>
            <p className="error-text">
              {error?.toLowerCase().includes('release json')
                ? 'Could not fetch the release manifest. The latest release may not have packages attached yet.'
                : error || 'Update check failed.'}
            </p>
            <div className="modal-actions">
              <button type="button" className="btn btn-active" onClick={() => void runCheck()}>
                Retry
              </button>
              <button type="button" className="btn btn-secondary" onClick={() => void openReleasePage()}>
                Open Release Page
              </button>
              <button type="button" className="btn btn-secondary" onClick={onClose}>
                Close
              </button>
            </div>
          </>
        )}
      </div>
      </div>
    </Modal>
  );
}

export async function fetchUpdaterSupported(): Promise<boolean> {
  try {
    return await invoke<boolean>('updater_supported');
  } catch {
    return false;
  }
}
