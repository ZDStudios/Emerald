//! The IPC surface.
//!
//! ## Two tiers, enforced twice
//!
//! Emerald renders untrusted pages as sibling webviews of its own UI, so every
//! command states who may call it and that is checked in two independent
//! places:
//!
//! 1. **Tauri's ACL** (`capabilities/chrome.json`, `capabilities/pages.json`)
//!    rejects the call before it reaches Rust.
//! 2. **[`guard_chrome`]** re-checks the calling webview's label inside the
//!    command body.
//!
//! Belt and braces, because the cost of getting this wrong is a web page
//! driving the browser. The label is not something a page can influence: it
//! comes from the `Webview` handle Tauri injects, not from the payload.
//!
//! Page-callable commands never take an origin as an argument. `page_origin`
//! derives it from `Webview::url()`, so `evil.example` cannot read or clobber
//! `bank.example`'s drafts by claiming to be it.

use crate::extensions::{self, InstalledExtension};
use crate::inject;
use crate::metrics::{self, MemorySample};
use crate::runtime::{
    self, apply_effects, now_ms, push_settings_to_pages, push_state, relayout, tab_label, Core,
};
use crate::settings::Settings;
use crate::store::{Bookmark, Draft, Space, SpaceId};
use crate::tabs::{Insets, Pane, Tab, TabId, TabState};
use serde::Serialize;
use tauri::{Emitter, Manager, State, Webview};

type Res<T> = Result<T, String>;

/// Reject anything that is not Emerald's own UI.
fn guard_chrome(webview: &Webview) -> Res<()> {
    if webview.label() == runtime::CHROME {
        Ok(())
    } else {
        // Deliberately terse. A page probing the IPC surface learns nothing.
        Err("not permitted".into())
    }
}

/// Origin and path of the page making the call, derived from the webview
/// itself. Never from the payload.
fn page_origin(webview: &Webview) -> Res<(String, String)> {
    if !webview.label().starts_with("tab:") {
        return Err("not a page".into());
    }
    let url = webview.url().map_err(|e| e.to_string())?;
    let origin = match (url.scheme(), url.host_str()) {
        (scheme, Some(host)) => match url.port() {
            Some(port) => format!("{scheme}://{host}:{port}"),
            None => format!("{scheme}://{host}"),
        },
        (scheme, None) => scheme.to_string(),
    };
    Ok((origin, url.path().to_string()))
}

/// The tab id encoded in a page webview's label.
fn page_tab_id(webview: &Webview) -> Res<TabId> {
    webview
        .label()
        .strip_prefix("tab:")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "not a page".into())
}

// ---------------------------------------------------------------------------
// State snapshot
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct StateSnapshot {
    pub settings: Settings,
    pub tabs: Vec<Tab>,
    pub active: Option<TabId>,
    pub panes: Vec<Pane>,
    pub spaces: Vec<Space>,
    pub active_space: SpaceId,
    pub bookmarks: Vec<Bookmark>,
    pub live_count: usize,
    pub archived_count: usize,
    pub draft_count: usize,
}

pub fn snapshot(core: &Core) -> StateSnapshot {
    let settings = core.settings.read().clone();
    let tabs = core.tabs.lock();
    let store = core.store.lock();
    StateSnapshot {
        settings,
        tabs: tabs.all().to_vec(),
        active: tabs.active(),
        panes: tabs.panes().to_vec(),
        spaces: store.spaces.clone(),
        active_space: tabs.active_space,
        bookmarks: store.bookmarks.clone(),
        live_count: tabs.live_count(),
        archived_count: tabs
            .all()
            .iter()
            .filter(|t| t.state == TabState::Archived)
            .count(),
        draft_count: store.drafts.len(),
    }
}

// ---------------------------------------------------------------------------
// Chrome commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_state(webview: Webview, core: State<Core>) -> Res<StateSnapshot> {
    guard_chrome(&webview)?;
    Ok(snapshot(&core))
}

