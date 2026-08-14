<!-- GENERATED FILE — do not edit. Source: src-tauri/src/settings.rs. Regenerate with `pnpm schema`. -->

# Settings schema

Every option Emerald persists, generated from the Rust types that define them.
Nothing here is aspirational: if an option is listed, it is read by running code.

Settings live in a single JSON file, hand-editable, at:

| Platform | Path |
| --- | --- |
| Linux | `~/.config/app.emerald.browser/settings.json` |
| macOS | `~/Library/Application Support/app.emerald.browser/settings.json` |
| Windows | `%APPDATA%\app.emerald.browser\settings.json` |

Out-of-range values are clamped on load, not rejected. Unknown keys are an
error, so a typo surfaces instead of silently doing nothing. A file that
fails to parse is moved aside to `settings.json.bak` and defaults are used.

**69 options** across 8 sections.

## Focus & Access → Attention

Decluttering, focus mode, and the motion budget. Written with ADHD in mind, though the tab ceiling is the single biggest memory lever in the browser and everyone benefits from it.

| Option | Type | Default | Range | Meaning |
| --- | --- | --- | --- | --- |
| `animation_speed` | `number` | `1` | 0 – 3 | Multiplier on every animation duration in the chrome. `0.0` removes animation entirely (transitions become instant, nothing is dropped). `1.0` is the designed speed. |
| `auto_archive_idle_minutes` | `number` | `0` | 0 – 10080 | Move a tab out of the strip and into the Archive after this many minutes without focus. Archived tabs are one keystroke away in the command palette and are never silently lost. `0` disables. |
| `badges` | `off` \| `dot` \| `count` | `dot` | — | How much unread state Emerald is willing to show you. |
| `declutter_toolbar` | `boolean` | `false` | — | Collapse the toolbar to address bar + active tab only. Everything else moves into the command palette. |
| `focus_dim_opacity` | `number` | `0.35` | 0 – 1 | Opacity applied to de-emphasised chrome in `FocusMode::Dim`. `0.0` is invisible, `1.0` is unchanged. |
| `focus_mode` | `off` \| `dim` \| `solo` | `off` | — | What "focus mode" does to everything that is not the active tab. |
| `hide_tabs_in_focus` | `boolean` | `false` | — | Hide the tab strip / sidebar entirely while focus mode is on. |
| `max_live_tabs` | `number` | `6` | 1 – 64 | Hard ceiling on how many tabs hold a live web process at once. When a new tab pushes the count past this, the least-recently-used unpinned tab is discarded: its process is terminated and its memory returned to the OS. The tab stays in the strip and reloads on click. |
| `reduce_page_motion` | `boolean` | `false` | — | Also advertise `prefers-reduced-motion: reduce` to every page, so sites that respect it calm down too. |
| `show_archive_count` | `boolean` | `true` | — | Show a single unobtrusive count of archived tabs rather than listing them. Off means the archive is silent until you open it. |
| `suspend_idle_minutes` | `number` | `20` | 0 – 1440 | Discard a tab's web process after this many minutes without focus. `0` disables time-based discarding (the `max_live_tabs` cap still applies). This is the only clock-driven policy in Emerald; see `docs/architecture.md` §6 for the one background timer it justifies. |

<details><summary>What the choices mean</summary>

**`badges`**

- `off` — No badges at all.
- `dot` — A 4px dot. No number, no colour change, no motion.
- `count` — A numeric count.

**`focus_mode`**

- `off` — Normal chrome.
- `dim` — Inactive tabs, sidebar and toolbar affordances fade to `focus_dim_opacity`; they stay clickable.
- `solo` — One thing at a time: everything except the active tab's content and the address bar is removed from the layout. Tabs remain reachable via the command palette.

</details>

## Focus & Access → Predictability

Nothing moves, plays, or reorders without being asked. Written with autism in mind. Note that `no_layout_shift` and `stable_tab_order` default to on — Emerald treats a calm baseline as the product, not as an accessibility mode you have to find.

