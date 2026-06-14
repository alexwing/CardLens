import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { LanguageProvider } from './lib/i18n';
import './styles.css';

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('No se encontro el elemento raiz #root');
}

createRoot(rootElement).render(
  <StrictMode>
    <LanguageProvider>
      <App />
    </LanguageProvider>
  </StrictMode>
);

// La app va empaquetada (Tauri): NO usamos service worker. Un SW de cache
// provocaba que, al actualizar, se viera la version antigua de la pagina. Aqui
// desregistramos cualquier SW previo y limpiamos sus caches; ademas el propio
// /sw.js es autodestructivo para limpiar instalaciones anteriores.
if ('serviceWorker' in navigator) {
  navigator.serviceWorker
    .getRegistrations()
    .then((registrations) => registrations.forEach((registration) => registration.unregister()))
    .catch(() => {});
}
if (typeof caches !== 'undefined' && typeof caches.keys === 'function') {
  caches
    .keys()
    .then((keys) => keys.forEach((key) => caches.delete(key)))
    .catch(() => {});
}
