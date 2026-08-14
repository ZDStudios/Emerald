// GENERATED FILE — do not edit.
// Source of truth: src-tauri/src/settings.rs. Regenerate with `pnpm schema`.

/** Accent hue. Semantics are fixed regardless of choice: the accent means "active or yours", red means destructive, yellow means needs-attention. */
export type Accent = 'green' | 'teal' | 'sapphire' | 'lavender' | 'mauve' | 'peach' | 'rosewater';
export const AccentValues: readonly Accent[] = ['green', 'teal', 'sapphire', 'lavender', 'mauve', 'peach', 'rosewater'] as const;
export const AccentHelp: Record<Accent, string> = {
  'green': "",
  'teal': "",
  'sapphire': "",
  'lavender': "",
  'mauve': "",
  'peach': "",
  'rosewater': "",
};

/** Autoplay handling. */
export type AutoplayPolicy = 'block_all' | 'block_audio' | 'allow';
export const AutoplayPolicyValues: readonly AutoplayPolicy[] = ['block_all', 'block_audio', 'allow'] as const;
export const AutoplayPolicyHelp: Record<AutoplayPolicy, string> = {
  'block_all': "No `<video>` or `<audio>` starts without a click. The default.",
  'block_audio': "Muted video may autoplay; anything with sound may not.",
  'allow': "Let pages do as they like.",
};

/** Notification badge treatment. */
export type BadgeStyle = 'off' | 'dot' | 'count';
export const BadgeStyleValues: readonly BadgeStyle[] = ['off', 'dot', 'count'] as const;
export const BadgeStyleHelp: Record<BadgeStyle, string> = {
  'off': "No badges at all.",
  'dot': "A 4px dot. No number, no colour change, no motion.",
  'count': "A numeric count.",
};

/** Chrome density. */
export type Density = 'compact' | 'comfortable' | 'roomy';
export const DensityValues: readonly Density[] = ['compact', 'comfortable', 'roomy'] as const;
export const DensityHelp: Record<Density, string> = {
  'compact': "",
  'comfortable': "",
  'roomy': "",
};

/** Draft recovery behaviour. */
export type DraftRestore = 'auto' | 'ask' | 'off';
export const DraftRestoreValues: readonly DraftRestore[] = ['auto', 'ask', 'off'] as const;
export const DraftRestoreHelp: Record<DraftRestore, string> = {
  'auto': "Refill the form immediately and say so in a dismissible bar.",
  'ask': "Offer a \"restore what you were writing\" bar and wait.",
  'off': "Keep saving drafts but never offer them; recover manually from the command palette.",
};

/** How aggressively Emerald removes everything that is not the active tab. */
export type FocusMode = 'off' | 'dim' | 'solo';
export const FocusModeValues: readonly FocusMode[] = ['off', 'dim', 'solo'] as const;
export const FocusModeHelp: Record<FocusMode, string> = {
  'off': "Normal chrome.",
  'dim': "Inactive tabs, sidebar and toolbar affordances fade to `focus_dim_opacity`; they stay clickable.",
  'solo': "One thing at a time: everything except the active tab's content and the address bar is removed from the layout. Tabs remain reachable via the command palette.",
};

/** Body typeface for reader mode and (optionally) all pages. */
export type ReadingFont = 'site_preference' | 'system' | 'atkinson' | 'open_dyslexic' | 'mono';
export const ReadingFontValues: readonly ReadingFont[] = ['site_preference', 'system', 'atkinson', 'open_dyslexic', 'mono'] as const;
export const ReadingFontHelp: Record<ReadingFont, string> = {
  'site_preference': "Leave the page's own font alone.",
  'system': "The OS UI serif/sans stack.",
  'atkinson': "Atkinson Hyperlegible — designed by the Braille Institute for low vision; unusually distinct letterforms. Emerald's default because it helps a wide range of readers without looking unusual.",
  'open_dyslexic': "OpenDyslexic — weighted bottoms to anchor letter orientation. Divisive and heavily debated in the research; offered because plenty of people report it helps them, which is reason enough.",
  'mono': "JetBrains Mono — fixed pitch. Some readers track better on a grid.",
};

