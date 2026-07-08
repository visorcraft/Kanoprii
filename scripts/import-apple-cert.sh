#!/usr/bin/env sh
# Import an Apple code-signing certificate into a temporary CI keychain.
# Requires APPLE_CERTIFICATE_BASE64 and APPLE_CERTIFICATE_PASSWORD.
# Usage: scripts/import-apple-cert.sh
set -eu

: "${APPLE_CERTIFICATE_BASE64:?Set APPLE_CERTIFICATE_BASE64}"
: "${APPLE_CERTIFICATE_PASSWORD:?Set APPLE_CERTIFICATE_PASSWORD}"
: "${APPLE_KEYCHAIN_PASSWORD:?Set APPLE_KEYCHAIN_PASSWORD}"

KEYCHAIN="${APPLE_KEYCHAIN_PATH:-$RUNNER_TEMP/build.keychain-db}"
KEYCHAIN_PASSWORD="$APPLE_KEYCHAIN_PASSWORD"

security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
security default-keychain -s "$KEYCHAIN"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
security set-keychain-settings -lut 21600 "$KEYCHAIN"

cert_file="$(mktemp).p12"
trap 'rm -f "$cert_file"' EXIT INT TERM
case "$(uname -s)" in
  Darwin) printf '%s' "$APPLE_CERTIFICATE_BASE64" | base64 -D > "$cert_file" ;;
  *) printf '%s' "$APPLE_CERTIFICATE_BASE64" | base64 --decode > "$cert_file" ;;
esac
security import "$cert_file" -k "$KEYCHAIN" -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign -T /usr/bin/security
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN"

echo "Apple signing certificate imported into $KEYCHAIN"
