import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { AnnouncerProvider } from './ui/Announcer';
import { ErrorBoundary } from './ui/ErrorBoundary';
import './styles.css';

if (import.meta.env.VITE_WDIO === '1') {
  void import('@wdio/tauri-plugin');
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <AnnouncerProvider>
      <ErrorBoundary onError={(error, info) => console.error('Top-level error:', error, info)}>
        <App />
      </ErrorBoundary>
    </AnnouncerProvider>
  </StrictMode>
);
