import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@brand/tokens.css';
import './app/global.css';
import './shared/ui/ui.css';
import './features/library/library.css';
import './features/session/session.css';
import './features/quickmenu/quickmenu.css';
import './features/volume/volume.css';
import { App } from './app/App';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
