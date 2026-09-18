#!/bin/bash
# Knowledgeable Project Starter
#
# DEFAULT: modern web client (vendored LibreChat + Knowledgeable extensions:
# concept badges, knowledge graph explorer, personal wiki, tutor activity).
#   ./start.sh              -> backend :3000 + Vite web client :3090
#                            open http://localhost:3090
#   ./start.sh --backend-only -> backend :3000 only
#
# The Flutter client in apps/client is deprecated (see
# docs/agent-context/roadmap_and_state.md); its modes remain below.
set -u

MODE="${1:-web}"

case "$MODE" in
  web|"")
    exec ./scripts/dev
    ;;
  --backend-only)
    exec ./scripts/dev --backend-only
    ;;
  native|--flutter-native)
    echo "WARNING: the Flutter client is deprecated; default is the web client." >&2
    MODE="native"
    ;;
  --web|--flutter-web|--web-debug|--rebuild)
    echo "WARNING: the Flutter client is deprecated; default is the web client." >&2
    ;;
  *)
    echo "Usage: ./start.sh [web|--backend-only|native|--web|--web-debug]" >&2
    exit 2
    ;;
esac

# --- Legacy Flutter modes below (deprecated, unchanged behavior) ---
REBUILD_WEB=0
if [ "${2:-}" = "--rebuild" ] || [ "$MODE" = "--rebuild" ]; then
  REBUILD_WEB=1
  if [ "$MODE" = "--rebuild" ]; then MODE="--web"; fi
fi

if command -v fvm >/dev/null 2>&1; then
  FLUTTER="fvm flutter"
else
  FLUTTER="flutter"
fi
# Flutter commands must run from the project dir (where pubspec.yaml lives).
CLIENT_DIR="apps/client"

echo "--- Knowledgeable Starter ($MODE) ---"

cleanup_backend() {
  if [ -n "${BACKEND_PID:-}" ] && kill -0 "$BACKEND_PID" 2>/dev/null; then
    echo "Stopping backend ($BACKEND_PID)..."
    kill "$BACKEND_PID" 2>/dev/null
  fi
}

# 1. Free backend port (and 8080 for web modes)
echo "Ensuring port 3000 is free..."
if command -v fuser >/dev/null 2>&1; then
  fuser -k 3000/tcp > /dev/null 2>&1 || true
  if [ "$MODE" = "--web" ] || [ "$MODE" = "--web-debug" ]; then
    echo "Ensuring port 8080 is free..."
    fuser -k 8080/tcp > /dev/null 2>&1 || true
  fi
  sleep 1
else
  echo "(fuser not found, skipping port kill)"
fi
echo "Ports ready."

# 2. Backend (background, health-gated)
echo "Starting backend server..."
cargo run --bin server > backend.log 2>&1 &
BACKEND_PID=$!
echo "Backend started (PID: $BACKEND_PID, logging to backend.log)"

echo "Waiting for backend health check (http://localhost:3000/health)..."
HEALTHY=0
for i in $(seq 1 60); do
  if curl -sf http://localhost:3000/health > /dev/null 2>&1; then
    echo "Backend is healthy."
    HEALTHY=1
    break
  fi
  if ! kill -0 $BACKEND_PID 2>/dev/null; then
    echo "ERROR: backend process died. Check backend.log:"
    tail -n 30 backend.log
    exit 1
  fi
  sleep 1
  if [ "$i" -eq 60 ]; then
    echo "ERROR: backend did not become healthy in 60s. Check backend.log:"
    tail -n 30 backend.log
    cleanup_backend
    exit 1
  fi
done

# 3. Frontend by mode
case "$MODE" in
  native|"")
    echo "Starting native Linux app (foreground — press 'r' for hot-reload, Ctrl+C to stop)..."
    trap "echo 'Stopping services...'; cleanup_backend; exit" SIGINT SIGTERM
    # Foreground so hot-reload keys work; backend is killed by the trap.
    (cd "$CLIENT_DIR" && $FLUTTER run -d linux --dart-define=API_BASE_URL=http://localhost:3000) 2>&1 | tee apps/client/flutter.log
    cleanup_backend
    ;;
  --web)
    if [ "$REBUILD_WEB" = "1" ] || [ ! -f "apps/client/build/web/main.dart.js" ]; then
      echo "Building web release (one-time, a minute or two)..."
      (cd "$CLIENT_DIR" && $FLUTTER build web --release --dart-define=API_BASE_URL=http://localhost:3000) 2>&1 | tee apps/client/flutter.log
    else
      echo "Release build found — serving instantly (no recompile). Use '--rebuild' to force a fresh build."
    fi
    echo "Serving Flutter release on http://localhost:8080 ..."
    python3 -m http.server 8080 --directory apps/client/build/web > apps/client/flutter.log 2>&1 &
    FRONTEND_PID=$!
    echo "Frontend serving (PID: $FRONTEND_PID). Open http://localhost:8080 in a browser."
    echo "--- Everything is running. Press Ctrl+C to stop all services ---"
    trap "echo 'Stopping services...'; kill $BACKEND_PID $FRONTEND_PID 2>/dev/null; exit" SIGINT SIGTERM
    wait
    ;;
  --web-debug)
    echo "Starting Flutter web DEBUG on http://localhost:8080 ..."
    echo "First compile takes 30-60s and shows a BLANK page until done — this is expected."
    (cd "$CLIENT_DIR" && $FLUTTER run -d web-server --web-hostname 0.0.0.0 --web-port 8080 --dart-define=API_BASE_URL=http://localhost:3000) 2>&1 | tee apps/client/flutter.log
    cleanup_backend
    ;;
  --backend-only)
    echo "Backend only. Health: http://localhost:3000/health"
    echo "--- Press Ctrl+C to stop ---"
    trap "echo 'Stopping services...'; cleanup_backend; exit" SIGINT SIGTERM
    wait $BACKEND_PID
    ;;
  *)
    echo "Unknown mode: $MODE"
    echo "Usage: ./start.sh [native|--web [--rebuild]|--web-debug|--backend-only]"
    cleanup_backend
    exit 1
    ;;
esac
