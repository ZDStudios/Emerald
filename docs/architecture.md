# Architecture

## 1. The engine decision, stated plainly

**Emerald is a native Rust shell (Tauri 2) around the operating system's own web engine.** It ships no rendering engine of its own.

| Platform | Engine actually used | Chromium? |
| --- | --- | --- |
| Linux | WebKitGTK (`libwebkit2gtk-4.1`) | No |
| macOS | WKWebView (system WebKit) | No |
| Windows | **WebView2 — Chromium/Blink** | **Yes** |

That last row is a real gap, not a footnote, and it is stated here rather than buried: **on Windows, Emerald renders with Chromium.** Tauri has no non-Chromium Windows backend, and neither does anything else that is shippable. The options for closing it are (a) bundle WebKit for Windows, which reintroduces the 150MB+ engine payload the whole design exists to avoid, (b) wait for Servo's Windows support to mature, or (c) accept it. Emerald accepts it and says so. Everything else in this document — the tab policy, the accessibility injection, the settings model — is engine-agnostic and works identically on all three.

### Why this and not the alternatives

The brief named three paths. Here is the honest reckoning of each.

**A from-scratch engine is not a project, it is an industry.** Blink, Gecko and WebKit each represent thousands of engineer-years and continue to consume hundreds of engineers permanently, because the web platform is a moving target with no freeze date. Any plan that starts with "write a layout engine" produces a browser that renders 2005's web in 2030. Not considered further.

**Servo via Verso, or Ladybird.** Genuinely not Chromium, genuinely exciting, and genuinely not ready. Both are missing large parts of the platform that ordinary sites assume — and the failure mode is not "this site looks slightly wrong", it is "this site does not work and the user cannot tell why". A browser whose selling point is *reducing* cognitive load cannot ship a rendering surface where the user has to hold "does this site work in my browser?" in their head at all times. That is the specific reason this is disqualifying for **this** product rather than a general dismissal: for a browser aimed at people who are already spending executive function on the interface, unpredictable rendering is the most expensive possible bug. Revisit when Servo passes WPT at a rate comparable to WebKit.

**CEF, stripped.** This is a legitimate engineering path, and if the only goal were resource efficiency it might even win — you get uniform behaviour on all three platforms and real control over which subsystems start. It loses on two counts. First, it ships a ~150MB engine per install and pays that cost on every platform, including the two where the OS already has a perfectly good engine sitting in memory shared with other applications. Second, "strip it aggressively" is a permanent maintenance commitment: every Chromium bump risks reintroducing a background service, and verifying that it has not is ongoing work that a small project will eventually stop doing. Emerald's approach makes that whole category of work structurally impossible rather than a discipline to maintain.

**The system webview wins because of what it does not do.** It ships no engine, so the download is ~6MB rather than ~150MB. The engine's code pages are already resident and are *shared with every other app on the machine using the same webview* — which is why Emerald's proportional memory (§4) is so much lower than its resident memory. There is no separate updater, no crash reporter, no field-trial service, no variations seed fetch, because none of that was ever linked in.

### What it costs

Being honest about the other side of the trade:

- **Three engines, three behaviours.** Emerald inherits WebKit's bugs on macOS/Linux and Blink's on Windows. A rendering difference between platforms is Emerald's problem to document but not to fix.
- **No engine-level control.** Emerald cannot change how the compositor schedules, cannot implement `overflow-anchor` where WebKit lacks it (so it is implemented in JavaScript instead — see §7), and cannot add a process-per-site policy.
- **The user's OS version is the engine version.** An old macOS means an old WebKit. There is no "update Emerald to fix a site" story on macOS the way there is for a browser that bundles its engine.
- **Web Speech is absent on Linux.** WebKitGTK ships no `SpeechRecognition`. This directly limits the dictation feature (§7).

---

## 2. Shape of the program

