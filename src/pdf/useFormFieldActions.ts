import { invoke } from '@tauri-apps/api/core';
import { useCallback } from 'react';
import type { FormFieldData } from '../app/types';

type UseFormFieldActionsOptions = {
  filePath: string;
  formFields: FormFieldData[];
  formDrafts: Record<string, string>;
  withLoading: <T>(fn: () => Promise<T>) => Promise<T | undefined>;
  markPdfEdited: () => void;
  loadFormFields: (path: string) => Promise<void>;
  showToast: (msg: string, kind?: 'error') => void;
};

export function useFormFieldActions(opts: UseFormFieldActionsOptions) {
  const applyFormField = useCallback(
    (name: string) => {
      const field = opts.formFields.find((entry) => entry.name === name);
      if (!field || !opts.filePath) return;
      const draft = opts.formDrafts[name] ?? '';
      void opts.withLoading(async () => {
        await invoke('set_pdf_form_field', { path: opts.filePath, name, value: draft });
        opts.markPdfEdited();
        await opts.loadFormFields(opts.filePath);
        opts.showToast(`Updated ${name}`);
      });
    },
    [opts],
  );

  return { applyFormField };
}
