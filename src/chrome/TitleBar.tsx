import React from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import kanopriiIcon from '../assets/kanoprii_face.png';
import { isTauriRuntime } from '../app/tauriRuntime';

type TitleBarProps = {
  title: string;
};

function TitleBarIcon({ children }: { children: React.ReactNode }) {
  return (
    <svg className="titlebar-btn-icon" viewBox="0 0 10 10" aria-hidden="true">
      {children}
    </svg>
  );
}

export function TitleBar({ title }: TitleBarProps) {
  const win = isTauriRuntime() ? getCurrentWindow() : null;

  return (
    <header className="window-titlebar">
      <div className="titlebar-leading">
        <img src={kanopriiIcon} alt="" className="titlebar-icon" aria-hidden="true" />
      </div>
      <div className="titlebar-drag" data-tauri-drag-region>
        <span className="titlebar-title">{title}</span>
      </div>
      <div className="titlebar-controls">
        <button
          type="button"
          className="titlebar-btn"
          aria-label="Minimize"
          disabled={!win}
          onClick={() => void win?.minimize()}
        >
          <TitleBarIcon>
            <path d="M1.5 6 L5 3 L8.5 6" />
          </TitleBarIcon>
        </button>
        <button
          type="button"
          className="titlebar-btn"
          aria-label="Maximize"
          disabled={!win}
          onClick={() => void win?.toggleMaximize()}
        >
          <TitleBarIcon>
            <path d="M1.5 4 L5 7 L8.5 4" />
          </TitleBarIcon>
        </button>
        <button
          type="button"
          className="titlebar-btn titlebar-btn-close"
          aria-label="Close"
          disabled={!win}
          onClick={() => void win?.close()}
        >
          <TitleBarIcon>
            <path d="M2 2 L8 8 M8 2 L2 8" />
          </TitleBarIcon>
        </button>
      </div>
    </header>
  );
}
