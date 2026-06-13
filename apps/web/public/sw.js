/*
 * Service worker de la PWA.
 * Estrategia:
 *  - Shell estatico (mismo origen): cache-first con relleno de cache en caliente.
 *  - /api, /images y /scans (o cualquier otro origen): siempre red, sin cache.
 *  - Navegaciones sin red: fallback a /index.html cacheado.
 */
const CACHE_NAME = 'pcd-shell-v1';
const SHELL_URLS = ['/', '/index.html', '/manifest.webmanifest', '/icon.svg'];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(CACHE_NAME)
      .then((cache) => cache.addAll(SHELL_URLS))
      .then(() => self.skipWaiting())
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key)))
      )
      .then(() => self.clients.claim())
  );
});

self.addEventListener('fetch', (event) => {
  const request = event.request;
  if (request.method !== 'GET') {
    return;
  }

  const url = new URL(request.url);
  const isSameOrigin = url.origin === self.location.origin;
  const isDynamic =
    url.pathname.startsWith('/api') ||
    url.pathname.startsWith('/images') ||
    url.pathname.startsWith('/scans');

  // API e imagenes dinamicas (y cualquier otro origen): solo red, nunca cache.
  if (!isSameOrigin || isDynamic) {
    return;
  }

  // Navegaciones: red primero con fallback al shell cacheado.
  if (request.mode === 'navigate') {
    event.respondWith(fetch(request).catch(() => caches.match('/index.html')));
    return;
  }

  // Estaticos del shell: cache-first.
  event.respondWith(
    caches.match(request).then((cached) => {
      if (cached) {
        return cached;
      }
      return fetch(request).then((response) => {
        if (response.ok) {
          const copy = response.clone();
          caches.open(CACHE_NAME).then((cache) => cache.put(request, copy));
        }
        return response;
      });
    })
  );
});
