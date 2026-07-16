import type { ReactNode } from 'react';

export type RibbonIconName =
  | 'save' | 'undo' | 'redo' | 'find' | 'select' | 'rotate' | 'duplicate'
  | 'highlight' | 'note' | 'draw' | 'shape' | 'stamp' | 'redact'
  | 'move' | 'insert' | 'delete' | 'split' | 'combine' | 'keep' | 'sort' | 'parity'
  | 'optimize' | 'ocr' | 'redactions' | 'summarize'
  | 'page-numbers' | 'bates' | 'header' | 'footer' | 'page-size' | 'watermark' | 'border' | 'crop' | 'flatten'
  | 'metadata' | 'sign' | 'panel'
  | 'view-pdf' | 'view-birdseye' | 'view-markdown'
  | 'continuous' | 'hidden-layers' | 'pdfua'
  | 'chevron-down' | 'chevron-up';

const PATHS: Record<RibbonIconName, ReactNode> = {
  save: <><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><path d="M17 21v-8H7v8M7 3v5h8"/></>,
  undo: <><path d="m9 14-5-5 5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11"/></>,
  redo: <><path d="m15 14 5-5-5-5"/><path d="M20 9H9.5a5.5 5.5 0 0 0 0 11H13"/></>,
  find: <><circle cx="11" cy="11" r="7"/><path d="m21 21-4.3-4.3"/></>,
  select: <path d="m3 3 7.07 16.97 2.51-7.39 7.39-2.51z"/>,
  rotate: <><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/></>,
  duplicate: <><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></>,
  highlight: <><path d="m9 11-6 6v3h9l3-3"/><path d="m22 12-4.6 4.6a2 2 0 0 1-2.8 0l-5.2-5.2a2 2 0 0 1 0-2.8L14 4z"/></>,
  note: <><path d="M15.5 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V8.5z"/><path d="M15 3v6h6"/></>,
  draw: <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5z"/>,
  shape: <><circle cx="8.5" cy="8.5" r="5.5"/><rect x="13" y="13" width="8" height="8" rx="1"/></>,
  stamp: <><path d="M5 22h14"/><path d="M19.27 13.73A2.5 2.5 0 0 0 17.5 13h-11a2.5 2.5 0 0 0-2.5 2.5V16h16v-2.27z"/><path d="M14 13V8.5C14 7 15 7 15 5a3 3 0 0 0-6 0c0 2 1 2 1 3.5V13"/></>,
  redact: <><rect x="3" y="9" width="18" height="6" rx="1"/><path d="M7 4h10M7 20h10"/></>,
  move: <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3"/>,
  insert: <><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M12 8v8M8 12h8"/></>,
  delete: <path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2m3 0v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/>,
  split: <><circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M20 4 8.12 15.88M14.47 14.48 20 20M8.12 8.12 12 12"/></>,
  combine: <path d="M8 3v7a4 4 0 0 0 8 0V3M5 21h14"/>,
  keep: <path d="M4 6h16M7 12h10M10 18h4"/>,
  sort: <><path d="m3 8 4-4 4 4M7 4v16M21 16l-4 4-4-4M17 20V4"/></>,
  parity: <><path d="M4 5h16M4 12h16M4 19h16"/><path d="M9 5v14"/></>,
  optimize: <path d="M13 2 3 14h9l-1 8 10-12h-9l1-8z"/>,
  ocr: <><circle cx="11" cy="11" r="7"/><path d="m21 21-4.3-4.3"/><path d="M8 11h6M11 8v6"/></>,
  redactions: <><rect x="3" y="9" width="18" height="6" rx="1"/><path d="m4 6 16 12M20 6 4 18"/></>,
  summarize: <><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6M8 13h8M8 17h5"/></>,
  'page-numbers': <path d="M4 9h16M4 15h16M10 3 8 21M16 3l-2 18"/>,
  bates: <path d="M4 7V4h16v3M9 20h6M12 4v16"/>,
  header: <><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18"/></>,
  footer: <><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 15h18"/></>,
  'page-size': <path d="M21 3 3 21M21 3h-6m6 0v6M3 21h6m-6 0v-6"/>,
  watermark: <path d="M12 2.7 6.5 9.1a7 7 0 1 0 11 0z"/>,
  border: <><rect x="3" y="3" width="18" height="18" rx="2"/><rect x="7" y="7" width="10" height="10" rx="1"/></>,
  crop: <><path d="M6 2v14a2 2 0 0 0 2 2h14"/><path d="M18 22V8a2 2 0 0 0-2-2H2"/></>,
  flatten: <><path d="m12 2 10 6.5-10 6.5L2 8.5z"/><path d="m2 15.5 10 6.5 10-6.5"/></>,
  metadata: <><circle cx="12" cy="12" r="9"/><path d="M12 8h.01M11 12h1v4h1"/></>,
  sign: <><path d="m17 3-9.5 9.5L5 19l6.5-2.5L21 7a2.12 2.12 0 0 0-4-4z"/><path d="m15 5 4 4"/></>,
  panel: <><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18"/></>,
  'view-pdf': <><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/></>,
  'view-birdseye': <><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></>,
  'view-markdown': <><path d="M4 5h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z"/><path d="M7 15V9l2.5 3L12 9v6M16.5 9v4.5M14.5 13l2 2 2-2"/></>,
  continuous: <path d="M12 3v18M8 7l4-4 4 4M8 17l4 4 4-4"/>,
  'hidden-layers': <><path d="M9.9 4.24A9 9 0 0 1 12 4c7 0 10 8 10 8a18.5 18.5 0 0 1-2.16 3.19M6.61 6.61A18.6 18.6 0 0 0 2 12s3 8 10 8a9.87 9.87 0 0 0 5.39-1.61"/><path d="m2 2 20 20"/><path d="M9.88 9.88a3 3 0 1 0 4.24 4.24"/></>,
  pdfua: <><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1"/><path d="m9 13 2 2 4-4"/></>,
  'chevron-down': <path d="m6 9 6 6 6-6"/>,
  'chevron-up': <path d="m18 15-6-6-6 6"/>,
};

export function RibbonIcon({
  name,
  className = 'ribbon-btn-icon',
}: {
  name: RibbonIconName;
  className?: string;
}) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      {PATHS[name]}
    </svg>
  );
}
