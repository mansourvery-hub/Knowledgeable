import type { CapacitorConfig } from '@capacitor/cli';

// Dev shell: point the WebView at a LAN backend so the app keeps the
// backend's single-origin guarantees (relative /api, no CORS changes):
//   CAPACITOR_SERVER_URL=http://<laptop-lan-ip>:3000 npx cap sync android
//
// Prod default: no `server` section, so the app serves the bundled `www/`
// (populated from the production web build, see package.json `sync:www`).
// NOTE: bundled-www against a *remote* backend still needs an absolute API
// base in the web client + CORS allowlisting — not wired yet, tracked in
// README.md. Do not ship a hardcoded LAN IP here; it rots on every DHCP
// change.
const serverUrl = process.env.CAPACITOR_SERVER_URL?.trim();

const config: CapacitorConfig = {
  appId: 'com.knowledgeable.app',
  appName: 'Knowledgeable',
  webDir: 'www',
  ...(serverUrl
    ? {
        server: {
          url: serverUrl,
          cleartext: serverUrl.startsWith('http://'),
        },
      }
    : {}),
};

export default config;
