//! Builds the content script handed to every page webview.
//!
//! The Focus & Access settings are only real if they reach the page. This
//! module is the boundary: it converts the parts of [`Settings`] that pages
//! need into a small JSON config, and pairs it with the static runtime in
//! `assets/content/emerald.js`.
//!
//! Two rules govern what crosses this boundary:
//!
//! 1. **Only what the page needs.** A page has no business knowing the user's
//!    search engine, tab count, or memory budget, so none of that is sent.
//! 2. **The page is never trusted about identity.** The config carries a tab
//!    id for correlation, but Rust derives the origin and path of every
//!    incoming message from `Webview::url()`, never from the page's claim.
//!    See `commands.rs::page_origin`.

use crate::settings::{
    AutoplayPolicy, DraftRestore, ReadingFont, RulerColor, RulerFollows, RulerMode, Settings,
};
use crate::tabs::TabId;
use serde::Serialize;

/// The static page runtime. One file, no bundler, no dependencies — it has to
/// run before the page's own scripts, so it must be small and synchronous.
pub const RUNTIME_JS: &str = include_str!("../../assets/content/emerald.js");

/// Everything a page webview is told about the user's settings.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PageConfig {
    pub tab_id: TabId,
    pub reading: PageReading,
    pub input: PageInput,
    pub calm: PageCalm,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PageReading {
    /// CSS `font-family` list, already resolved. Empty means "leave the page
    /// alone".
    pub font_stack: String,
    pub apply_to_all_pages: bool,
    pub letter_spacing_em: f32,
    pub word_spacing_em: f32,
    pub line_height: f32,
    pub paragraph_spacing_em: f32,
    pub font_size_pct: u16,
    pub max_line_length_ch: u16,
    pub ruler: RulerMode,
    pub ruler_height_px: u16,
    pub ruler_opacity: f32,
    /// Resolved to a concrete CSS colour so the page never has to know about
    /// Emerald's palette.
    pub ruler_css_color: String,
    pub ruler_follows: RulerFollows,
    pub hyphenate: bool,
    pub force_start_align: bool,
    pub dim_surrounding_text: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PageInput {
    pub min_target_px: u16,
    pub enlarge_form_fields: bool,
    pub field_font_size_pct: u16,
    pub voice_input: bool,
    pub autosave_drafts: bool,
    pub autosave_interval_ms: u32,
    pub draft_restore: DraftRestore,
    pub draft_skip_sensitive: bool,
    pub force_spellcheck: bool,
    pub guard_enter_submit: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PageCalm {
    pub autoplay: AutoplayPolicy,
    pub no_layout_shift: bool,
    pub show_blocked_media_placeholder: bool,
    pub reduce_motion: bool,
    pub suppress_popups: bool,
}

/// Resolve a [`ReadingFont`] to a CSS font stack.
///
/// `local()` comes first on purpose. Emerald installs OpenDyslexic and Atkinson
/// Hyperlegible as system fonts (see `docs/architecture.md` §7), because a page's
/// own Content-Security-Policy can forbid `font-src` from anything we inject —
/// a locally installed family sidesteps CSP entirely. The web-font fallback
/// exists only for reader mode on our own about: pages.
pub fn font_stack(font: ReadingFont) -> String {
    match font {
        ReadingFont::SitePreference => String::new(),
        ReadingFont::System => "system-ui, -apple-system, 'Segoe UI', sans-serif".into(),
        ReadingFont::Atkinson => {
            "'Atkinson Hyperlegible Next', 'Atkinson Hyperlegible', system-ui, sans-serif".into()
        }
        ReadingFont::OpenDyslexic => {
            "'OpenDyslexic', 'Open Dyslexic', 'Atkinson Hyperlegible', system-ui, sans-serif".into()
        }
        ReadingFont::Mono => "'JetBrains Mono', ui-monospace, monospace".into(),
    }
}

fn ruler_css_color(color: RulerColor, accent_hex: &str) -> String {
    // Catppuccin Mocha values, kept in step with src/styles/tokens.css.
    match color {
        RulerColor::Auto => accent_hex.to_string(),
        RulerColor::Yellow => "#f9e2af".into(),
        RulerColor::Green => "#a6e3a1".into(),
        RulerColor::Blue => "#89b4fa".into(),
        RulerColor::Pink => "#f5c2e7".into(),
        RulerColor::Neutral => "#9399b2".into(),
    }
}

/// Hex for the user's chosen accent, from the Mocha palette.
pub fn accent_hex(accent: crate::settings::Accent) -> &'static str {
    use crate::settings::Accent::*;
    match accent {
        Green => "#a6e3a1",
        Teal => "#94e2d5",
        Sapphire => "#74c7ec",
        Lavender => "#b4befe",
        Mauve => "#cba6f7",
        Peach => "#fab387",
        Rosewater => "#f5e0dc",
    }
}

impl PageConfig {
    pub fn from_settings(settings: &Settings, tab_id: TabId) -> Self {
        let r = &settings.focus_access.reading;
        let i = &settings.focus_access.input;
        let p = &settings.focus_access.predictability;
        let a = &settings.focus_access.attention;
        let accent = accent_hex(settings.appearance.accent);

        Self {
            tab_id,
            reading: PageReading {
                font_stack: font_stack(r.font),
                apply_to_all_pages: r.apply_to_all_pages,
                letter_spacing_em: r.letter_spacing_em,
                word_spacing_em: r.word_spacing_em,
                line_height: r.line_height,
                paragraph_spacing_em: r.paragraph_spacing_em,
                font_size_pct: r.font_size_pct,
                max_line_length_ch: r.max_line_length_ch,
                ruler: r.ruler,
                ruler_height_px: r.ruler_height_px,
                ruler_opacity: r.ruler_opacity,
                ruler_css_color: ruler_css_color(r.ruler_color, accent),
                ruler_follows: r.ruler_follows,
                hyphenate: r.hyphenate,
                force_start_align: r.force_start_align,
                dim_surrounding_text: r.dim_surrounding_text,
            },
            input: PageInput {
                min_target_px: i.min_target_px,
                enlarge_form_fields: i.enlarge_form_fields,
                field_font_size_pct: i.field_font_size_pct,
                voice_input: i.voice_input,
                autosave_drafts: i.autosave_drafts,
                autosave_interval_ms: i.autosave_interval_ms,
                draft_restore: i.draft_restore,
                draft_skip_sensitive: i.draft_skip_sensitive,
                force_spellcheck: i.force_spellcheck,
                guard_enter_submit: i.guard_enter_submit,
            },
            calm: PageCalm {
                autoplay: p.autoplay,
                no_layout_shift: p.no_layout_shift,
                show_blocked_media_placeholder: p.show_blocked_media_placeholder,
                reduce_motion: a.reduce_page_motion,
                suppress_popups: p.suppress_popups,
            },
        }
    }

    fn json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }
}

