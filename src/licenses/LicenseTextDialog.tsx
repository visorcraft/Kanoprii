import { useEscapeClose } from '../legal/useEscapeClose';
import { FocusTrap } from '../ui/FocusTrap';

type LicenseTextDialogProps = {
  title: string;
  detail: string;
  body: string;
  onClose: () => void;
};

export function LicenseTextDialog({
  title,
  detail,
  body,
  onClose,
}: LicenseTextDialogProps) {
  useEscapeClose(onClose, true);

  return (
    <div className="modal-backdrop licenses-gpl-backdrop" onClick={onClose}>
      <FocusTrap>
        <div
          className="modal licenses-gpl-dialog"
          onClick={(e) => e.stopPropagation()}
        >
          <h3>{title}</h3>
          <p className="modal-help">{detail}</p>
          <textarea
            className="licenses-gpl-dialog-body"
            readOnly
            value={body}
          />
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
