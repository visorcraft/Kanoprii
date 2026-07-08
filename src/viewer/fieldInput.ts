import type React from 'react';

/** Commit-on-Enter helper for numeric fields (Tab / blur commit via onBlur). */
export function onFieldKeyDown(
  e: React.KeyboardEvent<HTMLInputElement>,
  commit: () => void,
) {
  if (e.key === 'Enter') {
    commit();
    e.currentTarget.blur();
  }
}