#[tauri::command]
pub fn set_settings(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    settings: Settings,
) -> Res<StateSnapshot> {
    guard_chrome(&webview)?;
    let mut next = settings;
    next.clamp();
    *core.settings.write() = next;
    core.sync_policy();

    // The cap may now be lower than the number of live tabs.
    let effects = core.tabs.lock().enforce_live_cap();
    if !effects.is_empty() {
        apply_effects(&app, effects);
    }
    // Live-update open pages so a spacing change reflows what you are reading
    // rather than waiting for a reload.
    push_settings_to_pages(&app);
    relayout(&app);
    core.poke_sampler();
    core.flush();
    Ok(snapshot(&core))
}

#[tauri::command]
pub fn reset_settings(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
) -> Res<StateSnapshot> {
    guard_chrome(&webview)?;
    *core.settings.write() = Settings::default();
    core.sync_policy();
    push_settings_to_pages(&app);
    core.poke_sampler();
    core.flush();
    Ok(snapshot(&core))
}

#[tauri::command]
pub fn open_tab(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    url: String,
    space: Option<SpaceId>,
) -> Res<TabId> {
    guard_chrome(&webview)?;
    let resolved = core.settings.read().resolve_query(&url);
    let space = space.unwrap_or_else(|| core.tabs.lock().active_space);
    let (id, effects) = core.tabs.lock().open(resolved, space, now_ms());
    apply_effects(&app, effects);
    push_state(&app);
    Ok(id)
}

#[tauri::command]
pub fn activate_tab(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: TabId,
) -> Res<()> {
    guard_chrome(&webview)?;
    let effects = core.tabs.lock().activate(id, now_ms());
    apply_effects(&app, effects);
    push_state(&app);
    Ok(())
}

#[tauri::command]
pub fn close_tab(webview: Webview, app: tauri::AppHandle, core: State<Core>, id: TabId) -> Res<()> {
    guard_chrome(&webview)?;
    let effects = core.tabs.lock().close(id);
    apply_effects(&app, effects);
    core.checkpoint_session();
    push_state(&app);
    Ok(())
}

#[tauri::command]
pub fn discard_tab(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: TabId,
) -> Res<()> {
    guard_chrome(&webview)?;
    let effects = core.tabs.lock().discard(id);
    apply_effects(&app, effects);
    push_state(&app);
    Ok(())
}

#[tauri::command]
pub fn archive_tab(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: TabId,
) -> Res<()> {
    guard_chrome(&webview)?;
    let effects = core.tabs.lock().archive(id);
    apply_effects(&app, effects);
    push_state(&app);
    Ok(())
}

/// Discard every tab that is neither pinned nor on screen. The manual version
/// of the idle policy, for when you want the desk cleared now.
#[tauri::command]
pub fn discard_all_idle(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
) -> Res<usize> {
    guard_chrome(&webview)?;
    let (effects, freed) = core.tabs.lock().relieve_pressure(usize::MAX >> 1);
    apply_effects(&app, effects);
    push_state(&app);
    Ok(freed)
}

#[tauri::command]
pub fn navigate_tab(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: TabId,
    url: String,
) -> Res<()> {
    guard_chrome(&webview)?;
    let resolved = core.settings.read().resolve_query(&url);
    let effects = core.tabs.lock().navigate(id, resolved);
    apply_effects(&app, effects);
    push_state(&app);
    Ok(())
}

