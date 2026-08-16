//! Emerald's settings model.
//!
//! This module is the **single source of truth** for the Focus & Access panel.
//! Three artefacts are derived from the types below, never hand-maintained:
//!
//!   * `schema/focus-access.schema.json` — JSON Schema (validation + docs)
//!   * `src/lib/settings.gen.ts`         — TypeScript types + defaults for the chrome
//!   * `docs/settings-schema.md`         — the human-readable option table
//!
//! Run `pnpm schema` to regenerate all three. CI fails if they are stale.
//!
//! Design rules that are deliberately *not* settings, because making them
//! configurable would mean shipping a mode where they are off:
//!
//!   * No UI element in Emerald ever flashes, blinks, pulses, or breathes.
//!     Not in any theme, not at any animation speed, not for notifications.
//!   * No telemetry, no crash reporting, no update pings, no phone-home.
//!     There is no switch for this because there is no code for it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Bump when a migration is needed. `Settings::load` tolerates older files by
/// filling in defaults for anything absent, so most additions need no bump.
pub const SETTINGS_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Root
// ---------------------------------------------------------------------------

/// Emerald's complete persisted configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// Schema version of this file. Written by Emerald; do not edit by hand.
    pub version: u32,
    /// The Focus & Access panel: attention, predictability, reading, input.
    pub focus_access: FocusAccess,
    /// Colour, type and chrome layout.
    pub appearance: Appearance,
    /// Memory and process behaviour.
    pub performance: Performance,
    /// Data that leaves the machine, and data that stays on it.
    pub privacy: Privacy,
    /// Browser extensions. Engine-dependent — see [`Extensions`].
    pub extensions: Extensions,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            focus_access: FocusAccess::default(),
            appearance: Appearance::default(),
            performance: Performance::default(),
            privacy: Privacy::default(),
            extensions: Extensions::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Focus & Access
// ---------------------------------------------------------------------------

/// The Focus & Access panel. Reachable from anywhere with `Ctrl/Cmd+Shift+A`,
/// or from the command palette in one keystroke plus one selection.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct FocusAccess {
    /// Decluttering, focus mode, motion. Written with ADHD in mind.
    pub attention: Attention,
    /// Low-surprise, predictable UI. Written with autism in mind.
    pub predictability: Predictability,
    /// Typography and reading aids. Written with dyslexia in mind.
    pub reading: Reading,
    /// Targets, dictation and never losing typed text. Written with
    /// dysgraphia in mind.
    pub input: InputAssist,
}

// --- Attention -------------------------------------------------------------

/// Tab decluttering, focus mode, and motion budget.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Attention {
    /// Hard ceiling on how many tabs hold a live web process at once. When a
    /// new tab pushes the count past this, the least-recently-used unpinned
    /// tab is discarded: its process is terminated and its memory returned to
    /// the OS. The tab stays in the strip and reloads on click.
    #[schemars(range(min = 1, max = 64))]
    pub max_live_tabs: u16,

    /// Discard a tab's web process after this many minutes without focus.
    /// `0` disables time-based discarding (the `max_live_tabs` cap still
    /// applies). This is the only clock-driven policy in Emerald; see
    /// `docs/architecture.md` §6 for the one background timer it justifies.
    #[schemars(range(min = 0, max = 1440))]
    pub suspend_idle_minutes: u16,

    /// Move a tab out of the strip and into the Archive after this many
    /// minutes without focus. Archived tabs are one keystroke away in the
    /// command palette and are never silently lost. `0` disables.
    #[schemars(range(min = 0, max = 10080))]
    pub auto_archive_idle_minutes: u16,

    /// What "focus mode" does to everything that is not the active tab.
    pub focus_mode: FocusMode,

    /// Opacity applied to de-emphasised chrome in `FocusMode::Dim`.
    /// `0.0` is invisible, `1.0` is unchanged.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub focus_dim_opacity: f32,

    /// Hide the tab strip / sidebar entirely while focus mode is on.
    pub hide_tabs_in_focus: bool,

    /// How much unread state Emerald is willing to show you.
    pub badges: BadgeStyle,

    /// Multiplier on every animation duration in the chrome. `0.0` removes
    /// animation entirely (transitions become instant, nothing is dropped).
    /// `1.0` is the designed speed.
    #[schemars(range(min = 0.0, max = 3.0))]
    pub animation_speed: f32,

    /// Also advertise `prefers-reduced-motion: reduce` to every page, so sites
    /// that respect it calm down too.
    pub reduce_page_motion: bool,

    /// Collapse the toolbar to address bar + active tab only. Everything else
    /// moves into the command palette.
    pub declutter_toolbar: bool,

    /// Show a single unobtrusive count of archived tabs rather than listing
    /// them. Off means the archive is silent until you open it.
    pub show_archive_count: bool,
}