/// The full initialization script for a new page webview.
///
/// Runs before page scripts, so the autoplay and layout-shift guards are in
/// place before any of the page's own code can start media or insert content.
pub fn init_script(settings: &Settings, tab_id: TabId) -> String {
    format!(
        "globalThis.__EMERALD_CONFIG__ = {};\n{}",
        PageConfig::from_settings(settings, tab_id).json(),
        RUNTIME_JS
    )
}

/// A live settings update for a page that is already loaded.
///
/// Applying settings without a reload is the whole point of "reachable in
/// under two clicks": you change line spacing and the page you are reading
/// reflows underneath the panel.
pub fn update_script(settings: &Settings, tab_id: TabId) -> String {
    format!(
        "globalThis.__emerald && globalThis.__emerald.apply({});",
        PageConfig::from_settings(settings, tab_id).json()
    )
}

/// Restore a scroll offset after a discarded tab is brought back.
pub fn restore_scroll_script(y: f64) -> String {
    // `instant` rather than `smooth`: this is a restoration, not a motion.
    format!(
        "globalThis.__emerald && globalThis.__emerald.restoreScroll({y});",
        y = y.max(0.0)
    )
}

/// Toggle reader mode in an already-loaded page.
pub fn reader_script(on: bool) -> String {
    format!("globalThis.__emerald && globalThis.__emerald.reader({on});")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Accent, Settings};

    #[test]
    fn page_config_carries_no_private_settings() {
        let mut s = Settings::default();
        s.privacy.custom_search_url = "https://secret.example/?q={q}".into();
        s.performance.memory_budget_mb = 4096;

        let json = PageConfig::from_settings(&s, 1).json();
        assert!(!json.contains("secret.example"), "search config must not reach pages");
        assert!(!json.contains("memory_budget"), "perf config must not reach pages");
        assert!(!json.contains("4096"));
    }

    #[test]
    fn init_script_defines_config_before_the_runtime_reads_it() {
        let s = Settings::default();
        let script = init_script(&s, 7);
        let cfg_at = script.find("__EMERALD_CONFIG__").unwrap();
        let runtime_at = script.find("__emerald").unwrap();
        assert!(cfg_at < runtime_at, "config must be assigned first");
        assert!(script.contains("\"tab_id\":7"));
    }

    #[test]
    fn font_stack_prefers_locally_installed_families() {
        let stack = font_stack(ReadingFont::OpenDyslexic);
        assert!(stack.starts_with("'OpenDyslexic'"));
        // Always ends in a generic so text renders even with nothing installed.
        assert!(stack.ends_with("sans-serif"));
        assert!(font_stack(ReadingFont::SitePreference).is_empty());
    }

    #[test]
    fn auto_ruler_colour_follows_the_chosen_accent() {
        let mut s = Settings::default();
        s.appearance.accent = Accent::Mauve;
        s.focus_access.reading.ruler_color = RulerColor::Auto;
        let cfg = PageConfig::from_settings(&s, 1);
        assert_eq!(cfg.reading.ruler_css_color, accent_hex(Accent::Mauve));

        s.focus_access.reading.ruler_color = RulerColor::Neutral;
        let cfg = PageConfig::from_settings(&s, 1);
        assert_eq!(cfg.reading.ruler_css_color, "#9399b2");
    }

    #[test]
    fn update_script_is_defensive_about_a_page_without_the_runtime() {
        // A page that blocked our script (CSP) must not throw on every update.
        let js = update_script(&Settings::default(), 1);
        assert!(js.starts_with("globalThis.__emerald && "));
    }

    #[test]
    fn defaults_block_autoplay_and_hold_layout_still() {
        let cfg = PageConfig::from_settings(&Settings::default(), 1);
        assert_eq!(cfg.calm.autoplay, AutoplayPolicy::BlockAll);
        assert!(cfg.calm.no_layout_shift);
        assert!(cfg.input.autosave_drafts, "drafts on by default");
        assert!(cfg.input.draft_skip_sensitive, "and passwords excluded");
    }
}
