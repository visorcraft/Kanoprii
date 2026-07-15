import { TextStyle } from '../app/usePdfEditState';
import './EditToolbar.css';

export interface EditToolbarProps {
  style: TextStyle;
  onChange: (patch: Partial<TextStyle>) => void;
  onApply: () => void;
  onCancel: () => void;
}

function rgbToHex(color: TextStyle['color']): string {
  const toHex = (n: number) =>
    Math.max(0, Math.min(255, Math.round(n)))
      .toString(16)
      .padStart(2, '0');
  return `#${toHex(color.r)}${toHex(color.g)}${toHex(color.b)}`;
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const clean = hex.replaceAll('#', '').toLowerCase();
  const expanded =
    clean.length === 3
      ? clean
          .split('')
          .map((c) => c + c)
          .join('')
      : clean.slice(0, 6);
  return {
    r: parseInt(expanded.slice(0, 2), 16) || 0,
    g: parseInt(expanded.slice(2, 4), 16) || 0,
    b: parseInt(expanded.slice(4, 6), 16) || 0,
  };
}

export function EditToolbar({
  style,
  onChange,
  onApply,
  onCancel,
}: EditToolbarProps) {
  return (
    <div className="edit-toolbar" aria-label="Text editing toolbar">
      <select
        className="edit-toolbar-select"
        value={style.fontFamily}
        onChange={(e) =>
          onChange({ fontFamily: e.target.value as TextStyle['fontFamily'] })
        }
        aria-label="Font family"
      >
        <option value="Helvetica">Helvetica</option>
        <option value="LiberationSans">Liberation Sans</option>
        <option value="Courier">Courier</option>
      </select>

      <input
        className="edit-toolbar-number"
        type="number"
        min={6}
        max={144}
        value={style.fontSize}
        onChange={(e) => onChange({ fontSize: Number(e.target.value) })}
        aria-label="Font size"
      />

      <div
        className="edit-toolbar-group"
        role="group"
        aria-label="Text decoration"
      >
        <button
          type="button"
          aria-pressed={style.bold}
          className={style.bold ? 'active' : ''}
          onClick={() => onChange({ bold: !style.bold })}
          aria-label="Bold"
          title="Bold"
        >
          B
        </button>
        <button
          type="button"
          aria-pressed={style.italic}
          className={style.italic ? 'active' : ''}
          onClick={() => onChange({ italic: !style.italic })}
          aria-label="Italic"
          title="Italic"
        >
          I
        </button>
        <button
          type="button"
          aria-pressed={style.underline}
          className={style.underline ? 'active' : ''}
          onClick={() => onChange({ underline: !style.underline })}
          aria-label="Underline"
          title="Underline"
        >
          U
        </button>
      </div>

      <input
        className="edit-toolbar-color"
        type="color"
        value={rgbToHex(style.color)}
        onChange={(e) => onChange({ color: hexToRgb(e.target.value) })}
        aria-label="Text color"
        title="Text color"
      />

      <div
        className="edit-toolbar-group"
        role="group"
        aria-label="Text alignment"
      >
        <button
          type="button"
          aria-pressed={style.align === 'left'}
          className={style.align === 'left' ? 'active' : ''}
          onClick={() => onChange({ align: 'left' })}
          aria-label="Align left"
          title="Align left"
        >
          Left
        </button>
        <button
          type="button"
          aria-pressed={style.align === 'center'}
          className={style.align === 'center' ? 'active' : ''}
          onClick={() => onChange({ align: 'center' })}
          aria-label="Align center"
          title="Align center"
        >
          Center
        </button>
        <button
          type="button"
          aria-pressed={style.align === 'right'}
          className={style.align === 'right' ? 'active' : ''}
          onClick={() => onChange({ align: 'right' })}
          aria-label="Align right"
          title="Align right"
        >
          Right
        </button>
      </div>

      <div
        className="edit-toolbar-group edit-toolbar-actions"
        role="group"
        aria-label="Edit actions"
      >
        <button type="button" className="edit-toolbar-apply" onClick={onApply}>
          Apply
        </button>
        <button
          type="button"
          className="edit-toolbar-cancel"
          onClick={onCancel}
        >
          Cancel
        </button>
      </div>
    </div>
  );
}
