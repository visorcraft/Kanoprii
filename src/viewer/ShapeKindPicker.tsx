import type { ShapeKind } from '../app/constants';

const SHAPE_OPTIONS: Array<{ kind: ShapeKind; label: string }> = [
  { kind: 'square', label: 'Rect' },
  { kind: 'circle', label: 'Ellipse' },
  { kind: 'line', label: 'Line' },
  { kind: 'arrow', label: 'Arrow' },
];

type ShapeKindPickerProps = {
  value: ShapeKind;
  onChange: (kind: ShapeKind) => void;
  ariaLabel: string;
  className?: string;
};

export function ShapeKindPicker({ value, onChange, ariaLabel, className = '' }: ShapeKindPickerProps) {
  return (
    <div className={`shape-kind-toggle ${className}`.trim()} role="group" aria-label={ariaLabel}>
      {SHAPE_OPTIONS.map(({ kind, label }) => (
        <button
          key={kind}
          type="button"
          className={value === kind ? 'active' : ''}
          aria-pressed={value === kind}
          onClick={() => onChange(kind)}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
