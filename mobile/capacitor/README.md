# Knowledgeable — Capacitor Android shell (thin packaging)

The web client (`apps/web`) is the single canonical product UI.
This shell packages it for Android; it contains no product UI of its own.

## Dev (live backend on your LAN)

The WebView loads straight from the laptop backend, keeping single-origin
semantics (relative `/api`, no CORS changes):

```sh
# Terminal 1: backend + Vite (or backend-only; the shell talks to :3000)
./scripts/dev --backend-only

# Terminal 2: point the shell at the laptop's LAN IP, then sync + run
export CAPACITOR_SERVER_URL=http://<laptop-lan-ip>:3000
cd mobile/capacitor && npx cap sync android
# open mobile/capacitor/android in Android Studio → Run
```

`CAPACITOR_SERVER_URL` is the only knob. No IP is checked in — the old
hardcoded `192.168.x` value rotted on every DHCP change. Without the env
var, the config emits no `server` section (bundled-`www` mode).

## Prod (bundled UI)

```sh
cd mobile/capacitor && npm run sync:www
```

Builds the production web bundle (`apps/web` → `client/dist`), copies it to
`www/` (gitignored build artifact, minus precompressed `*.gz`/`*.br` — the
WebView serves no negotiated encoding and AGP rejects them as duplicates),
and syncs the Android project.

**Known limit (tracked, not wired):** bundled-`www` against a *remote*
backend needs two things this repo does not have yet — an absolute API base
URL in the web client (today `/api` is relative: Vite proxy in dev,
same-origin Axum serving in prod) and a CORS allowlist entry for the
`https://localhost` WebView origin (`QUALITY.md` requires explicit origins).
Until the backend has a public HTTPS deployment, dev-liveload above is the
supported path; real-device validation gates on that deployment first
(see `knowledgeable_mobile_agent_prompt.md` Phase 1 → Phase 4).