#[tauri::command]
pub fn reload_tab(webview: Webview, app: tauri::AppHandle, id: TabId) -> Res<()> {
    guard_chrome(&webview)?;
    if let Some(wv) = app.get_webview(&tab_label(id)) {
        wv.reload().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn history_back(webview: Webview, app: tauri::AppHandle, id: TabId) -> Res<()> {
    guard_chrome(&webview)?;
    // wry exposes no back/forward API; the page's own history object is the
    // portable route and behaves identically.
    if let Some(wv) = app.get_webview(&tab_label(id)) {
        let _ = wv.eval("history.back()");
    }
    Ok(())
}

#[tauri::command]
pub fn history_forward(webview: Webview, app: tauri::AppHandle, id: TabId) -> Res<()> {
    guard_chrome(&webview)?;
    if let Some(wv) = app.get_webview(&tab_label(id)) {
        let _ = wv.eval("history.forward()");
    }
    Ok(())
}

#[tauri::command]
pub fn set_pinned(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: TabId,
    pinned: bool,
) -> Res<()> {
    guard_chrome(&webview)?;
    core.tabs.lock().set_pinned(id, pinned);
    push_state(&app);
    Ok(())
}

#[tauri::command]
pub fn reorder_tab(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: TabId,
    to: usize,
) -> Res<bool> {
    guard_chrome(&webview)?;
    // Always user-initiated: this command only exists for drags.
    let moved = core.tabs.lock().reorder(id, to, true);
    push_state(&app);
    Ok(moved)
}

#[tauri::command]
pub fn set_insets(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    insets: Insets,
) -> Res<()> {
    guard_chrome(&webview)?;
    let effects = core.tabs.lock().set_insets(insets);
    apply_effects(&app, effects);
    Ok(())
}

/// Tell the core that the chrome is showing a full-surface overlay.
///
/// Necessary because page webviews are native widgets layered above the chrome
/// webview: without this the command palette and the settings panel open
/// correctly, respond to input, and are invisible behind the page.
#[tauri::command]
pub fn set_overlay(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    active: bool,
) -> Res<()> {
    guard_chrome(&webview)?;
    if core.set_overlay(active) {
        relayout(&app);
    }
    Ok(())
}

#[tauri::command]
pub fn set_panes(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    panes: Vec<Pane>,
) -> Res<()> {
    guard_chrome(&webview)?;
    let effects = core.tabs.lock().set_panes(panes, now_ms());
    apply_effects(&app, effects);
    push_state(&app);
    Ok(())
}

#[tauri::command]
pub fn split_with(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: TabId,
) -> Res<()> {
    guard_chrome(&webview)?;
    let effects = core.tabs.lock().split_with(id, now_ms());
    apply_effects(&app, effects);
    push_state(&app);
    Ok(())
}

#[tauri::command]
pub fn unsplit(webview: Webview, app: tauri::AppHandle, core: State<Core>) -> Res<()> {
    guard_chrome(&webview)?;
    let effects = core.tabs.lock().unsplit();
    apply_effects(&app, effects);
    push_state(&app);
    Ok(())
}

#[tauri::command]
pub fn toggle_reader(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: TabId,
) -> Res<bool> {
    guard_chrome(&webview)?;
    let on = {
        let mut tabs = core.tabs.lock();
        match tabs.get_mut(id) {
            Some(tab) => {
                tab.reader = !tab.reader;
                tab.reader
            }
            None => return Err("no such tab".into()),
        }
    };
    if let Some(wv) = app.get_webview(&tab_label(id)) {
        let _ = wv.eval(inject::reader_script(on));
    }
    push_state(&app);
    Ok(on)
}

#[tauri::command]
pub fn set_zoom(webview: Webview, app: tauri::AppHandle, id: TabId, factor: f64) -> Res<()> {
    guard_chrome(&webview)?;
    if let Some(wv) = app.get_webview(&tab_label(id)) {
        wv.set_zoom(factor.clamp(0.25, 5.0)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// --- spaces ----------------------------------------------------------------

#[tauri::command]
pub fn add_space(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    name: String,
    icon: String,
    accent: String,
) -> Res<SpaceId> {
    guard_chrome(&webview)?;
    let id = core.store.lock().add_space(name, icon, accent, now_ms());
    core.flush();
    push_state(&app);
    Ok(id)
}

#[tauri::command]
pub fn remove_space(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: SpaceId,
) -> Res<bool> {
    guard_chrome(&webview)?;
    let removed = core.store.lock().remove_space(id);
    if removed {
        // Tabs are never destroyed with their space; they move home so nothing
        // disappears without being asked for.
        let mut tabs = core.tabs.lock();
        let orphans: Vec<TabId> = tabs
            .all()
            .iter()
            .filter(|t| t.space == id)
            .map(|t| t.id)
            .collect();
        for tab_id in orphans {
            if let Some(tab) = tabs.get_mut(tab_id) {
                tab.space = 0;
            }
        }
        if tabs.active_space == id {
            tabs.active_space = 0;
        }
    }
    core.flush();
    push_state(&app);
    Ok(removed)
}

#[tauri::command]
pub fn switch_space(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: SpaceId,
) -> Res<()> {
    guard_chrome(&webview)?;
    let target = {
        let mut tabs = core.tabs.lock();
        tabs.active_space = id;
        // Focus the most recently used non-archived tab in the space.
        tabs.all()
            .iter()
            .filter(|t| t.space == id && t.state != TabState::Archived)
            .max_by_key(|t| t.last_active_ms)
            .map(|t| t.id)
    };
    let effects = match target {
        Some(tab_id) => core.tabs.lock().activate(tab_id, now_ms()),
        None => core.tabs.lock().open("about:blank".into(), id, now_ms()).1,
    };
    apply_effects(&app, effects);
    push_state(&app);
    Ok(())
}

// --- bookmarks / history ---------------------------------------------------

#[tauri::command]
pub fn add_bookmark(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    title: String,
    url: String,
) -> Res<u32> {
    guard_chrome(&webview)?;
    let space = core.tabs.lock().active_space;
    let id = core.store.lock().add_bookmark(title, url, space, now_ms());
    core.flush();
    push_state(&app);
    Ok(id)
}

#[tauri::command]
pub fn remove_bookmark(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: u32,
) -> Res<bool> {
    guard_chrome(&webview)?;
    let removed = core.store.lock().remove_bookmark(id);
    core.flush();
    push_state(&app);
    Ok(removed)
}

#[tauri::command]
pub fn clear_history(webview: Webview, core: State<Core>) -> Res<()> {
    guard_chrome(&webview)?;
    core.store.lock().clear_history();
    core.flush();
    Ok(())
}

#[tauri::command]
pub fn resolve_query(webview: Webview, core: State<Core>, input: String) -> Res<String> {
    guard_chrome(&webview)?;
    Ok(core.settings.read().resolve_query(&input))
}

// --- command palette -------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    /// `tab` | `archived` | `bookmark` | `history` | `draft` | `setting` | `action`
    pub kind: &'static str,
    pub title: String,
    pub subtitle: String,
    /// What the chrome should do when this is chosen.
    pub action: &'static str,
    /// Argument for the action: a tab id, a URL, or a settings anchor.
    pub arg: String,
    pub score: i64,
}

/// Static palette entries: settings sections and browser actions.
/// Kept here rather than in the chrome so search stays one implementation.
const PALETTE: &[(&str, &str, &str, &str)] = &[
    // (title, subtitle, action, arg)
    ("Focus & Access", "Attention, predictability, reading, input", "settings", "focus-access"),
    ("Focus mode", "One thing at a time", "settings", "attention"),
    ("Tab limit and idle discarding", "Attention", "settings", "attention"),
    ("Animation speed", "Attention", "settings", "attention"),
    ("Autoplay and layout shift", "Predictability", "settings", "predictability"),
    ("Reading font and spacing", "Reading", "settings", "reading"),
    ("Reading ruler", "Reading", "settings", "reading"),
    ("Dyslexia-friendly font", "Reading", "settings", "reading"),
    ("Click target size", "Input", "settings", "input"),
    ("Form draft recovery", "Input", "settings", "input"),
    ("Dictation", "Input", "settings", "input"),
    ("Theme and accent", "Appearance", "settings", "appearance"),
    ("Memory budget", "Performance", "settings", "performance"),
    ("Search engine", "Privacy", "settings", "privacy"),
    ("New tab", "Open a blank tab", "action", "new-tab"),
    ("Split view", "Put two tabs side by side", "action", "split"),
    ("Reader mode", "Reformat this page for reading", "action", "reader"),
    ("Discard idle tabs now", "Free memory immediately", "action", "discard-idle"),
    ("Archive", "Tabs moved out of the way", "action", "archive"),
    ("Recovered drafts", "Text saved from web forms", "action", "drafts"),
];

/// Rank a query against a candidate. Prefix beats word-start beats substring.
fn score_of(query: &str, text: &str) -> Option<i64> {
    let hay = text.to_lowercase();
    let pos = hay.find(query)?;
    let mut score = 100 - (pos as i64).min(60);
    if pos == 0 {
        score += 60;
    } else if hay.as_bytes().get(pos - 1) == Some(&b' ') {
        score += 30;
    }
    score -= (hay.len() as i64 - query.len() as i64).min(40) / 4;
    Some(score)
}

#[tauri::command]
pub fn search_all(
    webview: Webview,
    core: State<Core>,
    query: String,
    limit: Option<usize>,
) -> Res<Vec<SearchHit>> {
    guard_chrome(&webview)?;
    let q = query.trim().to_lowercase();
    let limit = limit.unwrap_or(24);
    let mut hits: Vec<SearchHit> = Vec::new();

    let tabs = core.tabs.lock();
    let store = core.store.lock();

    // Open and archived tabs first: switching to something already open is the
    // most common thing a quick-switcher is for.
    for tab in tabs.all() {
        let hay = format!("{} {}", tab.display_title(), tab.url);
        let score = if q.is_empty() { 10 } else { score_of(&q, &hay).unwrap_or(i64::MIN) };
        if score == i64::MIN {
            continue;
        }
        let archived = tab.state == TabState::Archived;
        hits.push(SearchHit {
            kind: if archived { "archived" } else { "tab" },
            title: tab.display_title().to_string(),
            subtitle: match tab.state {
                TabState::Live => tab.url.clone(),
                TabState::Discarded => format!("{}  · asleep", tab.url),
                TabState::Archived => format!("{}  · archived", tab.url),
            },
            action: "activate",
            arg: tab.id.to_string(),
            score: score + if archived { 0 } else { 220 },
        });
    }

    for (title, subtitle, action, arg) in PALETTE {
        let hay = format!("{title} {subtitle}");
        let score = if q.is_empty() { 5 } else { score_of(&q, &hay).unwrap_or(i64::MIN) };
        if score == i64::MIN {
            continue;
        }
        hits.push(SearchHit {
            kind: if *action == "settings" { "setting" } else { "action" },
            title: (*title).into(),
            subtitle: (*subtitle).into(),
            action: if *action == "settings" { "settings" } else { "action" },
            arg: (*arg).into(),
            score: score + 120,
        });
    }

    for bm in &store.bookmarks {
        let hay = format!("{} {}", bm.title, bm.url);
        let Some(score) = (if q.is_empty() { Some(0) } else { score_of(&q, &hay) }) else {
            continue;
        };
        hits.push(SearchHit {
            kind: "bookmark",
            title: bm.title.clone(),
            subtitle: bm.url.clone(),
            action: "open",
            arg: bm.url.clone(),
            score: score + 90,
        });
    }

    if !q.is_empty() {
        for entry in store.search_history(&q, 12, now_ms()) {
            hits.push(SearchHit {
                kind: "history",
                title: if entry.title.is_empty() { entry.url.clone() } else { entry.title.clone() },
                subtitle: entry.url.clone(),
                action: "open",
                arg: entry.url.clone(),
                score: 40 + entry.visits.min(20) as i64,
            });
        }
    }

    for draft in store.recent_drafts(8) {
        let hay = format!("{} {} {}", draft.label, draft.origin, draft.value);
        let Some(score) = (if q.is_empty() { Some(0) } else { score_of(&q, &hay) }) else {
            continue;
        };
        hits.push(SearchHit {
            kind: "draft",
            title: format!("{} — {}", draft.label, truncate(&draft.value, 60)),
            subtitle: format!("{}{}", draft.origin, draft.path),
            action: "open",
            arg: format!("{}{}", draft.origin, draft.path),
            score: score + 30,
        });
    }

    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.title.cmp(&b.title)));
    hits.truncate(limit);
    Ok(hits)
}

fn truncate(s: &str, n: usize) -> String {
    let cleaned = s.replace(['\n', '\r'], " ");
    if cleaned.chars().count() <= n {
        return cleaned;
    }
    let mut out: String = cleaned.chars().take(n.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[tauri::command]
pub fn recent_drafts(webview: Webview, core: State<Core>, limit: Option<usize>) -> Res<Vec<Draft>> {
    guard_chrome(&webview)?;
    let store = core.store.lock();
    Ok(store
        .recent_drafts(limit.unwrap_or(50))
        .into_iter()
        .cloned()
        .collect())
}

// ---------------------------------------------------------------------------
// Extensions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionState {
    /// False on macOS and Linux, where the engine cannot run Chrome
    /// extensions at all. The panel uses this to explain rather than to
    /// present a control that would do nothing.
    pub supported: bool,
    /// The engine name, so the explanation can be specific.
    pub engine: &'static str,
    pub directory: String,
    pub installed: Vec<InstalledExtension>,
}

fn engine_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "WebView2"
    } else if cfg!(target_os = "macos") {
        "WKWebView"
    } else {
        "WebKitGTK"
    }
}

fn extensions_dir(core: &Core) -> std::path::PathBuf {
    core.settings.read().extensions.dir(&core.config_dir)
}

#[tauri::command]
pub fn list_extensions(webview: Webview, core: State<Core>) -> Res<ExtensionState> {
    guard_chrome(&webview)?;
    let dir = extensions_dir(&core);
    let disabled = core.settings.read().extensions.disabled.clone();
    Ok(ExtensionState {
        supported: crate::settings::Extensions::supported_here(),
        engine: engine_name(),
        directory: dir.to_string_lossy().to_string(),
        installed: extensions::list(&dir, &disabled),
    })
}

/// Install a `.crx` or `.zip` the user already has on disk.
///
/// Emerald does not fetch from the Chrome Web Store; see `extensions.rs` for
/// why. The path comes from the chrome's file picker, and the command is
/// chrome-only, so a page cannot point this at anything.
#[tauri::command]
pub fn install_extension(
    webview: Webview,
    core: State<Core>,
    path: String,
) -> Res<InstalledExtension> {
    guard_chrome(&webview)?;
    let dir = extensions_dir(&core);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create extensions folder: {e}"))?;
    extensions::install_archive(std::path::Path::new(&path), &dir)
}

#[tauri::command]
pub fn set_extension_enabled(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: String,
    enabled: bool,
) -> Res<()> {
    guard_chrome(&webview)?;
    {
        let mut settings = core.settings.write();
        settings.extensions.disabled.retain(|d| d != &id);
        if !enabled {
            settings.extensions.disabled.push(id);
        }
    }
    core.flush();
    push_state(&app);
    Ok(())
}

#[tauri::command]
pub fn remove_extension(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    id: String,
) -> Res<()> {
    guard_chrome(&webview)?;
    let dir = extensions_dir(&core);
    extensions::remove(&dir, &id)?;
    {
        let mut settings = core.settings.write();
        settings.extensions.disabled.retain(|d| d != &id);
    }
    core.flush();
    push_state(&app);
    Ok(())
}

#[tauri::command]
pub fn memory_sample(webview: Webview) -> Res<MemorySample> {
    guard_chrome(&webview)?;
    Ok(metrics::sample())
}

// ---------------------------------------------------------------------------
// Page commands — callable by rendered web content
// ---------------------------------------------------------------------------
//
// These five are the entire surface a web page can reach. All of them are
// scoped by an origin the page does not get to choose.

#[tauri::command]
pub fn page_draft(
    webview: Webview,
    core: State<Core>,
    field: String,
    label: String,
    value: String,
) -> Res<()> {
    let (origin, path) = page_origin(&webview)?;
    if !core.settings.read().focus_access.input.autosave_drafts {
        return Ok(());
    }
    // Bound what a page can write. A draft is a form field, not a data store;
    // 256KB is far beyond any real answer box and stops a hostile page filling
    // the disk through the autosave path.
    const MAX_DRAFT_BYTES: usize = 256 * 1024;
    if value.len() > MAX_DRAFT_BYTES {
        return Err("draft too large".into());
    }
    core.store.lock().put_draft(Draft {
        key: format!("{origin}{path}#{field}"),
        origin,
        path,
        label: truncate(&label, 80),
        value,
        updated_ms: now_ms(),
    });
    Ok(())
}

#[tauri::command]
pub fn page_drafts(webview: Webview, core: State<Core>) -> Res<Vec<Draft>> {
    let (origin, path) = page_origin(&webview)?;
    let store = core.store.lock();
    // The page is handed back only what it saved on this exact origin+path.
    Ok(store
        .drafts_for(&origin, &path)
        .into_iter()
        .map(|d| Draft {
            // Strip the origin prefix so the page sees the field key it sent.
            key: d.key.rsplit('#').next().unwrap_or(&d.key).to_string(),
            ..d.clone()
        })
        .collect())
}

#[tauri::command]
pub fn page_clear_drafts(webview: Webview, core: State<Core>) -> Res<usize> {
    let (origin, path) = page_origin(&webview)?;
    let removed = core.store.lock().drop_drafts_for(&origin, &path);
    Ok(removed)
}

/// Chords Emerald owns. A page may ask for one of these and nothing else.
///
/// Validated against a fixed list rather than forwarded blindly: the chrome
/// acts on whatever arrives, so an unchecked string would let a page drive the
/// browser's keyboard surface.
const OWNED_SHORTCUTS: &[&str] = &[
    "mod+k", "mod+p", "mod+t", "mod+w", "mod+l", "mod+r", "mod+shift+r", "mod+shift+a", "mod+e",
    "mod+\\", "mod+,", "mod+1", "mod+2", "mod+3", "mod+4", "mod+5", "mod+6", "mod+7", "mod+8",
    "mod+9",
];

/// Relay a browser shortcut pressed while a page had keyboard focus.
///
/// Necessary because the chrome is a *sibling* webview, not an ancestor: a
/// keydown in a page is never seen by the chrome's own listener. Without this
/// the command palette would only open when focus happened to be in the
/// sidebar. See `assets/content/emerald.js`.
#[tauri::command]
pub fn page_shortcut(webview: Webview, app: tauri::AppHandle, combo: String) -> Res<()> {
    page_tab_id(&webview)?;
    if !OWNED_SHORTCUTS.contains(&combo.as_str()) {
        return Err("unknown shortcut".into());
    }
    app.emit_to(runtime::CHROME, "emerald://shortcut", combo)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn page_scroll(webview: Webview, core: State<Core>, y: f64) -> Res<()> {
    let id = page_tab_id(&webview)?;
    if let Some(tab) = core.tabs.lock().get_mut(id) {
        tab.scroll_y = y.max(0.0);
    }
    Ok(())
}

#[tauri::command]
pub fn page_favicon(
    webview: Webview,
    app: tauri::AppHandle,
    core: State<Core>,
    url: String,
) -> Res<()> {
    let id = page_tab_id(&webview)?;
    // Only http(s) — a page must not be able to point the chrome at a
    // `javascript:` or `file:` URL through the favicon field.
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("unsupported favicon scheme".into());
    }
    if let Some(tab) = core.tabs.lock().get_mut(id) {
        tab.favicon = Some(truncate(&url, 2048));
    }
    push_state(&app);
    Ok(())
}

// ---------------------------------------------------------------------------

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        get_state,
        set_settings,
        reset_settings,
        open_tab,
        activate_tab,
        close_tab,
        discard_tab,
        archive_tab,
        navigate_tab,
        reload_tab,
        history_back,
        history_forward,
        set_pinned,
        reorder_tab,
        set_insets,
        set_panes,
        set_overlay,
        split_with,
        unsplit,
        toggle_reader,
        set_zoom,
        add_space,
        remove_space,
        switch_space,
        add_bookmark,
        remove_bookmark,
        clear_history,
        search_all,
        resolve_query,
        memory_sample,
        list_extensions,
        install_extension,
        set_extension_enabled,
        remove_extension,
        recent_drafts,
        discard_all_idle,
        page_draft,
        page_drafts,
        page_clear_drafts,
        page_scroll,
        page_favicon,
        page_shortcut,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_is_character_safe_and_strips_newlines() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("a\nb", 10), "a b");
        let long = "é".repeat(100);
        let cut = truncate(&long, 10);
        assert_eq!(cut.chars().count(), 10);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn scoring_prefers_prefix_then_word_start_then_middle() {
        let prefix = score_of("read", "reading ruler").unwrap();
        let word = score_of("read", "enable reading ruler").unwrap();
        let middle = score_of("read", "unreadable setting").unwrap();
        assert!(prefix > word, "{prefix} > {word}");
        assert!(word > middle, "{word} > {middle}");
        assert!(score_of("zzz", "reading ruler").is_none());
    }

    #[test]
    fn every_palette_entry_has_a_known_action() {
        for (title, _, action, arg) in PALETTE {
            assert!(
                matches!(*action, "settings" | "action"),
                "{title} has unknown action {action}"
            );
            assert!(!arg.is_empty(), "{title} has no argument");
        }
    }
}
