#!/usr/bin/env sh
# Build an optimized release binary without OS installers (matches CI).
#
# Output: src-tauri/target/release/kanoprii
#
# Usage: scripts/build-no-bundle.sh
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

case "$(uname -s)" in
  Linux)
    if [ ! -f src-tauri/vendor/pdfium/libpdfium.so ]; then
      echo "PDFium not found; fetching prebuilt library..."
      "$root/scripts/fetch-pdfium.sh"
    fi
    ;;
  Darwin)
    if [ ! -f src-tauri/vendor/pdfium/libpdfium.dylib ]; then
      echo "PDFium not found; fetching prebuilt library..."
      "$root/scripts/fetch-pdfium.sh"
    fi
    ;;
  MINGW*|MSYS*|CYGWIN*)
    if [ ! -f src-tauri/vendor/pdfium/pdfium.dll ]; then
      echo "PDFium not found; fetching prebuilt library..."
      "$root/scripts/fetch-pdfium.sh"
    fi
    ;;
esac

echo "Building release binary (no bundle)..."
exec npx tauri build --no-bundle
