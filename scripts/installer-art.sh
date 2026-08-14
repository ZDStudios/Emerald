#!/usr/bin/env bash
#
# Rasterise the NSIS installer artwork from its SVG sources.
#
# NSIS predates PNG support in the places these images are used: Modern UI 2
# wants uncompressed 24-bit Windows 3.x bitmaps at exactly 164x314 (welcome
# sidebar) and 150x57 (page header). Get the format wrong and the installer
# either shows nothing or fails to compile, so this script pins every part of
# it: -flatten to drop alpha onto a known ground, -alpha off so no alpha
# channel is written at all, -type TrueColor to prevent ImageMagick from
# helpfully palettising an image with few colours, and the BMP3: prefix to
# force the old header rather than BMP v4/v5.
#
# The generated .bmp files are committed, because Windows CI runners have
# neither rsvg-convert nor a reason to install it.
#
# The sidebar wordmark is live text resolved through fontconfig, so JetBrains
# Mono has to be installed as a system font before this will run. If it is not,
# fontconfig substitutes silently and cheerfully, and the installer ships a
# wordmark in the wrong typeface — which nobody would notice until it was in
# front of users. So: check, and refuse.
#
# Requires: rsvg-convert (librsvg2-bin), convert (imagemagick), and
# ./scripts/install-fonts.sh having been run.

set -euo pipefail

cd "$(dirname "$0")/.."

if ! fc-match 'JetBrains Mono' 2>/dev/null | grep -qi jetbrains; then
  echo "error: JetBrains Mono is not installed as a system font." >&2
  echo "       fontconfig would substitute another face into the wordmark." >&2
  echo "       Run: pnpm install && ./scripts/install-fonts.sh" >&2
  exit 1
fi

src=assets/installer
out=src-tauri/installer

mkdir -p "$out"

render() {
  local name=$1 w=$2 h=$3 ground=$4
  local tmp
  tmp=$(mktemp -t "emerald-$name-XXXXXX.png")
  trap 'rm -f "$tmp"' RETURN

  rsvg-convert -w "$w" -h "$h" "$src/$name.svg" -o "$tmp"
  convert "$tmp" -background "$ground" -flatten -alpha off \
    -type TrueColor "BMP3:$out/$name.bmp"

  # Verify rather than assume: a silently wrong bitmap here surfaces as a
  # broken installer on a machine we do not have.
  local got
  got=$(identify -format '%wx%h %[bit-depth] %m' "$out/$name.bmp")
  echo "  $out/$name.bmp — $got"
  [[ $got == "${w}x${h} 8 BMP3" ]] || {
    echo "unexpected bitmap format for $name: $got" >&2
    exit 1
  }
}

echo "Rendering installer artwork:"
render sidebar 164 314 '#181825'
render header 150 57 '#eff1f5'
