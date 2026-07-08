#!/usr/bin/env sh
# Fetch the prebuilt PDFium runtime library required for PDF rendering.
#
# PDF rendering uses `pdfium-render`, which needs a standard PDFium build (the C
# `FPDF_*` API). This downloads the official prebuilt from bblanchon/pdfium-
# binaries into `src-tauri/vendor/pdfium/`, where the app's loader looks for it.
# The library is not committed to the repo (it is large and platform-specific).
#
# Usage: scripts/fetch-pdfium.sh [release-tag]   (defaults to the latest release)
set -eu

version="${1:-latest}"
root="$(cd "$(dirname "$0")/.." && pwd)"
dest="$root/src-tauri/vendor/pdfium"

os="$(uname -s)"
arch="$(uname -m)"
layout="unix"

case "$os" in
  Linux)
    case "$arch" in
      x86_64)  asset="pdfium-linux-x64.tgz" ;;
      aarch64) asset="pdfium-linux-arm64.tgz" ;;
      *) echo "Unsupported Linux arch: $arch" >&2; exit 1 ;;
    esac
    ;;
  Darwin)
    case "$arch" in
      x86_64)  asset="pdfium-mac-x64.tgz" ;;
      arm64)   asset="pdfium-mac-arm64.tgz" ;;
      *) echo "Unsupported macOS arch: $arch" >&2; exit 1 ;;
    esac
    ;;
  MINGW*|MSYS*|CYGWIN*)
    case "$arch" in
      x86_64) asset="pdfium-win-x64.tgz"; layout="win" ;;
      *) echo "Unsupported Windows arch: $arch" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported platform: $os-$arch. See https://github.com/bblanchon/pdfium-binaries/releases" >&2
    exit 1
    ;;
esac

if [ "$version" = latest ]; then
  url="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/$asset"
else
  url="https://github.com/bblanchon/pdfium-binaries/releases/download/$version/$asset"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
echo "Downloading $url"
curl -fSL "$url" -o "$tmp/pdfium.tgz"
mkdir -p "$dest"
if [ "$layout" = win ]; then
  tar xzf "$tmp/pdfium.tgz" -C "$tmp"
  if [ -f "$tmp/pdfium.dll" ]; then
    cp "$tmp/pdfium.dll" "$dest"/
  elif [ -f "$tmp/bin/pdfium.dll" ]; then
    cp "$tmp/bin/pdfium.dll" "$dest"/
  else
    echo "Unexpected pdfium-win archive layout" >&2
    exit 1
  fi
else
  tar xzf "$tmp/pdfium.tgz" -C "$tmp" lib
  cp "$tmp"/lib/libpdfium.* "$dest"/
fi
echo "PDFium installed to $dest"
