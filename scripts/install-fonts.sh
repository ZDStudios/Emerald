#!/usr/bin/env bash
# Install Emerald's reading typefaces as *system* fonts.
#
# Why system fonts rather than web fonts injected into the page:
#
#   A site's own Content-Security-Policy can forbid `font-src` for anything the
#   browser injects. That would make the dyslexia-friendly font fail silently on
#   precisely the text-heavy, CSP-hardened sites where a reader needs it most —
#   and fail in the worst way, by simply not happening. A font already installed
#   on the machine is outside CSP's reach and always resolves.
#
# The fonts ship in the repo via npm (@fontsource/*, all SIL OFL), so this
# script copies rather than downloads: no network, and the installed version is
# exactly the one the chrome UI uses.
#
# Run without arguments for a per-user install. Pass --system for /usr/share.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODULES="$REPO/node_modules/@fontsource"

if [[ ! -d "$MODULES" ]]; then
  echo "error: $MODULES not found — run 'pnpm install' first" >&2
  exit 1
fi

case "${1:-}" in
  --system)
    DEST="/usr/share/fonts/emerald"
    NEEDS_ROOT=1
    ;;
  "" | --user)
    DEST="${XDG_DATA_HOME:-$HOME/.local/share}/fonts/emerald"
    NEEDS_ROOT=0
    ;;
  *)
    echo "usage: $0 [--user | --system]" >&2
    exit 2
    ;;
esac

if [[ "$NEEDS_ROOT" == 1 && "$EUID" -ne 0 ]]; then
  echo "error: --system needs root; re-run with sudo" >&2
  exit 1
fi

# macOS keeps user fonts elsewhere.
if [[ "$(uname)" == "Darwin" && "$NEEDS_ROOT" == 0 ]]; then
  DEST="$HOME/Library/Fonts"
fi

mkdir -p "$DEST"

count=0
for family in jetbrains-mono atkinson-hyperlegible opendyslexic; do
  src="$MODULES/$family/files"
  if [[ ! -d "$src" ]]; then
    echo "warning: $family not installed in node_modules, skipping" >&2
    continue
  fi
  # Latin, normal, regular + bold. Enough for reading; not the whole 40-file set.
  while IFS= read -r -d '' file; do
    cp -f "$file" "$DEST/"
    count=$((count + 1))
  done < <(find "$src" -name "*-latin-[47]00-normal.woff2" -print0)
done

echo "installed $count font files to $DEST"

if command -v fc-cache >/dev/null 2>&1; then
  fc-cache -f "$DEST" >/dev/null
  echo "font cache refreshed"
fi

echo
echo "Verify with:"
echo "  fc-list | grep -iE 'opendyslexic|atkinson|jetbrains'"
