import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useState } from 'react';
import type { AnnotationData } from '../app/types';

type DocAnnotation = {
  page_index: number;
  index: number;
  data: AnnotationData;
};

type AnnotationsPanelProps = {
  filePath: string;
  pdfRevision: number;
  onGoToPage: (page: number) => void;
  onRemoveHighlight: (page: number, index: number) => void;
  onRemoveTextNote: (page: number, index: number) => void;
  onRemoveRedaction: (page: number, index: number) => void;
};

function annotLabel(data: AnnotationData): string {
  if (data.is_redaction) return 'Redaction';
  if (data.subtype === 'Highlight') return 'Highlight';
  if (data.subtype === 'Text') {
    const text = data.contents?.trim();
    return text ? `Note: "${text.length > 40 ? `${text.slice(0, 40)}…` : text}"` : 'Note';
  }
  if (data.subtype === 'Ink') return 'Ink stroke';
  if (data.subtype === 'Square' || data.subtype === 'Circle' || data.subtype === 'Line') return data.subtype;
  if (data.stamp_preset) return `Stamp: ${data.stamp_preset}`;
  return data.subtype || 'Annotation';
}

export function AnnotationsPanel({
  filePath,
  pdfRevision,
  onGoToPage,
  onRemoveHighlight,
  onRemoveTextNote,
  onRemoveRedaction,
}: AnnotationsPanelProps) {
  const [items, setItems] = useState<DocAnnotation[]>([]);

  const reload = useCallback(async () => {
    if (!filePath) {
      setItems([]);
      return;
    }
    try {
      const list = await invoke<DocAnnotation[]>('list_document_annotations', { path: filePath });
      setItems(list);
    } catch {
      setItems([]);
    }
  }, [filePath]);

  useEffect(() => {
    void reload();
  }, [reload, pdfRevision]);

  if (!filePath) return null;

  return (
    <div className="sidebar-panel annotations-panel" data-testid="annotations-panel">
      <h3>Annotations</h3>
      {items.length === 0 ? (
        <p className="muted" data-testid="annotations-empty">No annotations in this document.</p>
      ) : (
        <ul className="annotation-list">
          {items.map((item, i) => (
            <li
              key={`${item.page_index}-${item.index}-${item.data.subtype}-${i}`}
              className="annotation-row"
              data-testid="annotation-row"
            >
              <button type="button" className="annotation-row-main" onClick={() => onGoToPage(item.page_index)}>
                <span className="annotation-page">Page {item.page_index + 1}</span>
                <span className="annotation-label">{annotLabel(item.data)}</span>
              </button>
              {(item.data.subtype === 'Highlight' || item.data.subtype === 'Text' || item.data.is_redaction) && (
                <button
                  type="button"
                  className="annotation-delete"
                  title="Remove"
                  aria-label="Remove annotation"
                  onClick={() => {
                    if (item.data.is_redaction) onRemoveRedaction(item.page_index, item.index);
                    else if (item.data.subtype === 'Highlight') onRemoveHighlight(item.page_index, item.index);
                    else onRemoveTextNote(item.page_index, item.index);
                  }}
                >
                  ×
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
