import type { PageTextEditItem } from '../modals/PageEditsModal';
import type { PdfAnnotation } from '../pdf/usePdfDocument';
import type { PageRangeScope } from '../pageRange/types';

export type AnnotationData = PdfAnnotation & {
  is_redaction: boolean;
};

export interface FormFieldData {
  name: string;
  field_type: string;
  value: string;
  page_index: number | null;
  rect: [number, number, number, number] | null;
  options: string[];
  checked: boolean;
}

export type ViewMode = 'pdf' | 'markdown';
export type WorkspaceViewMode = 'tabs' | 'birdseye';
export type ScrollViewMode = 'single' | 'continuous';

export interface MarkdownSaveResult {
  markdown: string;
  markdownPath: string;
  written: boolean;
  conflict: boolean;
  ocrAvailable: boolean;
  ocrLanguage: string;
  pagesNeedingOcr: number;
  ocrTextBlocks: number;
  ocrMissingHints: number;
}

export interface MarkdownOcrNotice {
  tone: 'success' | 'warning';
  message: string;
}

export interface PdfIntelligentExtraction {
  headings: string[];
  emails: string[];
  urls: string[];
  dates: string[];
}

export interface PdfSummaryResult {
  pageCount: number;
  wordCount: number;
  titleGuess: string | null;
  overview: string;
  keyPoints: string[];
  extraction: PdfIntelligentExtraction;
  scannedPages: number;
}

export interface SummarySaveResult {
  summary: PdfSummaryResult;
  summaryPath: string;
  written: boolean;
  conflict: boolean;
}

export type PageTextEdit = PageTextEditItem;

export interface PageVectorEdit {
  index: number;
  x: number;
  y: number;
  width: number;
  height: number;
  kind: string;
}

export interface PdfSignatureInfo {
  field_name: string;
  signer_name: string | null;
  reason: string | null;
  location: string | null;
  signing_time: string | null;
  sub_filter: string | null;
  signed_percent: number | null;
}

export interface PdfSignatureVerificationEntry {
  field_name: string;
  status: string;
  signer_name: string | null;
  signing_time: string | null;
  integrity_ok: boolean;
  modifications_after_signing: boolean;
  summary: string;
}

export interface PdfSignatureVerificationSummary {
  signature_count: number;
  valid_count: number;
  invalid_count: number;
  document_modified: boolean;
  overall_valid: boolean;
  summary: string;
  signatures: PdfSignatureVerificationEntry[];
}

export interface PdfBookmarkEntry {
  title: string;
  depth: number;
  page_index: number | null;
}

export interface PdfUaReport {
  tagged: boolean;
  hasTitle: boolean;
  language: string | null;
  figuresTotal: number;
  figuresWithAlt: number;
  imageXobjects: number;
  pageCount: number;
  encrypted: boolean;
}

export interface PdfPageSize {
  width: number;
  height: number;
  rotation: number;
}

export interface PdfDocumentMetadata {
  title: string | null;
  author: string | null;
  subject: string | null;
  keywords: string | null;
  creator: string | null;
  producer: string | null;
  creation_date: string | null;
  mod_date: string | null;
}

export type PdfBrowserTarget = 'open' | 'insert' | 'merge' | 'replace' | 'interleave' | 'prepend';
export type PngExportScope = PageRangeScope;