impl Default for Attention {
    fn default() -> Self {
        Self {
            max_live_tabs: 6,
            suspend_idle_minutes: 20,
            auto_archive_idle_minutes: 0,
            focus_mode: FocusMode::Off,
            focus_dim_opacity: 0.35,
            hide_tabs_in_focus: false,
            badges: BadgeStyle::Dot,
            animation_speed: 1.0,
            reduce_page_motion: false,
            declutter_toolbar: false,
            show_archive_count: true,
        }
    }
}

/// How aggressively Emerald removes everything that is not the active tab.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FocusMode {
    /// Normal chrome.
    #[default]
    Off,
    /// Inactive tabs, sidebar and toolbar affordances fade to
    /// `focus_dim_opacity`; they stay clickable.
    Dim,
    /// One thing at a time: everything except the active tab's content and the
    /// address bar is removed from the layout. Tabs remain reachable via the
    /// command palette.
    Solo,
}

/// Notification badge treatment.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BadgeStyle {
    /// No badges at all.
    Off,
    /// A 4px dot. No number, no colour change, no motion.
    #[default]
    Dot,
    /// A numeric count.
    Count,
}

// --- Predictability --------------------------------------------------------

/// Low-surprise behaviour: nothing moves, plays, or reorders without asking.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Predictability {
    /// What happens when a page tries to start playing media on load.
    /// Emerald defaults to blocking everything and showing an inline
    /// play affordance instead.
    pub autoplay: AutoplayPolicy,

    /// Reserve space for images, iframes and ads before they load, and refuse
    /// late-arriving elements the ability to push content that is already on
    /// screen. Text you have started reading will not move.
    pub no_layout_shift: bool,

    /// How chrome transitions render. `Instant` is honoured even when
    /// `animation_speed` is above zero.
    pub transitions: TransitionStyle,

    /// Never reorder tabs automatically — no "most recently used" shuffling,
    /// no promoting a tab because it made noise. New tabs always append.
    pub stable_tab_order: bool,

    /// Ask before anything irreversible: closing a window with more than one
    /// tab, clearing history, discarding a recovered draft.
    pub confirm_destructive: bool,

    /// Colour treatment. Affects saturation and contrast only — never hue
    /// semantics, so a warning is the same colour family in every profile.
    pub sensory_profile: SensoryProfile,

    /// Show a short static notice where blocked media would have played,
    /// instead of nothing at all. Off means blocked media is invisible.
    pub show_blocked_media_placeholder: bool,

    /// Open new links in the current tab unless explicitly middle-clicked.
    /// Stops pages spawning tabs you did not ask for.
    pub suppress_popups: bool,
}

impl Default for Predictability {
    fn default() -> Self {
        Self {
            autoplay: AutoplayPolicy::BlockAll,
            no_layout_shift: true,
            transitions: TransitionStyle::Fade,
            stable_tab_order: true,
            confirm_destructive: true,
            sensory_profile: SensoryProfile::Standard,
            show_blocked_media_placeholder: true,
            suppress_popups: true,
        }
    }
}

/// Autoplay handling.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutoplayPolicy {
    /// No `<video>` or `<audio>` starts without a click. The default.
    #[default]
    BlockAll,
    /// Muted video may autoplay; anything with sound may not.
    BlockAudio,
    /// Let pages do as they like.
    Allow,
}

/// Chrome transition style.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionStyle {
    /// State changes are immediate. No interpolation of any kind.
    Instant,
    /// Opacity only. Nothing translates, scales, or bounces.
    #[default]
    Fade,
    /// Opacity plus a short translation on panels that slide in from an edge.
    Slide,
    /// Slide, plus a small overshoot on panels and a staged reveal on lists.
    /// The most expressive setting Emerald offers. Still opacity and transform
    /// only, still nothing that flashes, and still zeroed by `animation_speed`.
    Spring,
}

