#!/usr/bin/env bash
# Constitution, Security constraints: process spawning only inside hmd-executor and hmd-elevated.
set -euo pipefail
cd "$(dirname "$0")/.."
hits=$(grep -rnE 'process::Command|Command::new|std::process::\{[^}]*Command' --include='*.rs' crates apps 2>/dev/null \
  | grep -vE '^crates/(hmd-executor|hmd-elevated)/' || true)
if [ -n "$hits" ]; then
  echo "Process spawning is only allowed in hmd-executor and hmd-elevated:" >&2
  echo "$hits" >&2
  exit 1
fi
echo "OK: no process spawning outside allowed crates"