| Option | Type | Default | Range | Meaning |
| --- | --- | --- | --- | --- |
| `autoplay` | `block_all` \| `block_audio` \| `allow` | `block_all` | — | What happens when a page tries to start playing media on load. Emerald defaults to blocking everything and showing an inline play affordance instead. |
| `confirm_destructive` | `boolean` | `true` | — | Ask before anything irreversible: closing a window with more than one tab, clearing history, discarding a recovered draft. |
| `no_layout_shift` | `boolean` | `true` | — | Reserve space for images, iframes and ads before they load, and refuse late-arriving elements the ability to push content that is already on screen. Text you have started reading will not move. |
| `sensory_profile` | `standard` \| `muted` \| `high_contrast` | `standard` | — | Colour treatment. Affects saturation and contrast only — never hue semantics, so a warning is the same colour family in every profile. |
| `show_blocked_media_placeholder` | `boolean` | `true` | — | Show a short static notice where blocked media would have played, instead of nothing at all. Off means blocked media is invisible. |
| `stable_tab_order` | `boolean` | `true` | — | Never reorder tabs automatically — no "most recently used" shuffling, no promoting a tab because it made noise. New tabs always append. |
| `suppress_popups` | `boolean` | `true` | — | Open new links in the current tab unless explicitly middle-clicked. Stops pages spawning tabs you did not ask for. |
| `transitions` | `instant` \| `fade` \| `slide` \| `spring` | `fade` | — | How chrome transitions render. `Instant` is honoured even when `animation_speed` is above zero. |

<details><summary>What the choices mean</summary>

**`autoplay`**

- `block_all` — No `<video>` or `<audio>` starts without a click. The default.
- `block_audio` — Muted video may autoplay; anything with sound may not.
- `allow` — Let pages do as they like.

**`sensory_profile`**

- `standard` — Catppuccin Mocha as designed.
- `muted` — Accents desaturated toward the base; surfaces flattened. For when saturated colour is itself the problem.
- `high_contrast` — Raised text/background contrast beyond WCAG AAA, heavier focus rings.

**`transitions`**

- `instant` — State changes are immediate. No interpolation of any kind.
- `fade` — Opacity only. Nothing translates, scales, or bounces.
- `slide` — Opacity plus a short translation on panels that slide in from an edge.
- `spring` — Slide, plus a small overshoot on panels and a staged reveal on lists. The most expressive setting Emerald offers. Still opacity and transform only, still nothing that flashes, and still zeroed by `animation_speed`.

</details>

## Focus & Access → Reading

Typography, spacing and reading aids. Written with dyslexia in mind. The spacing floors from WCAG 1.4.12 (Text Spacing) are reachable and exceedable: line height 1.5×, letter spacing 0.12em, word spacing 0.16em, paragraph spacing 2×. Applies to reader mode always, and to every page when `apply_to_all_pages` is on.

