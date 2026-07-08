#!/usr/bin/env sh
# Build unsigned Windows installers for Kanoprii (msi/nsis per Tauri defaults).
#
# Usage: scripts/build-windows.sh
set -eu

os="$(uname -s)"
case "$os" in
  MINGW*|MSYS*|CYGWIN*) ;;
  *)
    echo "Windows builds must run on Windows (current OS: $os)." >&2
    exit 1
    ;;
esac

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

if [ ! -f src-tauri/vendor/pdfium/pdfium.dll ]; then
  echo "PDFium not found; fetching prebuilt library..."
  "$root/scripts/fetch-pdfium.sh"
fi

echo "Building Windows bundles (unsigned)..."
exec npx tauri build
