#!/usr/bin/env bash
# Runs locally what CI runs, so a push is not the first check. Usage: tools/check-all.sh
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

echo "== Rust: format, lint, test"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked -q

if rustup target list --installed 2>/dev/null | grep -q x86_64-pc-windows-gnu \
  && command -v x86_64-w64-mingw32-gcc >/dev/null; then
  echo "== Rust: lint for Windows (cross)"
  cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-gnu -- -D warnings
else
  echo "== Rust: Windows cross-lint skipped (needs the target and MinGW)"
fi

echo "== Guards"
tools/check-no-process-spawn.sh
cargo run -q --locked -p hmd-core --example ui_fixtures | diff -u apps/desktop/ui/src/app/data/catalog.generated.ts -

if command -v npx >/dev/null; then
  echo "== UI prototype"
  (cd apps/desktop/ui \
    && { [ -d node_modules ] || npm ci --ignore-scripts; } \
    && npx prettier --check "src/**/*.{ts,html,css}" \
    && npx ng test --watch=false \
    && npx ng build >/dev/null)
else
  echo "== UI prototype skipped (no Node.js in PATH)"
fi
echo "OK: all local checks passed"