/// Colour intensity profile. Hue meaning is constant across all three.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SensoryProfile {
    /// Catppuccin Mocha as designed.
    #[default]
    Standard,
    /// Accents desaturated toward the base; surfaces flattened. For when
    /// saturated colour is itself the problem.
    Muted,
    /// Raised text/background contrast beyond WCAG AAA, heavier focus rings.
    HighContrast,
}

// --- Reading ---------------------------------------------------------------

/// Typography, spacing and reading aids. Applies to reader mode always, and to
/// every page when `apply_to_all_pages` is on.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Reading {
    /// Typeface used for body text.
    pub font: ReadingFont,

    /// Apply this whole section to ordinary web pages, not just reader mode.
    /// Emerald overrides page fonts and spacing with `!important` rules; some
    /// icon-font sites will show tofu. Off by default for that reason.
    pub apply_to_all_pages: bool,

    /// Extra space between letters, in `em`. WCAG 1.4.12 asks for at least
    /// `0.12` to be *possible*; Emerald allows well past it.
    #[schemars(range(min = 0.0, max = 0.5))]
    pub letter_spacing_em: f32,

    /// Extra space between words, in `em`. WCAG 1.4.12 floor is `0.16`.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub word_spacing_em: f32,

    /// Line box height as a multiple of font size. WCAG 1.4.12 floor is `1.5`.
    #[schemars(range(min = 1.0, max = 3.0))]
    pub line_height: f32,

    /// Space after each paragraph, in `em`. WCAG 1.4.12 floor is `2.0` times
    /// the font size.
    #[schemars(range(min = 0.0, max = 4.0))]
    pub paragraph_spacing_em: f32,

    /// Body text size as a percentage of Emerald's 16px base.
    #[schemars(range(min = 75, max = 300))]
    pub font_size_pct: u16,

    /// Maximum measure in `ch` units. Long lines are the single biggest
    /// tracking problem for many dyslexic readers.
    #[schemars(range(min = 30, max = 140))]
    pub max_line_length_ch: u16,

    /// The reading-line highlight.
    pub ruler: RulerMode,

    /// Height of the ruler band, in CSS pixels.
    #[schemars(range(min = 8, max = 200))]
    pub ruler_height_px: u16,

    /// Ruler opacity. Kept low by default so it guides rather than shouts.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub ruler_opacity: f32,

    /// Ruler accent. `Auto` derives it from the current theme accent.
    pub ruler_color: RulerColor,

    /// What moves the ruler.
    pub ruler_follows: RulerFollows,

    /// Allow the renderer to hyphenate, which keeps ragged edges shorter
    /// without justification's rivers of whitespace.
    pub hyphenate: bool,

    /// Force left-aligned (start-aligned) text. Justified text creates
    /// uneven word spacing, which is a known problem for dyslexic readers,
    /// so Emerald defaults to overriding it.
    pub force_start_align: bool,

    /// Dim everything except the paragraph under the ruler.
    pub dim_surrounding_text: bool,
}

impl Default for Reading {
    fn default() -> Self {
        Self {
            font: ReadingFont::Atkinson,
            apply_to_all_pages: false,
            letter_spacing_em: 0.0,
            word_spacing_em: 0.0,
            line_height: 1.6,
            paragraph_spacing_em: 1.0,
            font_size_pct: 100,
            max_line_length_ch: 68,
            ruler: RulerMode::Off,
            ruler_height_px: 32,
            ruler_opacity: 0.14,
            ruler_color: RulerColor::Auto,
            ruler_follows: RulerFollows::Pointer,
            hyphenate: false,
            force_start_align: true,
            dim_surrounding_text: false,
        }
    }
}

/// Body typeface for reader mode and (optionally) all pages.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadingFont {
    /// Leave the page's own font alone.
    SitePreference,
    /// The OS UI serif/sans stack.
    System,
    /// Atkinson Hyperlegible — designed by the Braille Institute for low
    /// vision; unusually distinct letterforms. Emerald's default because it
    /// helps a wide range of readers without looking unusual.
    #[default]
    Atkinson,
    /// OpenDyslexic — weighted bottoms to anchor letter orientation.
    /// Divisive and heavily debated in the research; offered because plenty
    /// of people report it helps them, which is reason enough.
    OpenDyslexic,
    /// JetBrains Mono — fixed pitch. Some readers track better on a grid.
    Mono,
}

/// Reading-line highlight style.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RulerMode {
    /// No ruler.
    #[default]
    Off,
    /// A thin underline sitting on the baseline being read.
    Line,
    /// A translucent band covering the current line.
    Band,
    /// The current line stays lit; everything above and below is dimmed.
    Spotlight,
}

