import React, { useEffect } from 'react';
import { useEscapeClose } from '../legal/useEscapeClose';
import { FocusTrap } from './FocusTrap';

type ModalProps = {
  children: React.ReactNode;
  onClose: () => void;
  'aria-label'?: string;
  'aria-labelledby'?: string;
  'data-testid'?: string;
};

let openModalCount = 0;

export function Modal({ children, onClose, 'data-testid': dataTestId, ...aria }: ModalProps) {
  useEscapeClose(onClose, true);
  useEffect(() => {
    openModalCount += 1;
    document.body.classList.add('kanoprii-modal-open');
    return () => {
      openModalCount -= 1;
      if (openModalCount <= 0) {
        openModalCount = 0;
        document.body.classList.remove('kanoprii-modal-open');
      }
    };
  }, []);
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <FocusTrap>
        <div
          className="modal"
          onClick={(e) => e.stopPropagation()}
          role="dialog"
          aria-modal="true"
          data-testid={dataTestId}
          {...aria}
        >
          {children}
        </div>
      </FocusTrap>
    </div>
  );
}