/** Ruler colour. */
export type RulerColor = 'yellow' | 'green' | 'blue' | 'pink' | 'auto' | 'neutral';
export const RulerColorValues: readonly RulerColor[] = ['yellow', 'green', 'blue', 'pink', 'auto', 'neutral'] as const;
export const RulerColorHelp: Record<RulerColor, string> = {
  'yellow': "",
  'green': "",
  'blue': "",
  'pink': "",
  'auto': "Follow the theme accent.",
  'neutral': "Neutral grey — no chromatic tint at all.",
};

/** What drives the ruler's position. */
export type RulerFollows = 'pointer' | 'keyboard' | 'both';
export const RulerFollowsValues: readonly RulerFollows[] = ['pointer', 'keyboard', 'both'] as const;
export const RulerFollowsHelp: Record<RulerFollows, string> = {
  'pointer': "The pointer's vertical position.",
  'keyboard': "Up/Down arrows move it a line at a time; the pointer is ignored.",
  'both': "Either input moves it.",
};

/** Reading-line highlight style. */
export type RulerMode = 'off' | 'line' | 'band' | 'spotlight';
export const RulerModeValues: readonly RulerMode[] = ['off', 'line', 'band', 'spotlight'] as const;
export const RulerModeHelp: Record<RulerMode, string> = {
  'off': "No ruler.",
  'line': "A thin underline sitting on the baseline being read.",
  'band': "A translucent band covering the current line.",
  'spotlight': "The current line stays lit; everything above and below is dimmed.",
};

/** Address-bar search provider. */
export type SearchEngine = 'duck_duck_go' | 'startpage' | 'brave' | 'google' | 'custom';
export const SearchEngineValues: readonly SearchEngine[] = ['duck_duck_go', 'startpage', 'brave', 'google', 'custom'] as const;
export const SearchEngineHelp: Record<SearchEngine, string> = {
  'duck_duck_go': "",
  'startpage': "",
  'brave': "",
  'google': "",
  'custom': "Use `custom_search_url`.",
};

/** Colour intensity profile. Hue meaning is constant across all three. */
export type SensoryProfile = 'standard' | 'muted' | 'high_contrast';
export const SensoryProfileValues: readonly SensoryProfile[] = ['standard', 'muted', 'high_contrast'] as const;
export const SensoryProfileHelp: Record<SensoryProfile, string> = {
  'standard': "Catppuccin Mocha as designed.",
  'muted': "Accents desaturated toward the base; surfaces flattened. For when saturated colour is itself the problem.",
  'high_contrast': "Raised text/background contrast beyond WCAG AAA, heavier focus rings.",
};

/** Tab strip placement. */
export type TabLayout = 'sidebar' | 'top' | 'hidden';
export const TabLayoutValues: readonly TabLayout[] = ['sidebar', 'top', 'hidden'] as const;
export const TabLayoutHelp: Record<TabLayout, string> = {
  'sidebar': "Vertical sidebar, Zen/Arc style. Emerald's default: titles stay readable no matter how many tabs are open.",
  'top': "Horizontal strip along the top, Chrome style.",
  'hidden': "No visible tab strip. Tabs are reached through the command palette.",
};

/** Base palette. */
export type Theme = 'mocha' | 'latte' | 'system';
export const ThemeValues: readonly Theme[] = ['mocha', 'latte', 'system'] as const;
export const ThemeHelp: Record<Theme, string> = {
  'mocha': "Catppuccin Mocha. Emerald's designed-for palette.",
  'latte': "Catppuccin Latte, for bright rooms.",
  'system': "Follow the OS.",
};

/** Chrome transition style. */
export type TransitionStyle = 'instant' | 'fade' | 'slide';
export const TransitionStyleValues: readonly TransitionStyle[] = ['instant', 'fade', 'slide'] as const;
export const TransitionStyleHelp: Record<TransitionStyle, string> = {
  'instant': "State changes are immediate. No interpolation of any kind.",
  'fade': "Opacity only. Nothing translates, scales, or bounces.",
  'slide': "Opacity plus a short translation on panels that slide in from an edge.",
};

/** Colour, type and chrome layout. */
export interface Appearance {
  /** Which Catppuccin accent carries "this is active / this is yours". */
  accent: Accent;
  /** Show the favicon strip when the sidebar is collapsed. */
  collapsed_sidebar_favicons: boolean;
  /** Vertical rhythm of the chrome. */
  density: Density;
  /** Where tabs live. */
  tab_layout: TabLayout;
  /** Base palette. Emerald ships Catppuccin Mocha and a light counterpart. */
  theme: Theme;
  /** Chrome text size as a percentage of the 13px base. */
  ui_font_size_pct: number;
}