/// Ruler colour.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RulerColor {
    /// Follow the theme accent.
    #[default]
    Auto,
    Yellow,
    Green,
    Blue,
    Pink,
    /// Neutral grey — no chromatic tint at all.
    Neutral,
}

/// What drives the ruler's position.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RulerFollows {
    /// The pointer's vertical position.
    #[default]
    Pointer,
    /// Up/Down arrows move it a line at a time; the pointer is ignored.
    Keyboard,
    /// Either input moves it.
    Both,
}

// --- Input assist ----------------------------------------------------------

/// Making text entry survivable: bigger targets, dictation, and never losing
/// what was typed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct InputAssist {
    /// Minimum hit area for form controls and links, in CSS pixels. Emerald
    /// grows the clickable box without moving the visual box where it can, so
    /// pages do not reflow. WCAG 2.2 AA asks for 24; AAA asks for 44.
    #[schemars(range(min = 20, max = 96))]
    pub min_target_px: u16,

    /// Also grow the *visible* size of inputs, textareas and buttons. This one
    /// does change page layout, so it is separate from `min_target_px`.
    pub enlarge_form_fields: bool,

    /// Font size inside form fields, as a percentage. Under 100% is not
    /// offered — shrinking form text is never the accessible choice.
    #[schemars(range(min = 100, max = 250))]
    pub field_font_size_pct: u16,

    /// Show a dictation affordance on focused text inputs. Uses the platform
    /// speech API through the page's own `SpeechRecognition` where available;
    /// Emerald ships no speech model and sends no audio anywhere itself.
    /// See `docs/architecture.md` §7 for what this does and does not do.
    pub voice_input: bool,

    /// Keyboard shortcut that starts/stops dictation in the focused field.
    pub voice_input_shortcut: String,

    /// Keep a local copy of everything typed into web forms, so a crash, a
    /// misclick, or a session expiry never costs you the text.
    pub autosave_drafts: bool,

    /// How often a changed field is written to the draft store.
    #[schemars(range(min = 250, max = 60000))]
    pub autosave_interval_ms: u32,

    /// Drafts older than this are deleted. `0` keeps them until you clear them.
    #[schemars(range(min = 0, max = 365))]
    pub draft_retention_days: u16,

    /// What Emerald does when it finds a draft for a form you have just opened.
    pub draft_restore: DraftRestore,

    /// Exclude fields Emerald believes are sensitive (password, one-time code,
    /// card number, and anything with `autocomplete="off"`) from the draft
    /// store. Strongly recommended; on by default.
    pub draft_skip_sensitive: bool,

    /// Turn the renderer's spellchecker on everywhere, including fields where
    /// the site disabled it.
    pub force_spellcheck: bool,

    /// Never submit a form on plain Enter in a single-line field unless the
    /// field is the only one in the form. Stops half-written text being sent.
    pub guard_enter_submit: bool,
}

impl Default for InputAssist {
    fn default() -> Self {
        Self {
            min_target_px: 44,
            enlarge_form_fields: false,
            field_font_size_pct: 100,
            voice_input: false,
            voice_input_shortcut: "CmdOrCtrl+Shift+V".into(),
            autosave_drafts: true,
            autosave_interval_ms: 1500,
            draft_retention_days: 14,
            draft_restore: DraftRestore::Ask,
            draft_skip_sensitive: true,
            force_spellcheck: false,
            guard_enter_submit: false,
        }
    }
}

/// Draft recovery behaviour.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DraftRestore {
    /// Refill the form immediately and say so in a dismissible bar.
    Auto,
    /// Offer a "restore what you were writing" bar and wait.
    #[default]
    Ask,
    /// Keep saving drafts but never offer them; recover manually from the
    /// command palette.
    Off,
}

// ---------------------------------------------------------------------------
// Appearance
// ---------------------------------------------------------------------------