| Option | Type | Default | Range | Meaning |
| --- | --- | --- | --- | --- |
| `apply_to_all_pages` | `boolean` | `false` | — | Apply this whole section to ordinary web pages, not just reader mode. Emerald overrides page fonts and spacing with `!important` rules; some icon-font sites will show tofu. Off by default for that reason. |
| `dim_surrounding_text` | `boolean` | `false` | — | Dim everything except the paragraph under the ruler. |
| `font` | `site_preference` \| `system` \| `atkinson` \| `open_dyslexic` \| `mono` | `atkinson` | — | Typeface used for body text. |
| `font_size_pct` | `number` | `100` | 75 – 300 | Body text size as a percentage of Emerald's 16px base. |
| `force_start_align` | `boolean` | `true` | — | Force left-aligned (start-aligned) text. Justified text creates uneven word spacing, which is a known problem for dyslexic readers, so Emerald defaults to overriding it. |
| `hyphenate` | `boolean` | `false` | — | Allow the renderer to hyphenate, which keeps ragged edges shorter without justification's rivers of whitespace. |
| `letter_spacing_em` | `number` | `0` | 0 – 0.5 | Extra space between letters, in `em`. WCAG 1.4.12 asks for at least `0.12` to be *possible*; Emerald allows well past it. |
| `line_height` | `number` | `1.6` | 1 – 3 | Line box height as a multiple of font size. WCAG 1.4.12 floor is `1.5`. |
| `max_line_length_ch` | `number` | `68` | 30 – 140 | Maximum measure in `ch` units. Long lines are the single biggest tracking problem for many dyslexic readers. |
| `paragraph_spacing_em` | `number` | `1` | 0 – 4 | Space after each paragraph, in `em`. WCAG 1.4.12 floor is `2.0` times the font size. |
| `ruler` | `off` \| `line` \| `band` \| `spotlight` | `off` | — | The reading-line highlight. |
| `ruler_color` | `yellow` \| `green` \| `blue` \| `pink` \| `auto` \| `neutral` | `auto` | — | Ruler accent. `Auto` derives it from the current theme accent. |
| `ruler_follows` | `pointer` \| `keyboard` \| `both` | `pointer` | — | What moves the ruler. |
| `ruler_height_px` | `number` | `32` | 8 – 200 | Height of the ruler band, in CSS pixels. |
| `ruler_opacity` | `number` | `0.14` | 0 – 1 | Ruler opacity. Kept low by default so it guides rather than shouts. |
| `word_spacing_em` | `number` | `0` | 0 – 1 | Extra space between words, in `em`. WCAG 1.4.12 floor is `0.16`. |

<details><summary>What the choices mean</summary>

**`font`**

- `site_preference` — Leave the page's own font alone.
- `system` — The OS UI serif/sans stack.
- `atkinson` — Atkinson Hyperlegible — designed by the Braille Institute for low vision; unusually distinct letterforms. Emerald's default because it helps a wide range of readers without looking unusual.
- `open_dyslexic` — OpenDyslexic — weighted bottoms to anchor letter orientation. Divisive and heavily debated in the research; offered because plenty of people report it helps them, which is reason enough.
- `mono` — JetBrains Mono — fixed pitch. Some readers track better on a grid.

**`ruler`**

- `off` — No ruler.
- `line` — A thin underline sitting on the baseline being read.
- `band` — A translucent band covering the current line.
- `spotlight` — The current line stays lit; everything above and below is dimmed.

**`ruler_color`**

- `yellow` — 
- `green` — 
- `blue` — 
- `pink` — 
- `auto` — Follow the theme accent.
- `neutral` — Neutral grey — no chromatic tint at all.

**`ruler_follows`**

- `pointer` — The pointer's vertical position.
- `keyboard` — Up/Down arrows move it a line at a time; the pointer is ignored.
- `both` — Either input moves it.

</details>

## Focus & Access → Input

Larger targets, dictation, and never losing typed text. Written with dysgraphia in mind. `min_target_px` defaults to 44, which is WCAG 2.2 Target Size (Enhanced, AAA) rather than the 24px AA minimum.