/** Tab decluttering, focus mode, and motion budget. */
export interface Attention {
  /** Multiplier on every animation duration in the chrome. `0.0` removes animation entirely (transitions become instant, nothing is dropped). `1.0` is the designed speed. */
  animation_speed: number;
  /** Move a tab out of the strip and into the Archive after this many minutes without focus. Archived tabs are one keystroke away in the command palette and are never silently lost. `0` disables. */
  auto_archive_idle_minutes: number;
  /** How much unread state Emerald is willing to show you. */
  badges: BadgeStyle;
  /** Collapse the toolbar to address bar + active tab only. Everything else moves into the command palette. */
  declutter_toolbar: boolean;
  /** Opacity applied to de-emphasised chrome in `FocusMode::Dim`. `0.0` is invisible, `1.0` is unchanged. */
  focus_dim_opacity: number;
  /** What "focus mode" does to everything that is not the active tab. */
  focus_mode: FocusMode;
  /** Hide the tab strip / sidebar entirely while focus mode is on. */
  hide_tabs_in_focus: boolean;
  /** Hard ceiling on how many tabs hold a live web process at once. When a new tab pushes the count past this, the least-recently-used unpinned tab is discarded: its process is terminated and its memory returned to the OS. The tab stays in the strip and reloads on click. */
  max_live_tabs: number;
  /** Also advertise `prefers-reduced-motion: reduce` to every page, so sites that respect it calm down too. */
  reduce_page_motion: boolean;
  /** Show a single unobtrusive count of archived tabs rather than listing them. Off means the archive is silent until you open it. */
  show_archive_count: boolean;
  /** Discard a tab's web process after this many minutes without focus. `0` disables time-based discarding (the `max_live_tabs` cap still applies). This is the only clock-driven policy in Emerald; see `docs/architecture.md` §6 for the one background timer it justifies. */
  suspend_idle_minutes: number;
}

/** The Focus & Access panel. Reachable from anywhere with `Ctrl/Cmd+Shift+A`, or from the command palette in one keystroke plus one selection. */
export interface FocusAccess {
  /** Decluttering, focus mode, motion. Written with ADHD in mind. */
  attention: Attention;
  /** Targets, dictation and never losing typed text. Written with dysgraphia in mind. */
  input: InputAssist;
  /** Low-surprise, predictable UI. Written with autism in mind. */
  predictability: Predictability;
  /** Typography and reading aids. Written with dyslexia in mind. */
  reading: Reading;
}

/** Making text entry survivable: bigger targets, dictation, and never losing what was typed. */
export interface InputAssist {
  /** Keep a local copy of everything typed into web forms, so a crash, a misclick, or a session expiry never costs you the text. */
  autosave_drafts: boolean;
  /** How often a changed field is written to the draft store. */
  autosave_interval_ms: number;
  /** What Emerald does when it finds a draft for a form you have just opened. */
  draft_restore: DraftRestore;
  /** Drafts older than this are deleted. `0` keeps them until you clear them. */
  draft_retention_days: number;
  /** Exclude fields Emerald believes are sensitive (password, one-time code, card number, and anything with `autocomplete="off"`) from the draft store. Strongly recommended; on by default. */
  draft_skip_sensitive: boolean;
  /** Also grow the *visible* size of inputs, textareas and buttons. This one does change page layout, so it is separate from `min_target_px`. */
  enlarge_form_fields: boolean;
  /** Font size inside form fields, as a percentage. Under 100% is not offered — shrinking form text is never the accessible choice. */
  field_font_size_pct: number;
  /** Turn the renderer's spellchecker on everywhere, including fields where the site disabled it. */
  force_spellcheck: boolean;
  /** Never submit a form on plain Enter in a single-line field unless the field is the only one in the form. Stops half-written text being sent. */
  guard_enter_submit: boolean;
  /** Minimum hit area for form controls and links, in CSS pixels. Emerald grows the clickable box without moving the visual box where it can, so pages do not reflow. WCAG 2.2 AA asks for 24; AAA asks for 44. */
  min_target_px: number;
  /** Show a dictation affordance on focused text inputs. Uses the platform speech API through the page's own `SpeechRecognition` where available; Emerald ships no speech model and sends no audio anywhere itself. See `docs/architecture.md` §7 for what this does and does not do. */
  voice_input: boolean;
  /** Keyboard shortcut that starts/stops dictation in the focused field. */
  voice_input_shortcut: string;
}

