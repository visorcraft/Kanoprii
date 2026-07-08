import { useState } from 'react';

export function useSecurityFormState() {
  const [showProtectModal, setShowProtectModal] = useState(false);
  const [showPasswordModal, setShowPasswordModal] = useState(false);
  const [pendingEncryptedPath, setPendingEncryptedPath] = useState('');
  const [protectUserPassword, setProtectUserPassword] = useState('');
  const [protectUserPasswordConfirm, setProtectUserPasswordConfirm] = useState('');
  const [protectOwnerPassword, setProtectOwnerPassword] = useState('');
  const [showSignModal, setShowSignModal] = useState(false);
  const [signCertPath, setSignCertPath] = useState('');
  const [signCertPassword, setSignCertPassword] = useState('');
  const [signReason, setSignReason] = useState('');
  const [signLocation, setSignLocation] = useState('');
  const [showMetadataModal, setShowMetadataModal] = useState(false);
  const [metadataTitle, setMetadataTitle] = useState('');
  const [metadataAuthor, setMetadataAuthor] = useState('');
  const [metadataSubject, setMetadataSubject] = useState('');
  const [metadataKeywords, setMetadataKeywords] = useState('');
  const [metadataCreator, setMetadataCreator] = useState('');
  const [metadataProducer, setMetadataProducer] = useState('');
  const [metadataCreationDate, setMetadataCreationDate] = useState('');
  const [metadataModDate, setMetadataModDate] = useState('');
  const [pdfPasswordDraft, setPdfPasswordDraft] = useState('');
  const [showDecryptModal, setShowDecryptModal] = useState(false);
  const [decryptPassword, setDecryptPassword] = useState('');

  return {
    showProtectModal, setShowProtectModal,
    showPasswordModal, setShowPasswordModal,
    pendingEncryptedPath, setPendingEncryptedPath,
    protectUserPassword, setProtectUserPassword,
    protectUserPasswordConfirm, setProtectUserPasswordConfirm,
    protectOwnerPassword, setProtectOwnerPassword,
    showSignModal, setShowSignModal,
    signCertPath, setSignCertPath,
    signCertPassword, setSignCertPassword,
    signReason, setSignReason,
    signLocation, setSignLocation,
    showMetadataModal, setShowMetadataModal,
    metadataTitle, setMetadataTitle,
    metadataAuthor, setMetadataAuthor,
    metadataSubject, setMetadataSubject,
    metadataKeywords, setMetadataKeywords,
    metadataCreator, setMetadataCreator,
    metadataProducer, setMetadataProducer,
    metadataCreationDate, setMetadataCreationDate,
    metadataModDate, setMetadataModDate,
    pdfPasswordDraft, setPdfPasswordDraft,
    showDecryptModal, setShowDecryptModal,
    decryptPassword, setDecryptPassword,
  };
}

/** Canonical alias for this hook's state shape. */
export type SecurityState = ReturnType<typeof useSecurityFormState>;
