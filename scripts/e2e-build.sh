#!/usr/bin/env sh
# Build the debug Tauri binary used by the WebdriverIO smoke suite.
# Usage: scripts/e2e-build.sh
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/src-tauri/target}"
export VITE_WDIO=1
export TAURI_WEBDRIVER_PORT=4445

if [ ! -d node_modules ]; then
  npm ci
fi
npm ci --prefix e2e

"$root/scripts/fetch-pdfium.sh"
cleanup() {
  rm -f "$root/src-tauri/capabilities/e2e.json"
  # The e2e capability leaks into the generated permission schemas; restore them.
  git -C "$root" checkout -- src-tauri/gen/schemas 2>/dev/null || true
}
trap cleanup EXIT INT TERM
cp "$root/e2e/capabilities/e2e.json" "$root/src-tauri/capabilities/e2e.json"
npm run build
npx tauri build --debug --no-bundle --config src-tauri/tauri.e2e.conf.json -- --features wdio
