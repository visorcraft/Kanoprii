#!/usr/bin/env sh
# Authenticode-sign Windows release artifacts produced by `npx tauri build`.
# Usage: scripts/sign-windows.sh [path/to/certificate.pfx]
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
pfx="${1:-${WINDOWS_CERTIFICATE_PATH:-}}"
password="${WINDOWS_CERTIFICATE_PASSWORD:?Set WINDOWS_CERTIFICATE_PASSWORD}"
timestamp_url="${WINDOWS_TIMESTAMP_URL:-https://timestamp.digicert.com}"

if [ -z "$pfx" ]; then
  echo "Usage: scripts/sign-windows.sh path/to/certificate.pfx" >&2
  exit 1
fi

sign_one() {
  target="$1"
  echo "Signing $target"
  signtool sign /fd SHA256 /tr "$timestamp_url" /td SHA256 /f "$pfx" /p "$password" "$target"
}

signed=0
release_dir="$root/src-tauri/target/release"
bundle_dir="$release_dir/bundle"

for candidate in \
  "$release_dir/kanoprii.exe" \
  "$bundle_dir/msi/"*.msi \
  "$bundle_dir/nsis/"*.exe; do
  if [ -f "$candidate" ]; then
    sign_one "$candidate"
    signed=1
  fi
done

if [ "$signed" -eq 0 ]; then
  echo "No Windows release artifacts found to sign." >&2
  exit 1
fi

echo "Windows signing complete."
