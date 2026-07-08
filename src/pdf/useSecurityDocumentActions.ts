import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { PdfDocumentMetadata } from '../app/types';
import type { RunEdit } from './runEditTypes';

type UseSecurityDocumentActionsOptions = {
  filePath: string;
  originalPath: string;
  protectUserPassword: string;
  protectUserPasswordConfirm: string;
  protectOwnerPassword: string;
  decryptPassword: string;
  signCertPath: string;
  signCertPassword: string;
  signReason: string;
  signLocation: string;
  metadataTitle: string;
  metadataAuthor: string;
  metadataSubject: string;
  metadataKeywords: string;
  metadataCreator: string;
  metadataProducer: string;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  runEdit: RunEdit;
  loadPdfSignatures: (path: string) => Promise<void>;
  showToast: (msg: string, kind?: 'error') => void;
  setProtectUserPassword: (password: string) => void;
  setProtectUserPasswordConfirm: (password: string) => void;
  setProtectOwnerPassword: (password: string) => void;
  setShowProtectModal: (open: boolean) => void;
  setDecryptPassword: (password: string) => void;
  setShowDecryptModal: (open: boolean) => void;
  setSignCertPath: (path: string) => void;
  setSignCertPassword: (password: string) => void;
  setSignReason: (reason: string) => void;
  setSignLocation: (location: string) => void;
  setShowSignModal: (open: boolean) => void;
  setPdfRevision: React.Dispatch<React.SetStateAction<number>>;
  setMetadataTitle: (title: string) => void;
  setMetadataAuthor: (author: string) => void;
  setMetadataSubject: (subject: string) => void;
  setMetadataKeywords: (keywords: string) => void;
  setMetadataCreator: (creator: string) => void;
  setMetadataProducer: (producer: string) => void;
  setMetadataCreationDate: (date: string) => void;
  setMetadataModDate: (date: string) => void;
  setShowMetadataModal: (open: boolean) => void;
  setShowSignaturesPanel: React.Dispatch<React.SetStateAction<boolean>>;
};

export function useSecurityDocumentActions(opts: UseSecurityDocumentActionsOptions) {
  const openProtectModal = useCallback(() => {
    opts.setProtectUserPassword('');
    opts.setProtectUserPasswordConfirm('');
    opts.setProtectOwnerPassword('');
    opts.setShowProtectModal(true);
  }, [opts]);

  const openDecryptModal = useCallback(() => {
    opts.setDecryptPassword('');
    opts.setShowDecryptModal(true);
  }, [opts]);

  const handleRemovePdfPassword = useCallback(async () => {
    if (!opts.filePath || !opts.decryptPassword) return;
    const sourcePath = opts.originalPath || opts.filePath;
    await opts.withLoading(async () => {
      const written = await invoke<string>('remove_pdf_password', {
        path: sourcePath,
        password: opts.decryptPassword,
      });
      opts.setShowDecryptModal(false);
      opts.setDecryptPassword('');
      opts.showToast(`Saved decrypted copy to ${written}`);
    });
  }, [opts]);

  const openMetadataModal = useCallback(async (explicitPath?: string) => {
    const path = explicitPath ?? opts.filePath;
    if (!path) return;
    await opts.withLoading(async () => {
      const metadata = await invoke<PdfDocumentMetadata>('get_pdf_metadata', { path });
      opts.setMetadataTitle(metadata.title ?? '');
      opts.setMetadataAuthor(metadata.author ?? '');
      opts.setMetadataSubject(metadata.subject ?? '');
      opts.setMetadataKeywords(metadata.keywords ?? '');
      opts.setMetadataCreator(metadata.creator ?? '');
      opts.setMetadataProducer(metadata.producer ?? '');
      opts.setMetadataCreationDate(metadata.creation_date ?? '');
      opts.setMetadataModDate(metadata.mod_date ?? '');
      opts.setShowMetadataModal(true);
    });
  }, [opts]);

  const handleSaveMetadata = useCallback(async () => {
    await opts.runEdit({
      command: 'set_pdf_metadata',
      args: {
        title: opts.metadataTitle.trim() || null,
        author: opts.metadataAuthor.trim() || null,
        subject: opts.metadataSubject.trim() || null,
        keywords: opts.metadataKeywords.trim() || null,
        creator: opts.metadataCreator.trim() || null,
        producer: opts.metadataProducer.trim() || null,
      },
      skipReload: true,
      toast: 'Document metadata updated',
      onSuccess: () => opts.setShowMetadataModal(false),
    });
  }, [opts]);

  const handleProtectPdf = useCallback(async () => {
    if (!opts.filePath) return;
    const userPassword = opts.protectUserPassword;
    const confirm = opts.protectUserPasswordConfirm;
    if (!userPassword) {
      opts.showToast('User password is required', 'error');
      return;
    }
    if (userPassword !== confirm) {
      opts.showToast('Passwords do not match', 'error');
      return;
    }
    const ownerPassword = opts.protectOwnerPassword.trim();
    await opts.withLoading(async () => {
      const result = await invoke<string>('protect_pdf', {
        path: opts.filePath,
        userPassword,
        ownerPassword: ownerPassword || null,
      });
      opts.setShowProtectModal(false);
      opts.showToast(result);
    });
  }, [opts]);

  const openSignModal = useCallback(() => {
    opts.setSignCertPath('');
    opts.setSignCertPassword('');
    opts.setSignReason('');
    opts.setSignLocation('');
    opts.setShowSignModal(true);
  }, [opts]);

  const handleSignPdf = useCallback(async () => {
    if (!opts.filePath) return;
    const certPath = opts.signCertPath.trim();
    if (!certPath) {
      opts.showToast('Choose a PKCS#12 certificate (.p12/.pfx)', 'error');
      return;
    }
    if (!opts.signCertPassword) {
      opts.showToast('Certificate password is required', 'error');
      return;
    }
    await opts.withLoading(async () => {
      const result = await invoke<string>('sign_pdf', {
        path: opts.filePath,
        certPath,
        certPassword: opts.signCertPassword,
        reason: opts.signReason.trim() || null,
        location: opts.signLocation.trim() || null,
        fieldName: null,
        outputPath: null,
      });
      opts.markPdfEdited();
      opts.setShowSignModal(false);
      opts.setPdfRevision((r) => r + 1);
      await opts.loadPdfSignatures(opts.filePath);
      opts.showToast(result);
    });
  }, [opts]);

  const toggleSignaturesPanel = useCallback(() => {
    opts.setShowSignaturesPanel((prev) => !prev);
  }, [opts]);

  return {
    openProtectModal,
    openDecryptModal,
    handleRemovePdfPassword,
    openMetadataModal,
    handleSaveMetadata,
    handleProtectPdf,
    openSignModal,
    handleSignPdf,
    toggleSignaturesPanel,
  };
}
