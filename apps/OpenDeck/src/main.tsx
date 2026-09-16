import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import './styles.css';
import { bridge } from './bridge';

async function bootstrap() {
  const params = new URLSearchParams(window.location.search);
  if (params.get('qualification') !== 'v210') {
    try {
      const context = await bridge.qualificationContext();
      if (context.enabled) {
        params.set('qualification', 'v210');
        params.set('phase', context.phase);
        window.history.replaceState({}, '', `${window.location.pathname}?${params.toString()}`);
      }
    } catch {
      // Normal browser/dev mode may not expose the Tauri bridge.
    }
  }
  createRoot(document.getElementById('root')!).render(<StrictMode><App /></StrictMode>);
}

void bootstrap();
