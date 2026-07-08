import type { PageTextRun } from '../pdf/useTextLayerLoader';

type TextLayerProps = {
  runs: PageTextRun[];
  interactive: boolean;
};

export function TextLayer({ runs, interactive }: TextLayerProps) {
  if (runs.length === 0) return null;

  return (
    <div
      className="text-layer"
      data-testid="text-layer"
      style={{
        position: 'absolute',
        left: 0,
        top: 0,
        width: '100%',
        height: '100%',
        pointerEvents: interactive ? 'auto' : 'none',
        userSelect: interactive ? 'text' : 'none',
      }}
      aria-hidden={!interactive}
    >
      {runs.map((run, i) => (
        <span
          key={`${i}-${run.x}-${run.y}-${run.text.slice(0, 8)}`}
          style={{
            position: 'absolute',
            left: run.x,
            top: run.y,
            width: run.w,
            height: run.h,
            fontSize: run.h,
            lineHeight: 1,
            color: 'transparent',
            whiteSpace: 'pre',
            overflow: 'hidden',
          }}
        >
          {run.text}
        </span>
      ))}
    </div>
  );
}
