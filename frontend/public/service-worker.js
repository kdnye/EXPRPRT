self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open('fsi-expense-cache').then((cache) => cache.addAll(['/'])).catch(() => undefined)
  );
});

self.addEventListener('fetch', (event) => {
  event.respondWith(
    caches.match(event.request).then((response) => response || fetch(event.request))
  );
});