```
┌───────────────────────────────────────────────────────────────┐
│  Rust core (one process)                                      │
│                                                               │
│   commands.rs   IPC surface, two permission tiers             │
│   runtime.rs    the ONLY module that touches real webviews    │
│   gtk_layout.rs Linux: makes webview positioning work at all  │
│   ────────────── below here: no Tauri types at all ─────────  │
│   tabs.rs       tab model + discard policy    (13 unit tests) │
│   store.rs      spaces/bookmarks/history/drafts (10 tests)    │
│   settings.rs   the settings schema            (6 tests)      │
│   inject.rs     builds page content scripts    (6 tests)      │
│   metrics.rs    /proc memory accounting        (3 tests)      │
│   extensions.rs install + inventory extensions  (8 tests)     │
└───────────────────────────────────────────────────────────────┘
             │ webview lifecycle           │ init script + eval
             ▼                             ▼
┌────────────────────────┐   ┌──────────────────────────────────┐
│ webview "chrome"       │   │ webviews "tab:1", "tab:2", …     │
│ Solid + TS, 75KB       │   │ the web. One WebKit web process  │
│ full IPC access        │   │ each. Six IPC commands, total.   │
└────────────────────────┘   └──────────────────────────────────┘
```

The layering rule is load-bearing: **policy is pure data, effects are separate.** `tabs.rs` never creates a webview. It returns a list of `Effect`s — `Create`, `Destroy`, `Navigate`, `Relayout` — and `runtime.rs` applies them. That is what makes the thing this browser most needs to get right (when memory is released, and whose memory) testable without a display server. `cargo test` runs 49 tests in under a second, including "discarding a tab emits a Destroy, not a hide".

### Window composition

One OS window contains N+1 webviews. The chrome webview fills the window; page webviews are positioned into a rectangle inside it.

```
┌─ window "main" ───────────────────────────────┐
│┌─ "chrome" (fills window, created first) ────┐│
││ sidebar │  ← everything painted here shows  ││
││         │    only when no page covers it    ││
│└─────────┴─────────────────────────────────────┤
│          ┌─ "tab:7" (positioned into insets) ┐ │
│          │  the live page                    │ │
│          └───────────────────────────────────┘ │
└───────────────────────────────────────────────┘
```

The chrome measures its own content element with a `ResizeObserver` and reports **insets in pixels**; panes are stored as **fractions** of the resulting content rect. That split matters: window resizes are handled entirely in Rust from the last known insets, with no round trip to JavaScript, so dragging a window edge does not produce a stream of IPC calls and the page never lags behind the frame.

A pleasant consequence of the compositing order: anything the chrome paints in the content rect is visible exactly when no live webview covers it — which is precisely when there is no page. That is where the empty states and the "this tab is resting" explanation live, with no visibility logic at all.

### 2.1 Overlays have to stand the pages down

The same compositing order has a sharp edge. Page webviews are **native widgets layered above the chrome webview**, so an HTML overlay the chrome draws over the content area — the command palette, the settings panel — renders *behind* the page. It opens, it takes focus, it responds to input, and it is invisible. There is no z-index that fixes this; the page is not in the chrome's stacking context, it is a different widget.

So the chrome tells the core when it is showing a full-surface overlay (`set_overlay`), and `relayout` hides every page webview until it closes. One boolean, and the failure mode it prevents is a UI that looks broken while being entirely functional.

### 2.2 Shortcuts have to come back from the page

The mirror-image problem. Because the chrome is a *sibling* of the page rather than its ancestor, a keydown while you are reading is delivered to the page's webview and the chrome's `keydown` listener never runs. Every shortcut would work only when focus happened to be in the sidebar — which is almost never, and would have made the "keyboard-first command palette" claim false in practice.

The content script therefore listens at capture phase, matches against a short allowlist of chords Emerald owns, and forwards them via `page_shortcut`. Rust re-validates against the same list (a page must not be able to drive arbitrary chrome actions) and emits `emerald://shortcut` to the chrome, which runs exactly the same handler as a locally-pressed key. The allowlist is duplicated in three places — `emerald.js`, `commands.rs`, `App.tsx` — and that duplication is a known wart; anything not on it is left entirely alone so pages keep their own shortcuts.

### 2.3 Multi-webview positioning does not work on Linux without help

