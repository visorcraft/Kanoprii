#!/usr/bin/env sh
# Build a Linux AppImage for Kanoprii.
#
# Uses Tauri's linuxdeploy-based AppImage bundler (tools prefetched to
# ~/.cache/tauri via scripts/fetch-appimage-tools.sh). On glibc 2.38+ distros,
# linuxdeploy's strip step can fail - NO_STRIP=1 is the default (override with
# NO_STRIP=0 on older build hosts if needed).
#
# Usage: scripts/build-appimage.sh
set -eu

if [ "$(uname -s)" != Linux ]; then
  echo "AppImage builds are Linux-only (current OS: $(uname -s))." >&2
  exit 1
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

"$root/scripts/fetch-appimage-tools.sh"

if [ ! -f src-tauri/vendor/pdfium/libpdfium.so ]; then
  echo "PDFium not found; fetching prebuilt library..."
  "$root/scripts/fetch-pdfium.sh"
fi

export NO_STRIP="${NO_STRIP:-1}"

echo "Building AppImage (NO_STRIP=${NO_STRIP})..."
exec npx tauri build --bundles appimage