/// Colour, type and chrome layout.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    /// Base palette. Emerald ships Catppuccin Mocha and a light counterpart.
    pub theme: Theme,
    /// Which Catppuccin accent carries "this is active / this is yours".
    pub accent: Accent,
    /// Where tabs live.
    pub tab_layout: TabLayout,
    /// Chrome text size as a percentage of the 13px base.
    #[schemars(range(min = 75, max = 200))]
    pub ui_font_size_pct: u16,
    /// Vertical rhythm of the chrome.
    pub density: Density,
    /// Show the favicon strip when the sidebar is collapsed.
    pub collapsed_sidebar_favicons: bool,
    /// Show a bookmarks bar under the toolbar, Chrome-style.
    pub show_bookmarks_bar: bool,
    /// Show the tab strip's close buttons only on hover (Emerald) or always
    /// (Chrome). Small, but it is one of the things that makes a browser feel
    /// like the one you are used to.
    pub always_show_tab_close: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: Theme::Mocha,
            accent: Accent::Green,
            tab_layout: TabLayout::Sidebar,
            ui_font_size_pct: 100,
            density: Density::Comfortable,
            collapsed_sidebar_favicons: true,
            show_bookmarks_bar: false,
            always_show_tab_close: false,
        }
    }
}

/// Base palette.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Catppuccin Mocha. Emerald's designed-for palette.
    #[default]
    Mocha,
    /// Catppuccin Frappé. Warmer and lower-contrast than Mocha.
    Frappe,
    /// Catppuccin Macchiato. Between Frappé and Mocha.
    Macchiato,
    /// Catppuccin Latte, for bright rooms.
    Latte,
    /// Follow the OS, using Mocha and Latte.
    System,
}

/// Accent hue. Semantics are fixed regardless of choice: the accent means
/// "active or yours", red means destructive, yellow means needs-attention.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Accent {
    #[default]
    Green,
    Teal,
    Sapphire,
    Lavender,
    Mauve,
    Peach,
    Rosewater,
}

/// Tab strip placement.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TabLayout {
    /// Vertical sidebar, Zen/Arc style. Emerald's default: titles stay
    /// readable no matter how many tabs are open.
    #[default]
    Sidebar,
    /// Horizontal strip along the top, Chrome style.
    Top,
    /// No visible tab strip. Tabs are reached through the command palette.
    Hidden,
}

/// Chrome density.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    Compact,
    #[default]
    Comfortable,
    Roomy,
}

// ---------------------------------------------------------------------------
// Performance
// ---------------------------------------------------------------------------

/// Memory and process behaviour.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Performance {
    /// Soft ceiling on total resident memory across Emerald and all of its web
    /// processes, in MiB. When exceeded, tabs are discarded LRU-first until it
    /// is back under. `0` disables the check and stops the sampler entirely.
    #[schemars(range(min = 0, max = 65536))]
    pub memory_budget_mb: u32,

    /// How often the memory budget is checked, in seconds. This is the only
    /// recurring timer Emerald runs, and it does not start unless
    /// `memory_budget_mb > 0` or a time-based tab policy is enabled.
    #[schemars(range(min = 5, max = 600))]
    pub sampler_interval_s: u32,

    /// Restore tabs from the previous session as *discarded* placeholders
    /// rather than loading them. Startup stays flat regardless of how many
    /// tabs were open.
    pub restore_tabs_discarded: bool,

    /// Let the renderer use the GPU. Turning this off lowers memory on some
    /// systems at the cost of scrolling smoothness.
    pub hardware_acceleration: bool,

    /// Allow pages to use `<link rel=prefetch/preconnect>`. Off means Emerald
    /// makes no network request you did not initiate.
    pub allow_page_prefetch: bool,
}

