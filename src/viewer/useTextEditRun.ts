import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { createStructuralEditRunner } from '../pdf/runStructuralEdit';
import type { PageTextRun } from '../pdf/useTextLayerLoader';

export type TextLineInfo = {
  text: string;
  x: number;
  y: number;
  w: number;
  h: number;
};

type ActiveEditTarget =
  | { kind: 'line'; index: number; data: TextLineInfo }
  | { kind: 'run'; data: PageTextRun };

type UseTextEditRunOptions = {
  filePath: string;
  currentPage: number;
  pdfRevision: number;
  editTextRunMode: boolean;
  runEdit: ReturnType<typeof createStructuralEditRunner>;
};

const MAX_LINES_CACHE = 32;

function trimLinesCache(cache: Map<string, TextLineInfo[]>) {
  while (cache.size > MAX_LINES_CACHE) {
    const oldest = cache.keys().next().value;
    if (oldest === undefined) break;
    cache.delete(oldest);
  }
}

export function useTextEditRun(opts: UseTextEditRunOptions) {
  const [activeTarget, setActiveTarget] = useState<ActiveEditTarget | null>(null);
  const [draft, setDraft] = useState('');
  const [lines, setLines] = useState<TextLineInfo[]>([]);
  const linesCacheRef = useRef(new Map<string, TextLineInfo[]>());

  useEffect(() => {
    linesCacheRef.current.clear();
  }, [opts.filePath, opts.pdfRevision]);

  // Load decoded text lines when entering edit mode.
  useEffect(() => {
    if (!opts.editTextRunMode || !opts.filePath) {
      setLines([]);
      return;
    }
    const cacheKey = `${opts.filePath}\0${opts.currentPage}\0${opts.pdfRevision}`;
    const cached = linesCacheRef.current.get(cacheKey);
    if (cached) {
      setLines(cached);
      return;
    }
    void (async () => {
      try {
        const result = await invoke<TextLineInfo[]>('get_page_text_lines', {
          path: opts.filePath,
          pageIndex: opts.currentPage,
        });
        linesCacheRef.current.set(cacheKey, result);
        trimLinesCache(linesCacheRef.current);
        setLines(result);
      } catch {
        setLines([]);
      }
    })();
  }, [opts.editTextRunMode, opts.filePath, opts.currentPage, opts.pdfRevision]);

  const hitTestLine = useCallback(
    (x: number, y: number): TextLineInfo | null => {
      for (let i = lines.length - 1; i >= 0; i -= 1) {
        const line = lines[i]!;
        if (x >= line.x && x <= line.x + line.w && y >= line.y && y <= line.y + line.h) {
          return line;
        }
      }
      return null;
    },
    [lines],
  );

  const hitTestRun = useCallback(
    (runs: PageTextRun[], x: number, y: number): PageTextRun | null => {
      for (let i = runs.length - 1; i >= 0; i -= 1) {
        const run = runs[i]!;
        if (x >= run.x && x <= run.x + run.w && y >= run.y && y <= run.y + run.h) {
          return run;
        }
      }
      return null;
    },
    [],
  );

  const openLineEditor = useCallback((index: number, line: TextLineInfo) => {
    setActiveTarget({ kind: 'line', index, data: line });
    setDraft(line.text);
  }, []);

  const openRunEditor = useCallback((run: PageTextRun) => {
    setActiveTarget({ kind: 'run', data: run });
    setDraft(run.text);
  }, []);

  const cancelEdit = useCallback(() => {
    setActiveTarget(null);
    setDraft('');
  }, []);

  const applyEdit = useCallback(() => {
    if (!opts.filePath || !activeTarget || !draft.trim()) {
      cancelEdit();
      return;
    }
    const text = draft.trim();
    const target = activeTarget;
    cancelEdit();

    if (target.kind === 'line') {
      void opts.runEdit({
        command: 'replace_text_line',
        args: {
          path: opts.filePath,
          pageIndex: opts.currentPage,
          lineIndex: target.index,
          newText: text,
        },
        reloadAt: opts.currentPage,
        toast: 'Text replaced',
      });
    } else {
      void opts.runEdit({
        command: 'replace_text_region',
        args: {
          path: opts.filePath,
          pageIndex: opts.currentPage,
          x: target.data.x,
          y: target.data.y,
          w: target.data.w,
          h: target.data.h,
          newText: text,
          fontSize: target.data.h * 0.85,
        },
        reloadAt: opts.currentPage,
        toast: 'Text replaced',
      });
    }
  }, [activeTarget, cancelEdit, draft, opts]);

  return {
    activeRun: opts.editTextRunMode
      ? activeTarget?.kind === 'run'
        ? activeTarget.data
        : null
      : null,
    activeLine: opts.editTextRunMode
      ? activeTarget?.kind === 'line'
        ? activeTarget.data
        : null
      : null,
    draft,
    setDraft,
    lines,
    hitTestLine,
    hitTestRun,
    openLineEditor,
    openRunEditor,
    cancelEdit,
    applyEdit,
  };
}