/** Memory and process behaviour. */
export interface Performance {
  /** Allow pages to use `<link rel=prefetch/preconnect>`. Off means Emerald makes no network request you did not initiate. */
  allow_page_prefetch: boolean;
  /** Let the renderer use the GPU. Turning this off lowers memory on some systems at the cost of scrolling smoothness. */
  hardware_acceleration: boolean;
  /** Soft ceiling on total resident memory across Emerald and all of its web processes, in MiB. When exceeded, tabs are discarded LRU-first until it is back under. `0` disables the check and stops the sampler entirely. */
  memory_budget_mb: number;
  /** Restore tabs from the previous session as *discarded* placeholders rather than loading them. Startup stays flat regardless of how many tabs were open. */
  restore_tabs_discarded: boolean;
  /** How often the memory budget is checked, in seconds. This is the only recurring timer Emerald runs, and it does not start unless `memory_budget_mb > 0` or a time-based tab policy is enabled. */
  sampler_interval_s: number;
}

/** Low-surprise behaviour: nothing moves, plays, or reorders without asking. */
export interface Predictability {
  /** What happens when a page tries to start playing media on load. Emerald defaults to blocking everything and showing an inline play affordance instead. */
  autoplay: AutoplayPolicy;
  /** Ask before anything irreversible: closing a window with more than one tab, clearing history, discarding a recovered draft. */
  confirm_destructive: boolean;
  /** Reserve space for images, iframes and ads before they load, and refuse late-arriving elements the ability to push content that is already on screen. Text you have started reading will not move. */
  no_layout_shift: boolean;
  /** Colour treatment. Affects saturation and contrast only — never hue semantics, so a warning is the same colour family in every profile. */
  sensory_profile: SensoryProfile;
  /** Show a short static notice where blocked media would have played, instead of nothing at all. Off means blocked media is invisible. */
  show_blocked_media_placeholder: boolean;
  /** Never reorder tabs automatically — no "most recently used" shuffling, no promoting a tab because it made noise. New tabs always append. */
  stable_tab_order: boolean;
  /** Open new links in the current tab unless explicitly middle-clicked. Stops pages spawning tabs you did not ask for. */
  suppress_popups: boolean;
  /** How chrome transitions render. `Instant` is honoured even when `animation_speed` is above zero. */
  transitions: TransitionStyle;
}

/** What leaves the machine. The short answer is "nothing". */
export interface Privacy {
  /** Block third-party cookies. */
  block_third_party_cookies: boolean;
  /** Clear cookies and site data for non-pinned tabs at exit. */
  clear_site_data_on_exit: boolean;
  /** Custom search URL with `{q}` as the query placeholder. Used when `search_engine` is `Custom`. */
  custom_search_url: string;
  /** Send `Do Not Track` and `Sec-GPC`. */
  global_privacy_control: boolean;
  /** Search engine used for non-URL input in the address bar. */
  search_engine: SearchEngine;
}

