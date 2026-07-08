#!/usr/bin/env sh
# Regenerate docs/credits-third-party.md (Rust + shipped npm) from lockfiles.
# Usage: scripts/generate-credits.sh
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

mkdir -p docs
cargo about generate about.hbs \
  --manifest-path src-tauri/Cargo.toml \
  --output-file docs/credits-third-party-rust.md
sed -i 's/\r$//' docs/credits-third-party-rust.md

node scripts/generate-npm-credits.mjs

{
  cat docs/credits-third-party-rust.md
  echo
  echo '---'
  echo
  tail -n +3 docs/credits-npm.md
} > docs/credits-third-party.md

echo "Wrote docs/credits-third-party.md"