`tauri-runtime-wry` builds every child webview into the window's `default_vbox()`, a `GtkBox`. wry branches on container type: a `GtkFixed` child gets `put(x, y)`, a `GtkBox` child gets `pack_start(expand, fill)`. Packed children split the box's height equally, and wry's `set_bounds` only allocates when its internal `is_in_fixed_parent` flag is set — which it is not.

The consequence is that **`Webview::set_position` and `set_size` return `Ok(())` and do nothing.** With the chrome plus one page open, each took half the window height. The API does not fail, it lies.

`gtk_layout.rs` fixes it the way wry's own `gtk_multiwebview` example does — put the webviews in a `GtkFixed` — but after the fact, since Tauri offers no way to choose the container: it packs a `GtkFixed` into the vbox at startup and reparents each webview's widget into it on first layout. Roughly 130 lines, Linux-only, and it deletes cleanly if upstream ever changes.

### Native window decorations, on purpose

Emerald does not draw its own title bar, which is unusual for a browser in this style. A reimplemented title bar behaves subtly differently from every other window on the machine: snapping, keyboard window management, screen readers, and platform gestures all drift from the OS behaviour in ways that are individually small and collectively exhausting. "Predictable and low-surprise" has to include the window itself. The novelty budget is spent on the sidebar instead.

---

## 3. Tab lifecycle — what "suspend" actually means

Three states:

| State | Webview | Web process | In the strip? | Memory |
| --- | --- | --- | --- | --- |
| `Live` | yes | yes | yes | full |
| `Discarded` | **destroyed** | **terminated** | yes, dimmed | ~300 bytes of metadata |
| `Archived` | destroyed | terminated | no — in the palette | ~300 bytes |

**Discarding calls `Webview::close()`.** The webview is destroyed and WebKit terminates the `WebKitWebProcess` backing it; the kernel reclaims its pages. This is not a visual collapse, not a "sleeping tab" that still holds its heap, and not `background_throttling` (which Emerald *also* sets, as a separate and much weaker measure for tabs that stay live). §4 measures the difference.

What survives a discard: URL, title, favicon, scroll offset, reader state, pinned state, and every form draft — because drafts live in the core's store, not in the page. Coming back is a fresh load plus a scroll restore.

Three policies can discard a tab, in order of how much users notice them:

1. **The ceiling** (`max_live_tabs`, default 6). Synchronous, on every tab open and activation. The least-recently-used eligible tab goes. This is the main lever and it needs no timer.
2. **Idle time** (`suspend_idle_minutes`, default 20; `auto_archive_idle_minutes`, default off).
3. **Memory budget** (`memory_budget_mb`, default off). Discards roughly one tab per 150MB over budget per tick, then reassesses, rather than dumping everything at once.

**Nothing on screen is ever discarded, and neither is a pinned tab.** If the ceiling cannot be met without violating that, the ceiling yields — a policy should not override an explicit choice the user made. There is a test for exactly this (`on_screen_tabs_are_never_discarded_even_over_cap`).

Startup is independent of tab count: with `restore_tabs_discarded` on (the default), a restored session creates exactly one webview — the active tab — regardless of whether you left 3 tabs open or 300.

---

## 4. Resource behaviour, measured

Full method and raw numbers: **[benchmarks.md](benchmarks.md)**. Summary of what was actually measured, on this machine, with the shipped binary and no test-only code paths:

| | Emerald | Chromium, same machine |
| --- | --- | --- |
| Binary | **7.2 MB** (no engine shipped) | ~180 MB |
| Cold start, exec → usable | 1277 ms | — |
| Idle, 1 tab (PSS) | **235 MB** | 370 MB |
| 8 tabs, Emerald capped at 2 (PSS) | **270 MB** | 448 MB |
| Processes, 8 tabs | **5** | 15 |

Suspension, using the shipped ceiling and no test hooks: 8 awake tabs → 2 returned **904 MB RSS / 135 MB PSS** and terminated **6 web processes**. Not throttled, not hidden — gone, and reloaded on click.

