import { useState } from 'react';
import type { FormFieldData, PdfBookmarkEntry, PdfSignatureInfo, PdfSignatureVerificationSummary } from './types';

export function useDocumentPanelsState() {
  const [showSignaturesPanel, setShowSignaturesPanel] = useState(false);
  const [pdfSignatures, setPdfSignatures] = useState<PdfSignatureInfo[]>([]);
  const [signatureVerification, setSignatureVerification] = useState<PdfSignatureVerificationSummary | null>(null);
  const [showBookmarksPanel, setShowBookmarksPanel] = useState(false);
  const [pdfBookmarks, setPdfBookmarks] = useState<PdfBookmarkEntry[]>([]);
  const [showAnnotationsPanel, setShowAnnotationsPanel] = useState(false);
  const [showFormsPanel, setShowFormsPanel] = useState(false);
  const [showPdfUaPanel, setShowPdfUaPanel] = useState(false);
  const [showHiddenLayers, setShowHiddenLayers] = useState(false);
  const [showSidebar, setShowSidebar] = useState(true);
  const [formFields, setFormFields] = useState<FormFieldData[]>([]);
  const [formDrafts, setFormDrafts] = useState<Record<string, string>>({});

  return {
    showSignaturesPanel, setShowSignaturesPanel,
    pdfSignatures, setPdfSignatures,
    signatureVerification, setSignatureVerification,
    showBookmarksPanel, setShowBookmarksPanel,
    pdfBookmarks, setPdfBookmarks,
    showAnnotationsPanel, setShowAnnotationsPanel,
    showFormsPanel, setShowFormsPanel,
    showPdfUaPanel, setShowPdfUaPanel,
    showHiddenLayers, setShowHiddenLayers,
    showSidebar, setShowSidebar,
    formFields, setFormFields,
    formDrafts, setFormDrafts,
  };
}

/** Canonical alias for this hook's state shape. */
export type PanelsState = ReturnType<typeof useDocumentPanelsState>;
