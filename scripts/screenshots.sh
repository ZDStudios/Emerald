#!/usr/bin/env bash
# Regenerate the screenshots used by the README and the site.
#
# Reproducible on purpose: the demo page, the window size, the settings and the
# click coordinates all live in the repo, so anyone can rerun this and get the
# same images rather than taking someone's word for what the UI looks like.
#
# Needs: Xvfb, matchbox-window-manager, xdotool, ImageMagick (`import`).
#   sudo apt install xvfb matchbox-window-manager xdotool imagemagick
#
# Usage:  ./scripts/screenshots.sh [output-dir]

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-$REPO/docs/screenshots}"
BINARY="$REPO/src-tauri/target/release/emerald"
DISPLAY_NUM=":80"
GEOMETRY="1440x900x24"
PORT=8899

for tool in Xvfb matchbox-window-manager xdotool import; do
  command -v "$tool" >/dev/null || { echo "missing: $tool" >&2; exit 1; }
done
[[ -x "$BINARY" ]] || {
  echo "no binary — run: pnpm build && (cd src-tauri && cargo build --release --features custom-protocol)" >&2
  exit 1
}

mkdir -p "$OUT"
WORK="$(mktemp -d)"
PIDS=()
# Kill by recorded pid, never by pattern: `pkill -f` matches the *invoking*
# shell's command line too, so a pattern that appears in this script's own
# argv makes the script kill itself. It does exactly that, and the symptom is
# a mystery exit code after all the work has already succeeded.
cleanup() {
  for pid in "${PIDS[@]:-}"; do
    [[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
  done
  rm -rf "$WORK"
}
trap cleanup EXIT

# --- display -----------------------------------------------------------------
Xvfb "$DISPLAY_NUM" -screen 0 "$GEOMETRY" -nolisten tcp >/dev/null 2>&1 &
sleep 2
export DISPLAY="$DISPLAY_NUM"
# A window manager is required: without one there is no input focus, and every
# synthetic keystroke goes nowhere.
matchbox-window-manager -use_titlebar no >/dev/null 2>&1 &
sleep 2

# --- content -----------------------------------------------------------------
(cd "$OUT" && python3 -m http.server "$PORT" >/dev/null 2>&1) &
sleep 2

# --- settings ----------------------------------------------------------------
# A ceiling of 2 with 3 tabs open, so one tab is shown resting — that state is
# the whole product and it should be visible in the first screenshot.
mkdir -p "$WORK/cfg/app.emerald.browser"
cat > "$WORK/cfg/app.emerald.browser/settings.json" <<'JSON'
{
  "focus_access": { "attention": { "max_live_tabs": 2, "suspend_idle_minutes": 0 } },
  "performance": { "memory_budget_mb": 0 }
}
JSON

shot() { sleep "${2:-3}"; import -window root "$OUT/$1"; echo "  → $1"; }

# Launch Emerald with a given settings.json and three tabs of the demo page.
launch() {
  local profile="$1"
  pkill -x emerald 2>/dev/null || true
  sleep 2
  rm -rf "$WORK/cfg"
  mkdir -p "$WORK/cfg/app.emerald.browser"
  printf '%s' "$profile" > "$WORK/cfg/app.emerald.browser/settings.json"
  XDG_CONFIG_HOME="$WORK/cfg" WEBKIT_DISABLE_COMPOSITING_MODE=1 \
    "$BINARY" \
    "http://127.0.0.1:$PORT/demo-page.html" \
    "http://127.0.0.1:$PORT/demo-page.html?2" \
    "http://127.0.0.1:$PORT/demo-page.html?3" >/dev/null 2>&1 &
  PIDS+=("$!")
  sleep 14
}

echo "capturing…"

# --- Emerald's own layout: vertical sidebar ----------------------------------
launch '{
  "focus_access": { "attention": { "max_live_tabs": 2, "suspend_idle_minutes": 0 } },
  "performance": { "memory_budget_mb": 0 }
}'
shot 01-browser.png 1

# Command palette, via the sidebar button.
xdotool mousemove 69 808 click 1
shot 02-palette.png 3
xdotool key Escape; sleep 2

# Focus & Access → Attention.
xdotool mousemove 100 842 click 1
shot 03-focus-access.png 4

# Focus & Access → Reading, which carries the live type specimen.
xdotool mousemove 321 149 click 1
shot 04-reading.png 3

# --- The Chrome-shaped layout ------------------------------------------------
# A separate launch rather than clicking the preset button: coordinates inside a
# scrolling settings panel are the most fragile thing a screenshot script can
# depend on, and this settings file is exactly what the preset writes anyway.
launch '{
  "focus_access": { "attention": { "max_live_tabs": 2, "suspend_idle_minutes": 0 } },
  "appearance": {
    "tab_layout": "top",
    "show_bookmarks_bar": true,
    "always_show_tab_close": true,
    "theme": "macchiato",
    "accent": "sapphire"
  },
  "performance": { "memory_budget_mb": 0 }
}'
shot 05-chrome-like.png 1

echo "done → $OUT"