**The caveat that matters most:** Emerald's *marginal* cost per awake tab (~24.3 MB PSS) is roughly twice Chromium's (~11.1 MB). Emerald wins on the floor, not on the slope; the discard policy is what keeps totals down at high tab counts, not per-tab efficiency. benchmarks.md states this and two other caveats that cut against Emerald.

**Startup is the weak number:** 1.28s to a usable window, of which the Rust core accounts for 27ms and the chrome webview's own load accounts for ~1.1s. That is unattributed and unoptimised, and it is the first thing to profile.

### Read the PSS column, not the RSS column

RSS counts every physical page a process has mapped, **including pages shared with other processes**. Every WebKit web process maps the same large engine library, so a tree-total RSS counts it once per tab. PSS divides each shared page by the number of processes sharing it, and is the number that reflects what Emerald actually costs the machine. Emerald's whole argument — that not shipping an engine is cheaper — shows up in PSS and is invisible in RSS.

This cuts both ways and it should be said: **a comparison against Chrome or Arc that used RSS for one and PSS for the other would be dishonest, and so would quoting Emerald's shell process alone as "Emerald's memory".** `metrics.rs` therefore always walks the whole process tree, and the Performance settings panel shows both numbers side by side with the shell broken out, so the user can see the difference rather than take a claim on faith.

### Every background process, justified

Emerald runs **one** recurring timer, in one named thread (`emerald-policy`), and it exists to serve three settings: idle-suspend, auto-archive, and the memory budget.

**When all three are off, `Settings::needs_sampler()` is false and the thread parks on a condvar indefinitely — it does no work and consumes no CPU.** It is woken, not polled: changing a setting pokes it, and shutdown stops it.

Why a thread at all, rather than the alternatives: doing this in the chrome webview would keep a JavaScript context and its timers hot for something the user never sees; doing it on user input means a tab left idle overnight is still resident at breakfast, which is exactly the case the feature exists for. One parked thread costs a stack.

That is the complete list. There is no updater, no crash reporter, no telemetry uploader, no field-trial fetcher, no search-suggestion service, no favicon prefetcher, no safe-browsing list sync. `allow_page_prefetch` defaults to off, so Emerald opens no network connection the user did not initiate — including the speculative ones sites ask for.

---

## 5. Security: two tiers, checked twice

Emerald renders untrusted web pages as **sibling webviews of its own UI, inside the same window**. That is an unusual arrangement and it deserves an explicit threat model.

The bad outcome is a web page invoking core commands — changing settings, reading history, closing tabs. Two independent mechanisms prevent it:

**1. Tauri's ACL.** `build.rs` declares an app ACL manifest, which switches on access control for Emerald's *own* commands, not just plugin commands. Two capability files then define who may call what:

