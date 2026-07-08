#!/usr/bin/env sh
# Validate that a Tauri updater minisign private key is formatted correctly.
# Usage: scripts/validate-signing-key.sh [path/to/kanoprii.key]
set -eu

key_file="${1:-$HOME/.tauri/kanoprii.key}"

if [ ! -f "$key_file" ]; then
  echo "ERROR: key file not found: $key_file" >&2
  exit 1
fi

python3 - "$key_file" <<'PY'
import base64, sys

key_file = sys.argv[1]
with open(key_file) as f:
    lines = [line.rstrip('\n') for line in f if line.strip()]

if len(lines) != 2:
    print(f"ERROR: expected 2 non-empty lines, found {len(lines)}", file=sys.stderr)
    sys.exit(1)

comment, key = lines

if not comment.startswith('untrusted comment:'):
    print('ERROR: first line must start with "untrusted comment:"', file=sys.stderr)
    sys.exit(1)

if ' ' in key:
    print(f'ERROR: base64 key contains a space at position {key.index(" ")}', file=sys.stderr)
    sys.exit(1)

try:
    base64.b64decode(key, validate=True)
except Exception as e:
    print(f'ERROR: base64 key is invalid: {e}', file=sys.stderr)
    sys.exit(1)

print('OK: key format looks valid')
PY
