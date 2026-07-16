import { useEffect, useRef } from 'react';
import { VIEWER_PAGE_H, VIEWER_PAGE_W } from '../app/constants';

type TextEditTarget = {
  text: string;
  x: number;
  y: number;
  w: number;
  h: number;
};

type TextEditOverlayProps = {
  target: TextEditTarget;
  draft: string;
  onDraftChange: (value: string) => void;
  onApply: () => void;
  onCancel: () => void;
};

export function TextEditOverlay({
  target,
  draft,
  onDraftChange,
  onApply,
  onCancel,
}: TextEditOverlayProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, [target.text]);

  return (
    <input
      ref={inputRef}
      type="text"
      className="text-edit-overlay-input"
      value={draft}
      style={{
        position: 'absolute',
        left: target.x,
        top: target.y,
        width: Math.max(40, Math.min(Math.max(target.w, 320), VIEWER_PAGE_W - target.x)),
        height: Math.max(44, Math.min(Math.max(target.h, 44), VIEWER_PAGE_H - target.y)),
        fontSize: Math.max(14, target.h * 0.85),
      }}
      onChange={(e) => onDraftChange(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          onApply();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          onCancel();
        }
      }}
      onBlur={onApply}
    />
  );
}