/** Typography, spacing and reading aids. Applies to reader mode always, and to every page when `apply_to_all_pages` is on. */
export interface Reading {
  /** Apply this whole section to ordinary web pages, not just reader mode. Emerald overrides page fonts and spacing with `!important` rules; some icon-font sites will show tofu. Off by default for that reason. */
  apply_to_all_pages: boolean;
  /** Dim everything except the paragraph under the ruler. */
  dim_surrounding_text: boolean;
  /** Typeface used for body text. */
  font: ReadingFont;
  /** Body text size as a percentage of Emerald's 16px base. */
  font_size_pct: number;
  /** Force left-aligned (start-aligned) text. Justified text creates uneven word spacing, which is a known problem for dyslexic readers, so Emerald defaults to overriding it. */
  force_start_align: boolean;
  /** Allow the renderer to hyphenate, which keeps ragged edges shorter without justification's rivers of whitespace. */
  hyphenate: boolean;
  /** Extra space between letters, in `em`. WCAG 1.4.12 asks for at least `0.12` to be *possible*; Emerald allows well past it. */
  letter_spacing_em: number;
  /** Line box height as a multiple of font size. WCAG 1.4.12 floor is `1.5`. */
  line_height: number;
  /** Maximum measure in `ch` units. Long lines are the single biggest tracking problem for many dyslexic readers. */
  max_line_length_ch: number;
  /** Space after each paragraph, in `em`. WCAG 1.4.12 floor is `2.0` times the font size. */
  paragraph_spacing_em: number;
  /** The reading-line highlight. */
  ruler: RulerMode;
  /** Ruler accent. `Auto` derives it from the current theme accent. */
  ruler_color: RulerColor;
  /** What moves the ruler. */
  ruler_follows: RulerFollows;
  /** Height of the ruler band, in CSS pixels. */
  ruler_height_px: number;
  /** Ruler opacity. Kept low by default so it guides rather than shouts. */
  ruler_opacity: number;
  /** Extra space between words, in `em`. WCAG 1.4.12 floor is `0.16`. */
  word_spacing_em: number;
}

/** Emerald's complete persisted configuration. */
export interface Settings {
  /** Colour, type and chrome layout. */
  appearance: Appearance;
  /** The Focus & Access panel: attention, predictability, reading, input. */
  focus_access: FocusAccess;
  /** Memory and process behaviour. */
  performance: Performance;
  /** Data that leaves the machine, and data that stays on it. */
  privacy: Privacy;
  /** Schema version of this file. Written by Emerald; do not edit by hand. */
  version: number;
}

/** Exactly `Settings::default()` from settings.rs. */
export const DEFAULT_SETTINGS: Settings = {
  "appearance": {
    "accent": "green",
    "collapsed_sidebar_favicons": true,
    "density": "comfortable",
    "tab_layout": "sidebar",
    "theme": "mocha",
    "ui_font_size_pct": 100
  },
  "focus_access": {
    "attention": {
      "animation_speed": 1.0,
      "auto_archive_idle_minutes": 0,
      "badges": "dot",
      "declutter_toolbar": false,
      "focus_dim_opacity": 0.3499999940395355,
      "focus_mode": "off",
      "hide_tabs_in_focus": false,
      "max_live_tabs": 6,
      "reduce_page_motion": false,
      "show_archive_count": true,
      "suspend_idle_minutes": 20
    },
    "input": {
      "autosave_drafts": true,
      "autosave_interval_ms": 1500,
      "draft_restore": "ask",
      "draft_retention_days": 14,
      "draft_skip_sensitive": true,
      "enlarge_form_fields": false,
      "field_font_size_pct": 100,
      "force_spellcheck": false,
      "guard_enter_submit": false,
      "min_target_px": 44,
      "voice_input": false,
      "voice_input_shortcut": "CmdOrCtrl+Shift+V"
    },
    "predictability": {
      "autoplay": "block_all",
      "confirm_destructive": true,
      "no_layout_shift": true,
      "sensory_profile": "standard",
      "show_blocked_media_placeholder": true,
      "stable_tab_order": true,
      "suppress_popups": true,
      "transitions": "fade"
    },
    "reading": {
      "apply_to_all_pages": false,
      "dim_surrounding_text": false,
      "font": "atkinson",
      "font_size_pct": 100,
      "force_start_align": true,
      "hyphenate": false,
      "letter_spacing_em": 0.0,
      "line_height": 1.600000023841858,
      "max_line_length_ch": 68,
      "paragraph_spacing_em": 1.0,
      "ruler": "off",
      "ruler_color": "auto",
      "ruler_follows": "pointer",
      "ruler_height_px": 32,
      "ruler_opacity": 0.14000000059604645,
      "word_spacing_em": 0.0
    }
  },
  "performance": {
    "allow_page_prefetch": false,
    "hardware_acceleration": true,
    "memory_budget_mb": 0,
    "restore_tabs_discarded": true,
    "sampler_interval_s": 30
  },
  "privacy": {
    "block_third_party_cookies": true,
    "clear_site_data_on_exit": false,
    "custom_search_url": "",
    "global_privacy_control": true,
    "search_engine": "duck_duck_go"
  },
  "version": 1
};

