#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "${ROOT_DIR}"

if [[ -z "${CAZINO_DATABASE_URL:-}" ]]; then
  TEMP_DB_DIR=$(mktemp -d 2>/dev/null || mktemp -d -t 'cazino-e2e')
  DB_FILE="${TEMP_DB_DIR}/cazino-e2e.db"
  cleanup_tmp_db() {
    rm -rf -- "${TEMP_DB_DIR}"
  }
  trap cleanup_tmp_db EXIT
  export CAZINO_DATABASE_URL="sqlite://${DB_FILE}?mode=rwc"
fi

export CAZINO_PORT=${CAZINO_PORT:-3333}
export RUST_LOG=${RUST_LOG:-warn}

cargo run -- serve
