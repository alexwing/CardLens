/*
 * Service worker AUTODESTRUCTIVO.
 *
 * La app se distribuye empaquetada (Tauri escritorio/Android): no necesita
 * cache offline. Versiones anteriores registraban un SW que cacheaba el shell
 * (cache-first) y, al actualizar, mostraba la version ANTIGUA de la pagina.
 *
 * Este SW reemplaza a aquel: no intercepta peticiones (sin handler de fetch),
 * borra TODAS las caches, se desregistra y recarga las ventanas para que se
 * sirva la version nueva. Asi los usuarios que vienen de una version con SW
 * quedan limpios en cuanto el navegador comprueba /sw.js.
 */
self.addEventListener('install', () => self.skipWaiting());

self.addEventListener('activate', (event) => {
  event.waitUntil(
    (async () => {
      try {
        const keys = await caches.keys();
        await Promise.all(keys.map((key) => caches.delete(key)));
        await self.registration.unregister();
        const clients = await self.clients.matchAll({ type: 'window' });
        for (const client of clients) {
          client.navigate(client.url);
        }
      } catch (error) {
        // best-effort: si algo falla, al menos no cacheamos nada (sin fetch handler).
      }
    })()
  );
});
