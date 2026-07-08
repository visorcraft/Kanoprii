#!/usr/bin/env sh
# Sign (and optionally notarize) macOS release bundles produced by `npx tauri build`.
# Usage: scripts/sign-macos.sh [path/to/App.app]
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
bundle_dir="$root/src-tauri/target/release/bundle/macos"
identity="${APPLE_SIGNING_IDENTITY:?Set APPLE_SIGNING_IDENTITY}"

sign_app() {
  app="$1"
  echo "Signing $app"
  codesign --force --options runtime --timestamp --sign "$identity" --deep "$app"
  codesign --verify --deep --strict "$app"
}

notarize_app() {
  app="$1"
  : "${APPLE_ID:?Set APPLE_ID for notarization}"
  : "${APPLE_APP_PASSWORD:?Set APPLE_APP_PASSWORD for notarization}"
  : "${APPLE_TEAM_ID:?Set APPLE_TEAM_ID for notarization}"

  zip_path="$(mktemp).zip"
  trap 'rm -f "$zip_path"' RETURN
  ditto -c -k --keepParent "$app" "$zip_path"
  xcrun notarytool submit "$zip_path" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_APP_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait
  xcrun stapler staple "$app"
  xcrun stapler validate "$app"
}

if [ "$#" -gt 0 ]; then
  apps="$*"
else
  apps="$bundle_dir/"*.app
fi

signed=0
for app in $apps; do
  [ -d "$app" ] || continue
  sign_app "$app"
  if [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_APP_PASSWORD:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
    notarize_app "$app"
  fi
  signed=1
done

if [ "$signed" -eq 0 ]; then
  echo "No .app bundles found to sign under $bundle_dir" >&2
  exit 1
fi

for dmg in "$bundle_dir/"*.dmg; do
  [ -f "$dmg" ] || continue
  echo "Signing $dmg"
  codesign --force --timestamp --sign "$identity" "$dmg"
done

echo "macOS signing complete."