- `capabilities/chrome.json` — the `chrome` webview, **local origins only**, gets the privileged commands.
- `capabilities/pages.json` — `tab:*` webviews get **six** commands: `page_draft`, `page_drafts`, `page_clear_drafts`, `page_scroll`, `page_favicon` (write-only reports about the page's own state) and `page_shortcut` (a relay for the keyboard chords Emerald owns, re-validated in Rust against a fixed allowlist).

Note the chrome capability is scoped by `webviews: ["chrome"]` and deliberately **not** by `windows: ["main"]` — a window-scoped capability applies to every webview in that window, which would have handed the full command set to every page Emerald renders. That is a genuinely easy mistake to make and the comment in the file says so.

**2. A label check in Rust.** Every privileged command calls `guard_chrome(&webview)` and refuses anything whose label is not `chrome`. The label comes from the `Webview` handle Tauri injects, not from the payload, so a page cannot claim to be the chrome.

**Origins are derived, never accepted.** The page commands take no origin argument. `page_origin()` reads `Webview::url()` and derives the origin and path from it, so `evil.example` cannot read or overwrite `bank.example`'s drafts by asking nicely. Draft writes are additionally capped at 256KB so the autosave path cannot be used to fill the disk.

Why remote content is ACL-checked at all: Tauri's invoke path rejects any command from a non-local origin without an explicit `remote` capability. Without the manifest above, the five page commands would simply have been denied and draft recovery would have silently never worked — which is worth knowing, because "the feature quietly does nothing" is a worse failure than a loud one.

---

## 6. Settings: one source, three artefacts

`src-tauri/src/settings.rs` is the only place settings are defined. Running `pnpm schema` regenerates all three derived artefacts from it:

| Artefact | Purpose |
| --- | --- |
| `schema/focus-access.schema.json` | JSON Schema 2020-12, for validation and editors |
| `src/lib/settings.gen.ts` | TS types, default values, and **per-variant help text** used verbatim by the panel |
| `docs/settings-schema.md` | the complete option table — deliverable 4 |

They cannot drift, because none of them is written by hand. The Rust doc comments become the help text a user reads in the Focus & Access panel, which has a pleasant forcing effect: explaining a setting badly in the code produces a visibly bad settings panel.

Robustness choices, all tested:

- **Out-of-range values are clamped, not rejected.** A hand-edited `line_height: 400` yields 3.0, not an error dialog. The file is plain JSON and users are invited to edit it.
- **Unknown keys are an error** (`deny_unknown_fields`), so a typo surfaces instead of silently doing nothing.
- **A corrupt file is moved aside** to `settings.json.bak` and defaults are used; the user's file is never silently overwritten.
- **Writes are atomic** — temp file plus rename — so a power cut cannot truncate settings.
- **Partial files fill in defaults**, so a settings file written by an older version keeps working.

---

## 7. How accessibility settings reach the page

This is the part that determines whether §3 of the brief is real or decorative.

`inject.rs` converts the relevant settings into a small JSON config and pairs it with `assets/content/emerald.js`, injected as an **initialization script** — it runs *before* the page's own scripts. That timing is not incidental: the autoplay guard patches `HTMLMediaElement.prototype.play` before any page code can call it, and a guard installed later would already have lost.

Only what the page needs crosses the boundary. Search engine, memory budget and tab state are not sent; there is a test asserting the config contains no private settings.

Live updates use `eval` with an `apply(config)` call, so dragging the line-height slider reflows the page underneath the settings panel without a reload.

### What is implemented in the content script, and why

| Feature | Mechanism | Honest caveat |
| --- | --- | --- |
| Autoplay blocking | prototype patch + attribute stripping + `MutationObserver` | Rejects with `NotAllowedError`, so sites' existing fallbacks work |
| No layout shift | CSS space reservation + **JS scroll anchoring** | WebKit has no `overflow-anchor`; this is why it is hand-written |
| Reading spacing/fonts | injected stylesheet, text containers only | Overriding `*` breaks icon fonts — this is why `apply_to_all_pages` is off by default |
| Reading ruler | fixed overlay, rAF-throttled | — |
| Larger targets | `::after` hit-area overlay for links/buttons; real `min-height` for form fields | Replaced elements cannot carry pseudo-elements, hence two mechanisms |
| Form drafts | debounced `input` listener → core store; `pagehide` flush | Sensitive fields excluded by a deliberately broad heuristic |
| Reader mode | text-density extraction into a closed shadow root | Overlay, not a document replacement, so exiting restores the page exactly |
| Dictation | Web Speech API where present | **Absent on WebKitGTK — see below** |

**Fonts are installed as system fonts, not injected as web fonts.** A page's own Content-Security-Policy can forbid `font-src` for anything the browser injects, which would make the dyslexia-friendly font silently fail on exactly the content-heavy sites where it matters most. A locally installed family is outside CSP's reach. `font_stack()` therefore names families and relies on them being installed.

The families ship as `.woff2`, which fontconfig reads directly — FreeType has supported WOFF2 since 2.10.2, so there is no conversion step. Where they get installed from depends on how you got Emerald, and this is uneven:

| Install | Reading fonts on web pages |
| --- | --- |
| Linux `.deb` | **Yes** — six files into `/usr/share/fonts/emerald`, and dpkg's `fontconfig` trigger refreshes the cache |
| Dev checkout | **Yes** — `scripts/install-fonts.sh`, per-user or `--system` |
| Linux `.AppImage` | No. An AppImage does not install anything outside itself |
| macOS `.dmg`, Windows `.exe`/`.msi` | No. Neither bundler has a font-install step here |

Where the answer is no, the setting still works in Emerald's own interface — those faces are bundled into the frontend — but web page text falls back to the next family in the stack. That is a partial feature presented as a whole one, so it is listed in §9 rather than left to be discovered.

**Dictation on Linux is a real gap.** WebKitGTK ships no `SpeechRecognition` implementation. Emerald contains no speech model and sends no audio anywhere itself, so there is nothing to fall back to. Rather than showing a button that does nothing, the affordance detects the absence, says the engine has no built-in speech recognition, and points at the OS dictation — which does work in these fields. On macOS and Windows the Web Speech path is used where the engine provides it.

**Emerald's own UI in the page is inside a closed shadow root** (`attachShadow({mode:'closed'})`), so page CSS cannot restyle the draft-recovery bar and page scripts cannot read it.

---

## 7.5 Extensions: the sharpest price of not being Chromium

| Platform | Engine | Chrome extensions |
| --- | --- | --- |
| Windows | WebView2 | **Yes** — unpacked, loaded from a folder |
| macOS | WKWebView | **No.** The API does not exist |
| Linux | WebKitGTK | **No.** Its `extensions_path` loads compiled `.so` WebKit modules — a different technology that happens to share a name |

Emerald runs Chrome extensions on exactly the platform where it *is* Chromium. That symmetry is not a coincidence, it is the whole trade restated: the engine you did not ship is also the extension ecosystem you did not get.

The code is split along that line deliberately. `extensions.rs` handles installation and inventory — reading manifests, unpacking `.crx`, listing what is present — and all of that is cross-platform and unit-tested, because it is just files on disk. Loading them into a webview happens only where `browser_extensions_enabled` does anything. The settings panel reads a `supported` flag from the core and, on macOS and Linux, explains the situation instead of rendering a button that would silently do nothing. That is the specific failure the original brief warned about, and it would have been very easy to ship here.

**There is no Chrome Web Store button, and there will not be one.** The Store's `.crx` endpoint gates on a Chrome-branded user agent and serves under terms that do not cover third-party browsers. Emerald could impersonate Chrome and scrape it; several projects do. Instead it supports the two routes Chrome itself offers in developer mode: install a `.crx` or `.zip` you downloaded, or point at an unpacked folder.

Three security notes:

- **Extensions never load into the chrome webview**, only into `tab:*` webviews. An extension with access to Emerald's own UI would have access to its entire IPC surface.
- **Signatures are not verified.** Emerald has no Web Store public key to check against, and a signature from an unknown party proves nothing. The real boundary is the permission list shown before you enable anything — host permissions marked in the needs-attention hue, because those are the ones that mean "can read and rewrite these sites".
- **`remove` refuses any id containing a path separator or `..`**, so a crafted directory name cannot escape the extensions folder.

## 7.6 Two shapes, one browser

`appearance.tab_layout` picks between a vertical sidebar (Emerald's default), a horizontal strip (Chrome's shape) and no strip at all. The Appearance panel offers all three as one-click presets that also set the bookmarks bar and the tab-close behaviour.

The vertical default is the considered choice — a horizontal strip degrades into indistinguishable favicons exactly when you have enough tabs to need to tell them apart, and `TopTabs` mitigates that with a 5.5em floor and scrolling rather than pretending it does not happen. But "looks like the browser I already know" is a real accessibility property, not a concession: a familiar shape costs nothing to learn, and requiring someone to relearn tab management on day one is its own barrier. Shipping only the opinionated layout would have been the less accessible choice.

## 8. Spaces, kept deliberately small

A space is a name, an icon from Emerald's own set, an accent, and a tag on a tab. That is the whole feature.

What it is **not**: a separate profile directory, a separate cookie jar, or anything with a sync service. Arc's spaces are frequently described as heavy for what they deliver, and a browser arguing for frugality should not ship a synchronisation service to remember which tabs are work tabs. If per-space cookie isolation is wanted later, Tauri's `data_store_identifier` on the webview builder is the hook, and it would be opt-in per space because it costs a separate storage tree.

Space `0` cannot be deleted — there is always somewhere for a tab to live — and deleting a space moves its tabs home rather than destroying them. Nothing disappears without being asked for.

---

## 8.5 What has actually been run

Distinguishing "written" from "verified", because the difference matters:

**Verified by running the release binary under Xvfb and screenshotting:** the chrome renders and lays out; page webviews position correctly into the content rect; tab titles resolve; the command palette opens and searches across tabs, settings and actions; the Focus & Access panel opens, navigates its seven sections, and shows live tab counts; the Reading specimen renders in Atkinson Hyperlegible; the content script's 44px minimum target size is visibly applied to form fields; the tab ceiling terminates web processes (11 → 5, measured).

**Verified by unit test:** the discard policy, LRU victim selection, pinned/on-screen protection, the idle sweep, session restore, draft storage and sensitive-field exclusion, settings clamping and round-tripping, history ranking, layout arithmetic.

**Written but not yet verified end-to-end:** the shortcut relay from page to chrome (the code path is in place and ACL-granted, but the keyboard could not be driven reliably under Xvfb); draft recovery round-tripping through a real form on a real site; reader mode on a broad corpus; the reading ruler; dictation on any platform; everything on macOS and Windows.

## 9. Known gaps

Stated so they are not discovered as surprises:

1. **Windows renders with Chromium.** §1.
2. **No dictation on Linux.** §7.
3. **Back/forward use `history.back()`** via `eval`, because wry exposes no navigation-history API. Behaviour matches the page's own history, which is almost always right and occasionally differs on sites that manipulate history aggressively.
4. **`suppress_popups` is fixed at webview creation.** The `on_new_window` handler cannot be swapped afterwards, so changing the setting affects tabs opened after the change.
5. **Split view is two panes, horizontal.** The pane model is general (normalised rects) but the UI only builds 50/50 splits.
6. **`hardware_acceleration`, `block_third_party_cookies`, `global_privacy_control` and `clear_site_data_on_exit` are persisted and surfaced but not yet wired to engine calls.** They are in the schema because the schema is the design; they are listed here because shipping a switch that does nothing is exactly the "placeholder settings" failure the brief warns against. **Do not describe these four as working.**
7. **Reader extraction is a heuristic**, roughly 40 lines. It handles articles and documentation well and gives up gracefully on applications rather than producing a mangled page.
8. **The shortcut allowlist is duplicated in three places** — `emerald.js`, `commands.rs`, `App.tsx`. It should be generated from one source the way settings are. It is not, and a chord added to one and not the others will silently work in some contexts and not others.
9. **`file://` pages cannot use the page commands.** `capabilities/pages.json` grants the remote origins `http` and `https` only, so drafts, scroll restore and shortcut relay do not work on local files. Fine for browsing, surprising when testing with local fixtures — which is how it was found.
10. **The reading typefaces reach web pages only on `.deb` and dev installs.** The `.AppImage`, `.dmg`, `.exe` and `.msi` install no system fonts, so on those the dyslexia font and reading face apply to Emerald's own interface but not to page text. See the table in §7. Fixing it means a font-install step in three more bundlers; the `.deb` was done first because it was the one that could be tested here.
11. **Updates are checked, not applied.** `privacy.check_for_updates` asks GitHub's public releases API once per window and offers to download that platform's installer; it never replaces the running application. Tauri's updater plugin would, but it requires a minisign signature and this repository has no signing key — see `src-tauri/src/updater.rs`. Moving to the real updater needs three things: a keypair from `tauri signing generate-key`, `TAURI_SIGNING_PRIVATE_KEY` and its password in the repository's Actions secrets, and the public key plus an updater endpoint in `tauri.conf.json`. Until then the last click is a human's, which is the correct default for unsigned builds.
12. **Benchmarks are single-machine, Linux, WebKitGTK 2.52, under Xvfb with software rendering.** They are reproducible, not universal. macOS and Windows numbers are not yet collected, and Xvfb software rendering is not representative of a real GPU.
