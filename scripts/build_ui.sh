#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
UI_DIR="${ROOT_DIR}/ui"
OUTPUT_DIR_NAME="${DIST_DIR:-dist}"
DIST_DIR="${ROOT_DIR}/${OUTPUT_DIR_NAME}"

rm -rf -- "${DIST_DIR}"
mkdir -p -- "${DIST_DIR}"
cp -R "${UI_DIR}/." "${DIST_DIR}/"

cat <<CONFIG > "${DIST_DIR}/config.js"
window.CAZINO_CONFIG = {
  apiBase: "${PUBLIC_API_BASE_URL:-}",
  wsUrl: "${PUBLIC_WS_URL:-}",
};
CONFIG

echo "Static UI written to ${DIST_DIR}"
