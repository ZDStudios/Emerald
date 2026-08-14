#!/usr/bin/env bash
# Fail if the generated settings artefacts are stale.
#
# `settings.rs` is the single source of truth for configuration. Three files are
# derived from it and none is hand-written:
#
#   schema/focus-access.schema.json   JSON Schema
#   src/lib/settings.gen.ts           TS types, defaults, and the panel's help text
#   docs/settings-schema.md           the option table published as documentation
#
# The failure this guards against is quiet: someone adds a setting, the panel
# and the docs silently describe the old set, and the documentation becomes
# fiction. Regenerating is one command, so there is no reason to tolerate drift.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

FILES=(
  schema/focus-access.schema.json
  src/lib/settings.gen.ts
  docs/settings-schema.md
)

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

for f in "${FILES[@]}"; do
  [[ -f "$f" ]] && cp "$f" "$tmp/$(basename "$f")"
done

(cd src-tauri && cargo run --quiet --bin gen-schema)

stale=0
for f in "${FILES[@]}"; do
  base="$(basename "$f")"
  if [[ ! -f "$tmp/$base" ]]; then
    echo "MISSING (now generated): $f"
    stale=1
  elif ! diff -q "$tmp/$base" "$f" >/dev/null; then
    echo "STALE: $f"
    diff -u "$tmp/$base" "$f" | head -40 || true
    stale=1
  fi
done

if [[ "$stale" -ne 0 ]]; then
  cat >&2 <<'MSG'

Generated settings artefacts are out of date.
Run `pnpm schema` and commit the result.
MSG
  exit 1
fi

echo "generated settings artefacts are up to date"
