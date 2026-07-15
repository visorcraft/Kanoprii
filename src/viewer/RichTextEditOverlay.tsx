import { useEffect, useRef } from 'react';
import type { TextEditDraft } from '../app/usePdfEditState';
import { EditToolbar } from './EditToolbar';
import './RichTextEditOverlay.css';

type RichTextEditOverlayProps = {
  draft: TextEditDraft;
  onUpdate: (patch: Partial<TextEditDraft>) => void;
  onApply: () => void;
  onCancel: () => void;
};

export function RichTextEditOverlay({
  draft,
  onUpdate,
  onApply,
  onCancel,
}: RichTextEditOverlayProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    textareaRef.current?.focus();
    textareaRef.current?.select();
  }, []);

  const { pageRect, style } = draft;

  return (
    <div
      className="rich-text-edit-overlay"
      style={{
        position: 'absolute',
        left: pageRect.x,
        top: pageRect.y,
        width: pageRect.w,
        height: pageRect.h,
      }}
    >
      <EditToolbar
        style={style}
        onChange={(patch) => onUpdate({ style: { ...style, ...patch } })}
        onApply={onApply}
        onCancel={onCancel}
      />
      <textarea
        ref={textareaRef}
        className="rich-text-edit-textarea"
        value={draft.text}
        onChange={(e) => onUpdate({ text: e.target.value })}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            onApply();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            onCancel();
          }
        }}
        style={{
          fontFamily:
            style.fontFamily === 'LiberationSans'
              ? 'Liberation Sans, sans-serif'
              : style.fontFamily,
          fontSize: style.fontSize,
          fontWeight: style.bold ? 'bold' : 'normal',
          fontStyle: style.italic ? 'italic' : 'normal',
          textDecoration: style.underline ? 'underline' : 'none',
          color: `rgb(${style.color.r}, ${style.color.g}, ${style.color.b})`,
          textAlign: style.align,
        }}
      />
    </div>
  );
}
