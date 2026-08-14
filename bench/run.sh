#!/usr/bin/env bash
# Convenience wrapper: build release, then measure.
# Add a same-machine baseline with CHROMIUM=/path/to/chrome.
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

# The frontend must be built first, and the binary MUST carry
# `custom-protocol` — without that feature Tauri loads the chrome from the dev
# server at localhost:1420 instead of the bundled assets. A binary built plain
# still launches, still paints, and still benchmarks, but it is measuring a
# browser whose entire UI failed to load. Ask how I know.
pnpm build
(cd src-tauri && cargo build --release --features custom-protocol)

exec python3 bench/bench.py "$@" ${CHROMIUM:+--chromium "$CHROMIUM"}
