#!/usr/bin/env sh
# Quick local quality gate (matches CI minus Tauri bundle). Usage: scripts/smoke-test.sh
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

npm ci
npm run audit
npm run lint
npx tsc --noEmit
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
RUSTFLAGS=-Dwarnings cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets

echo "smoke-test: OK"
