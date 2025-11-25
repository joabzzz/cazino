#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "${ROOT_DIR}"

if [[ -z "${CAZINO_PORT:-}" ]]; then
  CAZINO_PORT=$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(('127.0.0.1', 0))
port = s.getsockname()[1]
s.close()
print(port)
PY
  )
  export CAZINO_PORT
fi

export E2E_BASE_URL="http://127.0.0.1:${CAZINO_PORT}"

scripts/run_e2e_server.sh &
SERVER_PID=$!
cleanup() {
  kill "$SERVER_PID" 2>/dev/null || true
  wait "$SERVER_PID" 2>/dev/null || true
}
trap cleanup EXIT

for attempt in {1..40}; do
  if curl -sf "${E2E_BASE_URL}/health" >/dev/null; then
    break
  fi
  sleep 0.5
  if [[ $attempt -eq 40 ]]; then
    echo "Server failed to become ready" >&2
    exit 1
  fi
done

npx playwright test -c tests/playwright.config.ts
