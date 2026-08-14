<div align="center">

<img src="assets/emerald-mark.svg" width="88" height="88" alt="">

# Emerald

**A calm, low-footprint web browser.**
Ships no rendering engine. Phones home to nothing.
Treats focus and accessibility as the product, not a panel you have to find.

</div>

---

## What this is

Emerald is a minimalist desktop browser built as a native Rust shell around the operating system's own web engine. It exists for three reasons, in this order:

1. **Feel calm.** Nothing flashes, autoplays, reorders itself, or moves the paragraph you are reading.
2. **Cost almost nothing to run.** No bundled engine, no background services, and a tab ceiling that genuinely returns memory to the OS rather than pretending to.
3. **Be usable by people whose brains don't work the "default" way.** The Focus & Access panel is first-class and reachable in one click or one keystroke from anywhere.

It is a working skeleton — the architecture, the design system, the settings model and the accessibility machinery are real and tested. [Known gaps are listed](docs/architecture.md#9-known-gaps) rather than glossed over.

## The engine, stated up front

| Platform | Engine | Chromium? |
| --- | --- | --- |
| Linux | WebKitGTK | No |
| macOS | WKWebView | No |
| **Windows** | **WebView2** | **Yes** |

Tauri has no non-Chromium Windows backend. That is a real gap and it is [documented, not hidden](docs/architecture.md#1-the-engine-decision-stated-plainly). Everything else works identically on all three.

## Documentation

| Doc | What's in it |
| --- | --- |
| **[Architecture](docs/architecture.md)** | Engine decision and what it costs, tab lifecycle, the security model, every background process justified |
| **[Design language](docs/design-language.md)** | Palette semantics, type scale, spacing, motion rules, icon style guide |
| **[Settings schema](docs/settings-schema.md)** | Every option, generated from the Rust types that define them |
| **[Benchmarks](docs/benchmarks.md)** | Measured numbers, the method, and a same-machine Chromium baseline |

## Build

Requires Rust 1.77+, Node 20+, pnpm.

```bash
# Linux only — the system engine and its headers
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
                 libayatana-appindicator3-dev build-essential

pnpm install
./scripts/install-fonts.sh    # reading typefaces, installed system-wide
pnpm app:dev                  # run
pnpm app:build                # bundle
```

Building the binary with plain `cargo build --release` produces one that loads
its UI from the dev server rather than the bundle, and it does not warn you —
use `pnpm app:build`, or add `--features custom-protocol`.

Why fonts are installed rather than bundled into pages: a site's own Content-Security-Policy can block a web font the browser injects, which would make the dyslexia-friendly font fail silently on exactly the text-heavy sites where it matters. A locally installed family is outside CSP's reach.

## Check

```bash
pnpm check      # tsc --noEmit + cargo clippy -D warnings
cargo test      # 41 unit tests, in src-tauri/
pnpm schema     # regenerate the schema, TS types and settings doc
pnpm bench      # measure; see docs/benchmarks.md
```

`settings.rs` is the single source of truth for configuration. The JSON Schema, the TypeScript types, the panel's help text and the settings documentation are all generated from it, so they cannot drift. `scripts/check-generated.sh` fails if they are stale.

## Layout

```
src/                  chrome UI — Solid + TypeScript, 75KB bundled
  icons/index.tsx     the icon set: one file, one grid, one hand
  styles/tokens.css   the design system, three layers
  lib/settings.gen.ts GENERATED from settings.rs
src-tauri/src/
  runtime.rs          the only module that touches real webviews
  gtk_layout.rs       Linux: makes multi-webview positioning work at all
  tabs.rs             tab model + discard policy — pure, 13 tests
  settings.rs         the settings schema — single source of truth
  commands.rs         IPC, two permission tiers, checked twice
  inject.rs           builds the page content script
  metrics.rs          /proc memory accounting, no dependencies
assets/content/       the page runtime: a11y, drafts, reader, autoplay guard
schema/               GENERATED JSON Schema
bench/                benchmark harness + fixture pages
```

## Keyboard

| | |
| --- | --- |
| <kbd>Ctrl/⌘</kbd><kbd>K</kbd> | Command palette — tabs, archive, bookmarks, history, settings |
| <kbd>Ctrl/⌘</kbd><kbd>⇧</kbd><kbd>A</kbd> | **Focus & Access**, from anywhere |
| <kbd>Ctrl/⌘</kbd><kbd>E</kbd> | Cycle focus mode: off → dim → solo |
| <kbd>Ctrl/⌘</kbd><kbd>⇧</kbd><kbd>R</kbd> | Reader mode |
| <kbd>Ctrl/⌘</kbd><kbd>\\</kbd> | Split view |
| <kbd>Ctrl/⌘</kbd><kbd>T</kbd> / <kbd>W</kbd> / <kbd>L</kbd> / <kbd>R</kbd> | New tab / close / address bar / reload |
| <kbd>Ctrl/⌘</kbd><kbd>1</kbd>–<kbd>9</kbd> | Switch tab |

## What Emerald will not do

There is no setting for any of these, because there is no code for them:

- **No telemetry, crash reporting, update pings, or accounts.** Emerald opens the connections you ask it to and no others; even page prefetch is off by default.
- **Nothing in the interface flashes, blinks, pulses, or breathes** — in any theme, at any animation speed, for any reason. This is [enforced in CSS](src/styles/tokens.css), not left to discipline.
- **Colour meaning never moves.** Red is destructive, yellow is needs-attention, the accent is "yours". Changing theme or accent never changes what a colour means.

## Licence

MPL-2.0. Catppuccin palette (MIT). JetBrains Mono, Atkinson Hyperlegible and OpenDyslexic are each under the SIL Open Font License.
