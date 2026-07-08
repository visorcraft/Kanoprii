#!/usr/bin/env sh
# Build Linux deb and rpm packages for Kanoprii.
#
# Usage: scripts/build-linux-packages.sh
set -eu

if [ "$(uname -s)" != Linux ]; then
  echo "deb/rpm builds are Linux-only (current OS: $(uname -s))." >&2
  exit 1
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

if [ ! -f src-tauri/vendor/pdfium/libpdfium.so ]; then
  echo "PDFium not found; fetching prebuilt library..."
  "$root/scripts/fetch-pdfium.sh"
fi

echo "Building deb and rpm packages..."
exec npx tauri build --bundles deb,rpm