impl Default for Performance {
    fn default() -> Self {
        Self {
            memory_budget_mb: 0,
            sampler_interval_s: 30,
            restore_tabs_discarded: true,
            hardware_acceleration: true,
            allow_page_prefetch: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Privacy
// ---------------------------------------------------------------------------

/// What leaves the machine. The short answer is "nothing".
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Privacy {
    /// Search engine used for non-URL input in the address bar.
    pub search_engine: SearchEngine,
    /// Custom search URL with `{q}` as the query placeholder. Used when
    /// `search_engine` is `Custom`.
    pub custom_search_url: String,
    /// Send `Do Not Track` and `Sec-GPC`.
    pub global_privacy_control: bool,
    /// Clear cookies and site data for non-pinned tabs at exit.
    pub clear_site_data_on_exit: bool,
    /// Block third-party cookies.
    pub block_third_party_cookies: bool,
}

impl Default for Privacy {
    fn default() -> Self {
        Self {
            search_engine: SearchEngine::DuckDuckGo,
            custom_search_url: String::new(),
            global_privacy_control: true,
            clear_site_data_on_exit: false,
            block_third_party_cookies: true,
        }
    }
}

/// Address-bar search provider.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchEngine {
    #[default]
    DuckDuckGo,
    Startpage,
    Brave,
    Google,
    /// Use `custom_search_url`.
    Custom,
}

impl SearchEngine {
    pub fn template(self, custom: &str) -> String {
        match self {
            Self::DuckDuckGo => "https://duckduckgo.com/?q={q}".into(),
            Self::Startpage => "https://www.startpage.com/sp/search?query={q}".into(),
            Self::Brave => "https://search.brave.com/search?q={q}".into(),
            Self::Google => "https://www.google.com/search?q={q}".into(),
            Self::Custom => custom.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Extensions
// ---------------------------------------------------------------------------

/// Browser extensions.
///
/// **Read this before enabling it.** Extension support is not Emerald's to
/// give — it belongs to whichever engine the operating system provides, and
/// the three are not the same:
///
/// | Platform | Engine | Chrome extensions? |
/// | --- | --- | --- |
/// | Windows | WebView2 | **Yes**, unpacked, loaded from a folder |
/// | macOS | WKWebView | **No.** The API does not exist |
/// | Linux | WebKitGTK | **No.** Its `extensions_path` loads compiled `.so` WebKit extensions, which are a different technology that happens to share a name |
///
/// So Emerald can run a Chrome extension on Windows and cannot on the two
/// platforms where it is *not* using Chromium. That is the direct cost of the
/// engine decision in `docs/architecture.md` §1, and the settings panel says so
/// on the platforms where it applies rather than showing a button that does
/// nothing.
///
/// There is also no one-click Chrome Web Store install anywhere. The Web Store
/// serves `.crx` files to Chrome-branded user agents under terms that do not
/// cover third-party browsers, so Emerald does not scrape it. What it does
/// support is installing a `.crx` **you** downloaded, and loading an unpacked
/// extension folder — the same two routes Chrome itself offers in developer
/// mode.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Extensions {
    /// Load extensions into page webviews. Has no effect on macOS or Linux.
    pub enabled: bool,

    /// Where unpacked extensions live. Empty means the default:
    /// `<config dir>/extensions`.
    pub directory: String,

    /// Extension folder names that are installed but switched off. Emerald
    /// keeps the files so re-enabling costs nothing.
    pub disabled: Vec<String>,

    /// Warn before installing an extension, showing the permissions its
    /// manifest requests. Extensions run with wide access to page content;
    /// this is on by default and turning it off is a real decision.
    pub confirm_permissions: bool,
}

impl Default for Extensions {
    fn default() -> Self {
        Self {
            enabled: false,
            directory: String::new(),
            disabled: Vec::new(),
            confirm_permissions: true,
        }
    }
}

impl Extensions {
    /// Resolve the extensions directory against the profile directory.
    pub fn dir(&self, config_dir: &Path) -> PathBuf {
        if self.directory.trim().is_empty() {
            config_dir.join("extensions")
        } else {
            PathBuf::from(&self.directory)
        }
    }

    /// True where the platform's engine can actually run Chrome extensions.
    pub const fn supported_here() -> bool {
        cfg!(target_os = "windows")
    }
}

// ---------------------------------------------------------------------------
// Validation + persistence
// ---------------------------------------------------------------------------

/// `x = x.clamp(lo, hi)`, written once so the ranges below read as a table.
/// Uses `Ord::clamp`/`f32::clamp` rather than hand-rolled comparisons so a
/// zero lower bound on an unsigned field is a no-op instead of a warning.
macro_rules! clamp_field {
    ($s:expr, $lo:expr, $hi:expr) => {
        $s = $s.clamp($lo, $hi)
    };
}

impl Settings {
    /// Force every numeric field back inside its documented range.
    ///
    /// The settings file is plain JSON and users are invited to edit it. A
    /// hand-written `line_height: 400` should produce a clamped value, not an
    /// unusable browser, and never an error dialog.
    pub fn clamp(&mut self) {
        let a = &mut self.focus_access.attention;
        clamp_field!(a.max_live_tabs, 1, 64);
        clamp_field!(a.suspend_idle_minutes, 0, 1440);
        clamp_field!(a.auto_archive_idle_minutes, 0, 10080);
        clamp_field!(a.focus_dim_opacity, 0.0, 1.0);
        clamp_field!(a.animation_speed, 0.0, 3.0);

        let r = &mut self.focus_access.reading;
        clamp_field!(r.letter_spacing_em, 0.0, 0.5);
        clamp_field!(r.word_spacing_em, 0.0, 1.0);
        clamp_field!(r.line_height, 1.0, 3.0);
        clamp_field!(r.paragraph_spacing_em, 0.0, 4.0);
        clamp_field!(r.font_size_pct, 75, 300);
        clamp_field!(r.max_line_length_ch, 30, 140);
        clamp_field!(r.ruler_height_px, 8, 200);
        clamp_field!(r.ruler_opacity, 0.0, 1.0);

        let i = &mut self.focus_access.input;
        clamp_field!(i.min_target_px, 20, 96);
        clamp_field!(i.field_font_size_pct, 100, 250);
        clamp_field!(i.autosave_interval_ms, 250, 60_000);
        clamp_field!(i.draft_retention_days, 0, 365);

        clamp_field!(self.appearance.ui_font_size_pct, 75, 200);
        clamp_field!(self.performance.memory_budget_mb, 0, 65_536);
        clamp_field!(self.performance.sampler_interval_s, 5, 600);

        self.version = SETTINGS_VERSION;
    }

    /// True when any policy needs a recurring timer. When this is false,
    /// Emerald runs no periodic work at all — see `docs/architecture.md` §6.
    pub fn needs_sampler(&self) -> bool {
        let a = &self.focus_access.attention;
        a.suspend_idle_minutes > 0
            || a.auto_archive_idle_minutes > 0
            || self.performance.memory_budget_mb > 0
    }

    /// Resolve a query typed into the address bar to a navigable URL.
    pub fn resolve_query(&self, input: &str) -> String {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return "about:blank".into();
        }
        // Already a URL with a scheme we can hand straight to the engine.
        if let Ok(u) = url::Url::parse(trimmed) {
            if matches!(u.scheme(), "http" | "https" | "file" | "about" | "data") {
                return trimmed.to_string();
            }
        }
        // `example.com`, `localhost:3000`, `10.0.0.4/status` — host-looking and
        // space-free, so treat as a URL rather than a search.
        let host = trimmed.split('/').next().unwrap_or(trimmed);
        let bare = host.split(':').next().unwrap_or(host);
        // A bare IPv4 literal is a host, not a search. `host_has_tld` asks for
        // an alphabetic last label, so `127.0.0.1:3000` used to be handed to
        // the search engine — which is a strange thing for a browser to do
        // with an address.
        let is_ipv4 = !bare.is_empty()
            && bare.split('.').count() == 4
            && bare
                .split('.')
                .all(|o| !o.is_empty() && o.len() <= 3 && o.bytes().all(|b| b.is_ascii_digit()));
        let loopback = bare == "localhost"
            || bare == "127.0.0.1"
            || bare == "[::1]"
            || bare.ends_with(".localhost");
        let looks_like_host =
            !trimmed.contains(' ') && (loopback || is_ipv4 || host_has_tld(bare));
        if looks_like_host {
            // `localhost` and loopback literals get http, everything else https.
            //
            // Nothing serves https on localhost without a certificate someone
            // had to go and make, so defaulting a dev server to https turns
            // `localhost:8899` into a connection error and makes the address
            // bar look broken. Every other browser special-cases loopback the
            // same way. Found by typing `localhost:8899` at a local server and
            // watching Emerald navigate confidently to `https://localhost:8899`.
            let scheme = if loopback { "http" } else { "https" };
            return format!("{scheme}://{trimmed}");
        }
        let template = self
            .privacy
            .search_engine
            .template(&self.privacy.custom_search_url);
        if template.is_empty() {
            return format!("https://duckduckgo.com/?q={}", percent_encode(trimmed));
        }
        template.replace("{q}", &percent_encode(trimmed))
    }

    pub fn path(config_dir: &Path) -> PathBuf {
        config_dir.join("settings.json")
    }

    /// Read settings from disk, tolerating absence and damage.
    ///
    /// A missing file yields defaults. A corrupt file yields defaults *and*
    /// leaves the broken file renamed to `settings.json.bak` rather than
    /// silently overwriting whatever the user had.
    pub fn load(config_dir: &Path) -> Self {
        let path = Self::path(config_dir);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        match serde_json::from_str::<Self>(&raw) {
            Ok(mut s) => {
                s.clamp();
                s
            }
            Err(err) => {
                eprintln!("emerald: settings.json is not readable ({err}); using defaults");
                let _ = std::fs::rename(&path, path.with_extension("json.bak"));
                Self::default()
            }
        }
    }

    /// Write settings atomically: full write to a temp file in the same
    /// directory, then rename. A power cut mid-save cannot truncate the file.
    pub fn save(&self, config_dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(config_dir)?;
        let path = Self::path(config_dir);
        let tmp = path.with_extension("json.tmp");
        let body = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, &path)
    }
}

fn host_has_tld(host: &str) -> bool {
    match host.rsplit_once('.') {
        Some((left, tld)) => {
            !left.is_empty()
                && tld.len() >= 2
                && tld.chars().all(|c| c.is_ascii_alphabetic())
                && !host.starts_with('.')
        }
        None => false,
    }
}

/// Minimal `application/x-www-form-urlencoded` component encoder. Pulling a
/// crate in for eleven lines would be silly.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip_through_json() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn partial_file_fills_in_defaults() {
        // Only one leaf specified; everything else must come from Default.
        let json = r#"{"focus_access":{"reading":{"font":"open_dyslexic"}}}"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.focus_access.reading.font, ReadingFont::OpenDyslexic);
        assert_eq!(s.focus_access.reading.line_height, 1.6);
        assert_eq!(s.focus_access.attention.max_live_tabs, 6);
    }

