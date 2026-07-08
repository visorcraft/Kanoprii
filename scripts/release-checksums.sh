#!/usr/bin/env sh
# Write SHA256SUMS.txt for every file under a release artifact directory.
# Usage: scripts/release-checksums.sh [directory] [output-file]
set -eu

dir="${1:-.}"
out="${2:-$dir/SHA256SUMS.txt}"

if command -v sha256sum >/dev/null 2>&1; then
  hash_file() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
  hash_file() { shasum -a 256 "$1" | awk '{print $1}'; }
else
  echo "sha256sum or shasum is required" >&2
  exit 1
fi

(
  cd "$dir"
  : > "$(basename "$out")"
  find . -type f ! -name 'SHA256SUMS.txt' | LC_ALL=C sort | while read -r path; do
    rel="${path#./}"
    printf '%s  %s\n' "$(hash_file "$rel")" "$rel"
  done >> "$(basename "$out")"
)

echo "Wrote $out"
