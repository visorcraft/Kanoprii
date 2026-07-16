import { useEffect, useRef } from 'react';

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
        width: Math.max(target.w, 120),
        height: Math.max(target.h, 24),
        fontSize: target.h * 0.85,
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
