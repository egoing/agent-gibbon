'use strict';
// Register before the app module; never send error payloads, DOM or note contents.
for (const kind of ['error', 'unhandledrejection']) {
  window.addEventListener(kind, () => {
    window.__TAURI__?.core.invoke('report_frontend_error', { kind }).catch(() => {});
  });
}