| Option | Type | Default | Range | Meaning |
| --- | --- | --- | --- | --- |
| `autosave_drafts` | `boolean` | `true` | — | Keep a local copy of everything typed into web forms, so a crash, a misclick, or a session expiry never costs you the text. |
| `autosave_interval_ms` | `number` | `1500` | 250 – 60000 | How often a changed field is written to the draft store. |
| `draft_restore` | `auto` \| `ask` \| `off` | `ask` | — | What Emerald does when it finds a draft for a form you have just opened. |
| `draft_retention_days` | `number` | `14` | 0 – 365 | Drafts older than this are deleted. `0` keeps them until you clear them. |
| `draft_skip_sensitive` | `boolean` | `true` | — | Exclude fields Emerald believes are sensitive (password, one-time code, card number, and anything with `autocomplete="off"`) from the draft store. Strongly recommended; on by default. |
| `enlarge_form_fields` | `boolean` | `false` | — | Also grow the *visible* size of inputs, textareas and buttons. This one does change page layout, so it is separate from `min_target_px`. |
| `field_font_size_pct` | `number` | `100` | 100 – 250 | Font size inside form fields, as a percentage. Under 100% is not offered — shrinking form text is never the accessible choice. |
| `force_spellcheck` | `boolean` | `false` | — | Turn the renderer's spellchecker on everywhere, including fields where the site disabled it. |
| `guard_enter_submit` | `boolean` | `false` | — | Never submit a form on plain Enter in a single-line field unless the field is the only one in the form. Stops half-written text being sent. |
| `min_target_px` | `number` | `44` | 20 – 96 | Minimum hit area for form controls and links, in CSS pixels. Emerald grows the clickable box without moving the visual box where it can, so pages do not reflow. WCAG 2.2 AA asks for 24; AAA asks for 44. |
| `voice_input` | `boolean` | `false` | — | Show a dictation affordance on focused text inputs. Uses the platform speech API through the page's own `SpeechRecognition` where available; Emerald ships no speech model and sends no audio anywhere itself. See `docs/architecture.md` §7 for what this does and does not do. |
| `voice_input_shortcut` | `string` | `CmdOrCtrl+Shift+V` | — | Keyboard shortcut that starts/stops dictation in the focused field. |

<details><summary>What the choices mean</summary>

**`draft_restore`**

- `auto` — Refill the form immediately and say so in a dismissible bar.
- `ask` — Offer a "restore what you were writing" bar and wait.
- `off` — Keep saving drafts but never offer them; recover manually from the command palette.

</details>

## Appearance

Colour, type, and chrome layout.

| Option | Type | Default | Range | Meaning |
| --- | --- | --- | --- | --- |
| `accent` | `green` \| `teal` \| `sapphire` \| `lavender` \| `mauve` \| `peach` \| `rosewater` | `green` | — | Which Catppuccin accent carries "this is active / this is yours". |
| `always_show_tab_close` | `boolean` | `false` | — | Show the tab strip's close buttons only on hover (Emerald) or always (Chrome). Small, but it is one of the things that makes a browser feel like the one you are used to. |
| `collapsed_sidebar_favicons` | `boolean` | `true` | — | Show the favicon strip when the sidebar is collapsed. |
| `density` | `compact` \| `comfortable` \| `roomy` | `comfortable` | — | Vertical rhythm of the chrome. |
| `show_bookmarks_bar` | `boolean` | `false` | — | Show a bookmarks bar under the toolbar, Chrome-style. |
| `tab_layout` | `sidebar` \| `top` \| `hidden` | `sidebar` | — | Where tabs live. |
| `theme` | `mocha` \| `frappe` \| `macchiato` \| `latte` \| `system` | `mocha` | — | Base palette. Emerald ships Catppuccin Mocha and a light counterpart. |
| `ui_font_size_pct` | `number` | `100` | 75 – 200 | Chrome text size as a percentage of the 13px base. |

<details><summary>What the choices mean</summary>

**`tab_layout`**

- `sidebar` — Vertical sidebar, Zen/Arc style. Emerald's default: titles stay readable no matter how many tabs are open.
- `top` — Horizontal strip along the top, Chrome style.
- `hidden` — No visible tab strip. Tabs are reached through the command palette.

**`theme`**

- `mocha` — Catppuccin Mocha. Emerald's designed-for palette.
- `frappe` — Catppuccin Frappé. Warmer and lower-contrast than Mocha.
- `macchiato` — Catppuccin Macchiato. Between Frappé and Mocha.
- `latte` — Catppuccin Latte, for bright rooms.
- `system` — Follow the OS, using Mocha and Latte.

</details>

## Performance

Memory and process behaviour. `memory_budget_mb` and the idle-tab policies are the only things that cause Emerald to run a recurring timer; with all of them off it does no periodic work whatsoever.

