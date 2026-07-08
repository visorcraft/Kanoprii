#!/usr/bin/env sh
# Build unsigned macOS bundles for Kanoprii (.app / .dmg).
#
# Usage: scripts/build-macos.sh
set -eu

if [ "$(uname -s)" != Darwin ]; then
  echo "macOS builds require Darwin (current OS: $(uname -s))." >&2
  exit 1
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

if [ ! -f src-tauri/vendor/pdfium/libpdfium.dylib ]; then
  echo "PDFium not found; fetching prebuilt library..."
  "$root/scripts/fetch-pdfium.sh"
fi

echo "Building macOS bundles (unsigned)..."
exec npx tauri build