    #[test]
    fn out_of_range_values_are_clamped_not_rejected() {
        let json = r#"{"focus_access":{"reading":{"line_height":400.0,"font_size_pct":9999},
                        "attention":{"max_live_tabs":0}}}"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.clamp();
        assert_eq!(s.focus_access.reading.line_height, 3.0);
        assert_eq!(s.focus_access.reading.font_size_pct, 300);
        assert_eq!(s.focus_access.attention.max_live_tabs, 1);
    }

    #[test]
    fn sampler_is_off_when_no_policy_needs_it() {
        let mut s = Settings::default();
        s.focus_access.attention.suspend_idle_minutes = 0;
        s.focus_access.attention.auto_archive_idle_minutes = 0;
        s.performance.memory_budget_mb = 0;
        assert!(!s.needs_sampler());

        s.performance.memory_budget_mb = 512;
        assert!(s.needs_sampler());
    }

    #[test]
    fn address_bar_distinguishes_urls_from_searches() {
        let s = Settings::default();
        assert_eq!(s.resolve_query("https://a.example/x"), "https://a.example/x");
        assert_eq!(s.resolve_query("example.com"), "https://example.com");
        assert_eq!(s.resolve_query("localhost:1420"), "http://localhost:1420");
        assert!(s.resolve_query("how do i tie a bowline").contains("duckduckgo"));
        assert!(s.resolve_query("how do i tie a bowline").contains("bowline"));
        // A bare word is a search, not a hostname.
        assert!(s.resolve_query("recipes").contains("duckduckgo"));
    }

    #[test]
    fn unknown_keys_are_rejected_so_typos_surface() {
        let json = r#"{"focus_acess":{}}"#;
        assert!(serde_json::from_str::<Settings>(json).is_err());
    }

    #[test]
    fn loopback_resolves_to_http_not_https() {
        // Nothing serves https on localhost without a certificate someone made,
        // so defaulting a dev server to https is a guaranteed connection error.
        let s = Settings::default();
        assert_eq!(s.resolve_query("localhost:8899"), "http://localhost:8899");
        assert_eq!(s.resolve_query("localhost"), "http://localhost");
        assert_eq!(s.resolve_query("127.0.0.1:3000"), "http://127.0.0.1:3000");
        assert_eq!(s.resolve_query("app.localhost"), "http://app.localhost");
    }

    #[test]
    fn public_hosts_still_resolve_to_https() {
        let s = Settings::default();
        assert_eq!(s.resolve_query("example.com"), "https://example.com");
        assert_eq!(s.resolve_query("example.com/a/b"), "https://example.com/a/b");
    }

    #[test]
    fn a_sentence_is_a_search_not_a_host() {
        let s = Settings::default();
        let out = Settings::default().resolve_query("how much is a cat");
        assert!(out.starts_with("http"), "{out}");
        assert!(out.contains("how"), "{out}");
        // and it must not have been mistaken for a hostname
        assert!(!out.starts_with("https://how"), "{out}");
        let _ = s;
    }
}
