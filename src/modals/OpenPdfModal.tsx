import { useEffect, useId, useState, type KeyboardEvent } from 'react';
import { Modal } from '../ui/Modal';

type OpenPdfModalProps = {
  filePath: string;
  nativeDialogs: boolean;
  recentPdfs: string[];
  fileNameFromPath: (path: string) => string;
  onFilePathChange: (path: string) => void;
  onClose: () => void;
  onOpenPdfView: (path?: string) => void | Promise<void>;
  onOpenBirdsEye: (path?: string) => void | Promise<void>;
  openDirectlyInBirdsEye: boolean;
  onChooseNative: () => boolean | Promise<boolean>;
  onBrowse: () => void;
};

export function OpenPdfModal({
  filePath,
  nativeDialogs,
  recentPdfs,
  fileNameFromPath,
  onFilePathChange,
  onClose,
  onOpenPdfView,
  onOpenBirdsEye,
  openDirectlyInBirdsEye,
  onChooseNative,
  onBrowse,
}: OpenPdfModalProps) {
  const pathId = useId();
  const [step, setStep] = useState<'path' | 'view'>('path');
  const canContinue = !!filePath.trim();

  useEffect(() => {
    if (!canContinue) setStep('path');
  }, [canContinue]);

  useEffect(() => {
    if (openDirectlyInBirdsEye && step === 'view') setStep('path');
  }, [openDirectlyInBirdsEye, step]);

  const acceptPath = () => {
    if (!canContinue) return;
    if (openDirectlyInBirdsEye) {
      void onOpenBirdsEye();
    } else {
      setStep('view');
    }
  };

  const onFieldKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      acceptPath();
      e.currentTarget.blur();
    }
  };

  return (
    <Modal onClose={onClose}>
      <h3>Open PDF</h3>
      {step === 'path' ? (
        <>
          {!nativeDialogs && (
            <p className="modal-help">Native file picker is disabled for this session. Enter a path or use Browse….</p>
          )}
          <label htmlFor={pathId}>PDF path:</label>
          <div className="modal-path-row">
            <input
              id={pathId}
              type="text"
              value={filePath}
              onChange={(e) => onFilePathChange(e.target.value)}
              onKeyDown={onFieldKeyDown}
              className="modal-input"
              placeholder="/path/to/document.pdf"
              data-testid="open-pdf-path"
              autoFocus
            />
            {nativeDialogs && (
              <button
                onClick={() => {
                  void Promise.resolve(onChooseNative()).then((picked) => {
                    if (picked && !openDirectlyInBirdsEye) setStep('view');
                  });
                }}
                className="btn"
                data-testid="native-open-pdf"
              >
                Choose file…
              </button>
            )}
            <button onClick={onBrowse} className="btn">Browse…</button>
          </div>
          {recentPdfs.length > 0 && (
            <div className="recent-list" aria-label="Recently opened PDFs">
              <h4>Recently Opened</h4>
              {recentPdfs.map((path) => (
                <button
                  key={path}
                  className="recent-row"
                  onClick={() => {
                    onFilePathChange(path);
                    if (openDirectlyInBirdsEye) {
                      void onOpenBirdsEye(path);
                    } else {
                      setStep('view');
                    }
                  }}
                >
                  <span className="recent-name">{fileNameFromPath(path)}</span>
                  <span className="recent-path">{path}</span>
                </button>
              ))}
            </div>
          )}
        </>
      ) : (
        <div className="open-view-choice" aria-label="Choose how to open this PDF">
          <button type="button" className="open-view-card" data-testid="open-pdf-view" onClick={() => onOpenPdfView()}>
            <strong>PDF View</strong>
            <span>Open in the standard tabbed viewer.</span>
          </button>
          <button type="button" className="open-view-card" data-testid="open-birdseye-view" onClick={() => onOpenBirdsEye()}>
            <strong>Bird&apos;s Eye View</strong>
            <span>Open as a section for arranging pages across documents.</span>
          </button>
        </div>
      )}
      <div className="modal-actions">
        <button onClick={onClose} className="btn btn-secondary">Cancel</button>
        {step === 'view' && (
          <button onClick={() => setStep('path')} className="btn btn-secondary">Back</button>
        )}
        {step === 'path' && (
          <button
            onClick={acceptPath}
            className="btn"
            disabled={!canContinue}
            data-testid="open-pdf-submit"
          >
            {openDirectlyInBirdsEye ? 'Open' : 'Next'}
          </button>
        )}
      </div>
    </Modal>
  );
}
