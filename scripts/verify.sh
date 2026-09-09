#!/usr/bin/env bash
set -euo pipefail

# Thorough verification per prompt_guidelines.md -> Verification Protocol
# Modern, robust, long-lived: uses cargo fmt/check/test/clippy + flutter analyze/test

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

echo "== Rust: fmt =="
cargo fmt --all -- --check

echo "== Rust: check =="
cargo check --workspace --all-targets --all-features

echo "== Rust: clippy =="
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "== Rust: test =="
cargo test --workspace --all-features

echo "== Flutter: pub get =="
if command -v fvm >/dev/null 2>&1; then
  FLUTTER="fvm flutter"
else
  FLUTTER="flutter"
fi

pushd "$ROOT_DIR/apps/client" >/dev/null
$FLUTTER pub get
echo "== Flutter: analyze =="
$FLUTTER analyze
echo "== Flutter: test =="
$FLUTTER test
popd >/dev/null

echo "== All checks passed =="