| Option | Type | Default | Range | Meaning |
| --- | --- | --- | --- | --- |
| `allow_page_prefetch` | `boolean` | `false` | — | Allow pages to use `<link rel=prefetch/preconnect>`. Off means Emerald makes no network request you did not initiate. |
| `hardware_acceleration` | `boolean` | `true` | — | Let the renderer use the GPU. Turning this off lowers memory on some systems at the cost of scrolling smoothness. |
| `memory_budget_mb` | `number` | `0` | 0 – 65536 | Soft ceiling on total resident memory across Emerald and all of its web processes, in MiB. When exceeded, tabs are discarded LRU-first until it is back under. `0` disables the check and stops the sampler entirely. |
| `restore_tabs_discarded` | `boolean` | `true` | — | Restore tabs from the previous session as *discarded* placeholders rather than loading them. Startup stays flat regardless of how many tabs were open. |
| `sampler_interval_s` | `number` | `30` | 5 – 600 | How often the memory budget is checked, in seconds. This is the only recurring timer Emerald runs, and it does not start unless `memory_budget_mb > 0` or a time-based tab policy is enabled. |

## Privacy

What leaves the machine. The short answer is nothing that you did not ask for.

| Option | Type | Default | Range | Meaning |
| --- | --- | --- | --- | --- |
| `block_third_party_cookies` | `boolean` | `true` | — | Block third-party cookies. |
| `clear_site_data_on_exit` | `boolean` | `false` | — | Clear cookies and site data for non-pinned tabs at exit. |
| `custom_search_url` | `string` | *(empty)* | — | Custom search URL with `{q}` as the query placeholder. Used when `search_engine` is `Custom`. |
| `global_privacy_control` | `boolean` | `true` | — | Send `Do Not Track` and `Sec-GPC`. |
| `search_engine` | `duck_duck_go` \| `startpage` \| `brave` \| `google` \| `custom` | `duck_duck_go` | — | Search engine used for non-URL input in the address bar. |

<details><summary>What the choices mean</summary>

**`search_engine`**

- `duck_duck_go` — 
- `startpage` — 
- `brave` — 
- `google` — 
- `custom` — Use `custom_search_url`.

</details>

## Extensions

Chrome extensions run on Windows, where Emerald's engine is WebView2, and do              not run on macOS or Linux, whose engines have no Chrome extension system.              These options are still read and stored everywhere — installing on a platform              that cannot load them is allowed and simply does nothing until you run Emerald              somewhere that can. There is no Chrome Web Store integration; see              `docs/architecture.md` §7.5.

| Option | Type | Default | Range | Meaning |
| --- | --- | --- | --- | --- |
| `confirm_permissions` | `boolean` | `true` | — | Warn before installing an extension, showing the permissions its manifest requests. Extensions run with wide access to page content; this is on by default and turning it off is a real decision. |
| `directory` | `string` | *(empty)* | — | Where unpacked extensions live. Empty means the default: `<config dir>/extensions`. |
| `disabled` | `string[]` | `[]` | — | Extension folder names that are installed but switched off. Emerald keeps the files so re-enabling costs nothing. |
| `enabled` | `boolean` | `false` | — | Load extensions into page webviews. Has no effect on macOS or Linux. |

## Not settings, on purpose

Some behaviour is not configurable because a configuration point implies a
mode where it is off, and those modes should not exist:

- **Nothing in Emerald's chrome flashes, blinks, pulses, or breathes.** Not in
any theme, not at any animation speed, not to get your attention. There is no
setting because there is no code path that does it.
- **No telemetry, no crash reporting, no update ping, no phone-home.** Emerald
opens exactly the connections you ask it to. There is no opt-out because there
is nothing to opt out of.
- **Hue semantics are fixed.** The accent means "active, or yours"; red means
destructive; yellow means needs-attention. `sensory_profile` changes saturation
and contrast, never which colour means what, so muscle memory survives the
switch.
