#!/usr/bin/env bash
#
# Build the web app and run longxia-server bound to all interfaces, ready to be
# fronted by a tunnel for HTTPS. Prints the shared access token, then the
# commands to open a public URL. Nothing here is destructive.
#
# Usage:
#   scripts/expose.sh                 # generates a token, uses the app data-dir DB
#   PORT=9000 scripts/expose.sh       # different port
#   LONGXIA_TOKEN=... scripts/expose.sh   # reuse an existing token
#   LONGXIA_DB=/path/to.db scripts/expose.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${PORT:-8787}"

# Shared access token: reuse if provided, else generate a strong one.
if [ -z "${LONGXIA_TOKEN:-}" ]; then
  LONGXIA_TOKEN="$(openssl rand -hex 32)"
  echo "Generated a new access token:"
  echo "  LONGXIA_TOKEN=$LONGXIA_TOKEN"
  echo "Share it with whoever should have access; they enter it in the app once."
  echo
fi
export LONGXIA_TOKEN

# Default to the app's data-dir database (which holds the imported CC-CEDICT).
export LONGXIA_DB="${LONGXIA_DB:-$HOME/Library/Application Support/com.longxia.study/longxia.db}"
export LONGXIA_ADDR="0.0.0.0:${PORT}"
export LONGXIA_WEB_DIR="${ROOT}/app/dist"

echo "Building the web app..."
( cd "${ROOT}/app" && npm run build )

echo
echo "Starting longxia-server on ${LONGXIA_ADDR}"
echo "  web:  ${LONGXIA_WEB_DIR}"
echo "  db:   ${LONGXIA_DB}"
echo
echo "In another terminal, expose it over HTTPS with one of:"
echo "  cloudflared tunnel --url http://localhost:${PORT}"
echo "  ngrok http ${PORT}"
echo "  tailscale funnel ${PORT}"
echo "Open the tunnel's URL, then enter the access token when the app asks."
echo

exec cargo run --release --manifest-path "${ROOT}/app/src-tauri/Cargo.toml" -p longxia-server
