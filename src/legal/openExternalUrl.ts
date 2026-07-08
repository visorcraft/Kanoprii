import { invoke } from '@tauri-apps/api/core';

function isHttpUrl(url: string): boolean {
  try {
    const { protocol } = new URL(url);
    return protocol === 'http:' || protocol === 'https:';
  } catch {
    return false;
  }
}

export function openExternalUrl(url: string): void {
  if (!url || !isHttpUrl(url)) return;
  void invoke('open_external_url', { url }).catch(() => {
    window.open(url, '_blank', 'noopener,noreferrer');
  });
}
