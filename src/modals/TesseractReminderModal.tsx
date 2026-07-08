import { Modal } from '../ui/Modal';

export type TesseractInstallGuide = {
  platform: string;
  summary: string;
  steps: string[];
  installCommand: string | null;
  downloadUrl: string | null;
  licenseNote: string;
};

type TesseractReminderModalProps = {
  guide: TesseractInstallGuide;
  doNotRemind: boolean;
  onDoNotRemindChange: (checked: boolean) => void;
  onClose: () => void;
  onCopyInstallCommand: () => void;
};

export function TesseractReminderModal({
  guide,
  doNotRemind,
  onDoNotRemindChange,
  onClose,
  onCopyInstallCommand,
}: TesseractReminderModalProps) {
  return (
    <Modal onClose={onClose} aria-label="Tesseract installation reminder">
      <h3>Read text from scanned PDFs (optional)</h3>
      <p className="modal-help">{guide.summary}</p>
      <p className="modal-help">{guide.licenseNote}</p>
      <ol className="modal-steps">
        {guide.steps.map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ol>
      {guide.installCommand && (
        <>
          <label htmlFor="tesseract-install-command">Install command</label>
          <div className="modal-path-row">
            <input
              id="tesseract-install-command"
              type="text"
              readOnly
              value={guide.installCommand}
              className="modal-input"
            />
            <button type="button" className="btn" onClick={onCopyInstallCommand}>Copy</button>
          </div>
        </>
      )}
      {guide.downloadUrl && (
        <p className="modal-help">
          <a href={guide.downloadUrl} target="_blank" rel="noreferrer">
            {guide.platform === 'windows'
              ? 'Download Tesseract for Windows'
              : 'Tesseract project page'}
          </a>
        </p>
      )}
      <div className="modal-actions modal-actions-split">
        <label className="modal-checkbox-row">
          <input
            type="checkbox"
            checked={doNotRemind}
            onChange={(e) => onDoNotRemindChange(e.target.checked)}
          />
          <span>Do not remind me again</span>
        </label>
        <button type="button" onClick={onClose} className="btn btn-secondary" data-testid="tesseract-reminder-close">Close</button>
      </div>
    </Modal>
  );
}
