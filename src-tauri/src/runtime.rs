//! The only module that touches real Tauri webviews.
//!
//! Everything below this layer is plain data. `runtime.rs` owns the window,
//! turns [`Effect`]s from `tabs.rs` into webview lifecycle calls, and runs the
//! single background timer Emerald permits itself.
//!
//! ## Window shape
//!
//! One OS window holds N+1 webviews:
//!
//! ```text
//!   ┌─ window "main" ───────────────────────────────┐
//!   │ ┌─ webview "chrome" (full size, on top) ─────┐│
//!   │ │ sidebar │  ← the content rect is a hole    ││
//!   │ │         │    punched by `insets`           ││
//!   │ └─────────┴───────────────────────────────────┘│
//!   │           ┌─ webview "tab:7" ────────────────┐ │
//!   │           │  the live page                   │ │
//!   │           └──────────────────────────────────┘ │
//!   └───────────────────────────────────────────────┘
//! ```
//!
//! The chrome webview is created first, so page webviews stack above it, and
//! the chrome is sized to fill the window while page webviews are positioned
//! into the inset rectangle. Non-visible page webviews are `hide()`-ed rather
//! than moved off-screen.

use crate::inject;
use crate::settings::Settings;
use crate::store::Store;
use crate::tabs::{Effect, Insets, TabId, TabManager, TabState};
use parking_lot::{Mutex, RwLock};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex as StdMutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{
    webview::WebviewBuilder, window::WindowBuilder, Emitter, LogicalPosition, LogicalSize, Manager,
    WebviewUrl,
};

pub const CHROME: &str = "chrome";
pub const MAIN_WINDOW: &str = "main";

pub fn tab_label(id: TabId) -> String {
    format!("tab:{id}")
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Startup milestones on stderr when `EMERALD_TRACE=1`.
///
/// This is instrumentation, not telemetry: it is off unless you turn it on, it
/// writes to your terminal, and it leaves no file and opens no socket. `bench/`
/// reads these lines to produce the numbers in `docs/benchmarks.md` — quoting
/// a startup figure that was not measured would be worse than quoting none.
pub fn trace(milestone: &str, start: std::time::Instant) {
    if std::env::var_os("EMERALD_TRACE").is_some() {
        eprintln!(
            "emerald-trace {milestone} {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
}

/// URLs passed on the command line: `emerald https://a.example https://b.example`.
///
/// Anything that is not a flag is treated as a URL or a search, resolved
/// through the same address-bar logic as typing it.
fn cli_urls() -> Vec<String> {
    std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect()
}

/// Shared application state, managed by Tauri.
pub struct Core {
    pub settings: RwLock<Settings>,
    pub tabs: Mutex<TabManager>,
    pub store: Mutex<Store>,
    pub config_dir: PathBuf,
    /// True while the chrome is showing a full-surface overlay — the command
    /// palette or the settings panel.
    ///
    /// Page webviews are native widgets stacked *above* the chrome webview, so
    /// anything the chrome paints in the content rect is behind them. An HTML
    /// overlay would open, be perfectly functional, and be completely
    /// invisible. While this flag is set, `relayout` hides every page webview
    /// so the chrome owns the whole window.
    overlay: AtomicBool,
    /// Set while the sampler thread should keep running.
    sampler: Arc<Sampler>,
}

impl Core {
    /// Persist settings and state. Cheap when nothing changed.
    pub fn flush(&self) {
        let settings = self.settings.read().clone();
        if let Err(e) = settings.save(&self.config_dir) {
            eprintln!("emerald: could not save settings: {e}");
        }
        let mut store = self.store.lock();
        if let Err(e) = store.save_if_dirty(&self.config_dir) {
            eprintln!("emerald: could not save state: {e}");
        }
    }

    /// Snapshot the open tabs into the session for next launch.
    pub fn checkpoint_session(&self) {
        let tabs = self.tabs.lock();
        let mut store = self.store.lock();
        store.save_session(tabs.all(), tabs.active(), tabs.active_space);
    }
}

// ---------------------------------------------------------------------------
// The one background timer
// ---------------------------------------------------------------------------

/// Emerald runs exactly one recurring task, and only when a setting requires
/// it. It exists to serve three policies:
///
///   * `attention.suspend_idle_minutes` — discard tabs idle for too long
///   * `attention.auto_archive_idle_minutes` — archive tabs idle for longer
///   * `performance.memory_budget_mb` — discard tabs when the tree is too big
///
/// With all three off, `Settings::needs_sampler()` is false, the thread parks
/// permanently on its condvar and consumes nothing. It is woken — not polled —
/// when settings change or when the app shuts down.
///
/// Justification for it existing at all: the alternatives are worse. Doing this
/// work on a timer in the chrome webview would keep a JS context hot; doing it
/// on user input would mean a tab idle overnight is still resident at breakfast.
/// One parked thread costs a stack and nothing else.
struct Sampler {
    lock: StdMutex<bool>,
    cv: Condvar,
    running: AtomicBool,
}

impl Sampler {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            lock: StdMutex::new(false),
            cv: Condvar::new(),
            running: AtomicBool::new(true),
        })
    }

    /// Wake the thread immediately (settings changed, or we are shutting down).
    fn poke(&self) {
        let mut guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        *guard = true;
        self.cv.notify_all();
    }

    fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.poke();
    }

    /// Sleep for `dur`, returning early if poked. `false` means "shut down".
    fn wait(&self, dur: Duration) -> bool {
        let mut guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        while !*guard {
            let (g, timeout) = self
                .cv
                .wait_timeout(guard, dur)
                .unwrap_or_else(|e| e.into_inner());
            guard = g;
            if timeout.timed_out() {
                break;
            }
        }
        *guard = false;
        self.running.load(Ordering::SeqCst)
    }
}

fn spawn_sampler(app: tauri::AppHandle, sampler: Arc<Sampler>) {
    std::thread::Builder::new()
        .name("emerald-policy".into())
        .spawn(move || loop {
            let (interval, suspend_ms, archive_ms, budget_mb, needed) = {
                let core = app.state::<Core>();
                let s = core.settings.read();
                (
                    Duration::from_secs(s.performance.sampler_interval_s.max(5) as u64),
                    s.focus_access.attention.suspend_idle_minutes as u64 * 60_000,
                    s.focus_access.attention.auto_archive_idle_minutes as u64 * 60_000,
                    s.performance.memory_budget_mb,
                    s.needs_sampler(),
                )
            };

            // Nothing to do: park indefinitely until settings change.
            let wait_for = if needed { interval } else { Duration::from_secs(3600) };
            if !sampler.wait(wait_for) {
                return;
            }
            if !needed {
                continue;
            }

            let core = app.state::<Core>();
            let mut effects = {
                let mut tabs = core.tabs.lock();
                tabs.sweep(now_ms(), suspend_ms, archive_ms)
            };

            if budget_mb > 0 {
                let sample = crate::metrics::sample();
                if !sample.unsupported && sample.rss_mb() > budget_mb as f64 {
                    // Free roughly one tab per 150MB over budget, then let the
                    // next tick reassess rather than dumping everything at once.
                    let over = sample.rss_mb() - budget_mb as f64;
                    let want = ((over / 150.0).ceil() as usize).clamp(1, 8);
                    let (fx, freed) = core.tabs.lock().relieve_pressure(want);
                    if freed > 0 {
                        eprintln!(
                            "emerald: {:.0}MB over budget, discarded {freed} tab(s)",
                            over
                        );
                    }
                    effects.extend(fx);
                }
            }

            if !effects.is_empty() {
                apply_effects(&app, effects);
                push_state(&app);
            }
            core.flush();
        })
        .expect("failed to spawn policy thread");
}

// ---------------------------------------------------------------------------
// Effect application
// ---------------------------------------------------------------------------

/// Apply a batch of [`Effect`]s to real webviews.
pub fn apply_effects(app: &tauri::AppHandle, effects: Vec<Effect>) {
    let mut needs_layout = false;
    for effect in effects {
        match effect {
            Effect::Create { id, url } => create_tab_webview(app, id, &url),
            Effect::Destroy { id } => {
                if let Some(wv) = app.get_webview(&tab_label(id)) {
                    // close() destroys the webview; on WebKit that terminates
                    // its web process and returns the memory to the OS.
                    if let Err(e) = wv.close() {
                        eprintln!("emerald: could not close tab {id}: {e}");
                    }
                }
                needs_layout = true;
            }
            Effect::Navigate { id, url } => {
                if let (Some(wv), Ok(parsed)) = (app.get_webview(&tab_label(id)), url.parse()) {
                    let _ = wv.navigate(parsed);
                }
            }
            Effect::Relayout => needs_layout = true,
        }
    }
    if needs_layout {
        relayout(app);
    }
}

fn create_tab_webview(app: &tauri::AppHandle, id: TabId, url: &str) {
    let Some(window) = app.get_window(MAIN_WINDOW) else {
        return;
    };
    let label = tab_label(id);
    if app.get_webview(&label).is_some() {
        return; // already live
    }

    let (init, throttling, scroll) = {
        let core = app.state::<Core>();
        let settings = core.settings.read();
        let scroll = core.tabs.lock().get(id).map(|t| t.scroll_y).unwrap_or(0.0);
        (
            inject::init_script(&settings, id),
            // Background tabs that survive the discard policy still get their
            // timers suspended by the engine. Free, and it stops idle pages
            // burning CPU behind the active one.
            tauri::utils::config::BackgroundThrottlingPolicy::Suspend,
            scroll,
        )
    };

    // A blank tab gets Emerald's own new-tab page, served from the app bundle.
    // It is loaded into a *page* webview like any website, so it is subject to
    // the same discard policy and gets the same content script — there is no
    // privileged "internal page" tier to keep in sync.
    let target = if url.is_empty() || url == "about:blank" || url == "about:newtab" {
        WebviewUrl::App("newtab.html".into())
    } else {
        match url.parse() {
            Ok(parsed) => WebviewUrl::External(parsed),
            Err(_) => WebviewUrl::App("newtab.html".into()),
        }
    };

    let app_for_nav = app.clone();
    let app_for_title = app.clone();
    let app_for_load = app.clone();

    let builder = WebviewBuilder::new(&label, target)
        .initialization_script(init)
        .background_throttling(throttling)
        .zoom_hotkeys_enabled(true)
        .on_navigation(move |url| {
            // Record the URL the tab is actually on. Returning true always:
            // Emerald does not block navigation, it only observes it.
            let core = app_for_nav.state::<Core>();
            let mut tabs = core.tabs.lock();
            if let Some(tab) = tabs.get_mut(id) {
                tab.url = url.to_string();
                tab.loading = true;
            }
            drop(tabs);
            push_state(&app_for_nav);
            true
        })
        .on_document_title_changed(move |_wv, title| {
            let core = app_for_title.state::<Core>();
            {
                let mut tabs = core.tabs.lock();
                if let Some(tab) = tabs.get_mut(id) {
                    tab.title = title;
                }
            }
            push_state(&app_for_title);
        })
        .on_page_load(move |wv, payload| {
            if !matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                return;
            }
            let core = app_for_load.state::<Core>();
            let (url, title) = {
                let mut tabs = core.tabs.lock();
                match tabs.get_mut(id) {
                    Some(tab) => {
                        tab.loading = false;
                        tab.url = payload.url().to_string();
                        (tab.url.clone(), tab.title.clone())
                    }
                    None => return,
                }
            };
            // `on_document_title_changed` only fires when the title *changes*
            // after the document is created, so a page whose <title> is parsed
            // with the rest of the head never emits it and the tab would keep
            // showing its URL. Read it directly on load as the reliable path;
            // the signal handler stays for later client-side title changes.
            if title.is_empty() {
                let app = app_for_load.clone();
                let _ = wv.eval_with_callback("document.title", move |raw| {
                    let Ok(found) = serde_json::from_str::<String>(&raw) else {
                        return;
                    };
                    if found.is_empty() {
                        return;
                    }
                    let core = app.state::<Core>();
                    if let Some(tab) = core.tabs.lock().get_mut(id) {
                        tab.title = found;
                    }
                    push_state(&app);
                });
            }

            core.store.lock().record_visit(&url, &title, now_ms());
            if scroll > 0.0 {
                let _ = wv.eval(inject::restore_scroll_script(scroll));
            }
            let reader_on = core.tabs.lock().get(id).map(|t| t.reader).unwrap_or(false);
            if reader_on {
                let _ = wv.eval(inject::reader_script(true));
            }
            push_state(&app_for_load);
        });

    // `suppress_popups` is applied at build time because the handler cannot be
    // swapped later; changing the setting affects webviews created after it.
    let suppress = {
        let core = app.state::<Core>();
        let s = core.settings.read();
        s.focus_access.predictability.suppress_popups
    };
    let app_for_popup = app.clone();
    let builder = if suppress {
        builder.on_new_window(move |url, _features| {
            // A page asking for a new window gets a tab instead — in Emerald's
            // own tab strip, in the current space, where you can see it. Nothing
            // opens off-screen or behind the window.
            let core = app_for_popup.state::<Core>();
            let space = core.tabs.lock().active_space;
            let effects = core.tabs.lock().open(url.to_string(), space, now_ms()).1;
            apply_effects(&app_for_popup, effects);
            push_state(&app_for_popup);
            tauri::webview::NewWindowResponse::Deny
        })
    } else {
        builder
    };

    // Position it correctly on creation to avoid a frame at the wrong size.
    let (pos, size) = pending_rect(app, id).unwrap_or((
        LogicalPosition::new(0.0, 0.0),
        LogicalSize::new(800.0, 600.0),
    ));

    match window.add_child(builder, pos, size) {
        Ok(wv) => {
            let core = app.state::<Core>();
            let showing = {
                let tabs = core.tabs.lock();
                tabs.panes().iter().any(|p| p.tab == id)
            };
            if !showing {
                let _ = wv.hide();
            }
        }
        Err(e) => eprintln!("emerald: could not create webview for tab {id}: {e}"),
    }
}

/// The rect a tab should occupy right now, if it is in a visible pane.
fn pending_rect(
    app: &tauri::AppHandle,
    id: TabId,
) -> Option<(LogicalPosition<f64>, LogicalSize<f64>)> {
    let window = app.get_window(MAIN_WINDOW)?;
    let scale = window.scale_factor().ok()?;
    let size = window.inner_size().ok()?.to_logical::<f64>(scale);
    let core = app.state::<Core>();
    let tabs = core.tabs.lock();
    let pane = tabs.panes().iter().find(|p| p.tab == id)?;
    let (x, y, w, h) = tabs.rect_for(pane, size.width, size.height);
    Some((LogicalPosition::new(x, y), LogicalSize::new(w, h)))
}

/// Position every live webview: visible panes into their rects, everything
/// else hidden.
pub fn relayout(app: &tauri::AppHandle) {
    let Some(window) = app.get_window(MAIN_WINDOW) else {
        return;
    };
    let Ok(scale) = window.scale_factor() else {
        return;
    };
    let Ok(phys) = window.inner_size() else {
        return;
    };
    let size = phys.to_logical::<f64>(scale);

    // Chrome always fills the window; it draws its own layout and leaves a
    // hole where content goes.
    if let Some(chrome) = app.get_webview(CHROME) {
        let _ = chrome.set_position(LogicalPosition::new(0.0, 0.0));
        let _ = chrome.set_size(LogicalSize::new(size.width, size.height));
        #[cfg(target_os = "linux")]
        crate::gtk_layout::place(&chrome, 0.0, 0.0, size.width, size.height);
    }

    let core = app.state::<Core>();
    let tabs = core.tabs.lock();
    // An overlay owns the whole window; page webviews would cover it.
    let visible: Vec<_> = if core.overlay.load(Ordering::SeqCst) {
        Vec::new()
    } else {
        tabs.panes().to_vec()
    };

    for tab in tabs.all() {
        let Some(wv) = app.get_webview(&tab_label(tab.id)) else {
            continue;
        };
        match visible.iter().find(|p| p.tab == tab.id) {
            Some(pane) => {
                let (x, y, w, h) = tabs.rect_for(pane, size.width, size.height);
                if std::env::var_os("EMERALD_TRACE").is_some() {
                    eprintln!(
                        "emerald-layout tab:{} rect=({x},{y},{w},{h}) win=({},{}) insets={:?}",
                        tab.id,
                        size.width,
                        size.height,
                        tabs.insets()
                    );
                }
                let _ = wv.set_position(LogicalPosition::new(x, y));
                let _ = wv.set_size(LogicalSize::new(w, h));
                let _ = wv.show();
                // On Linux the two calls above are silently no-ops; this is
                // what actually moves the webview. See gtk_layout.rs.
                #[cfg(target_os = "linux")]
                crate::gtk_layout::place(&wv, x, y, w, h);
            }
            None => {
                let _ = wv.hide();
                #[cfg(target_os = "linux")]
                crate::gtk_layout::hide(&wv);
            }
        }
    }
}

/// Push the whole UI state to the chrome webview.
///
/// One event carrying a snapshot, rather than a stream of granular deltas.
/// The state is small (tens of tabs), it makes the chrome a pure function of
/// core state, and it removes any chance of the two drifting apart.
pub fn push_state(app: &tauri::AppHandle) {
    let core = app.state::<Core>();
    let snapshot = crate::commands::snapshot(&core);
    let _ = app.emit_to(CHROME, "emerald://state", snapshot);
}

/// Re-inject settings into every live page without reloading it.
pub fn push_settings_to_pages(app: &tauri::AppHandle) {
    let core = app.state::<Core>();
    let settings = core.settings.read().clone();
    let ids: Vec<TabId> = core
        .tabs
        .lock()
        .all()
        .iter()
        .filter(|t| t.state == TabState::Live)
        .map(|t| t.id)
        .collect();
    for id in ids {
        if let Some(wv) = app.get_webview(&tab_label(id)) {
            let _ = wv.eval(inject::update_script(&settings, id));
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(crate::commands::handler())
        .setup(|app| {
            let boot = std::time::Instant::now();
            let handle = app.handle().clone();
            let config_dir = handle
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            std::fs::create_dir_all(&config_dir).ok();

            let settings = Settings::load(&config_dir);
            let mut store = Store::load(&config_dir);
            store.prune_drafts(now_ms(), settings.focus_access.input.draft_retention_days);

            let mut tabs = TabManager::new(
                settings.focus_access.attention.max_live_tabs,
                settings.focus_access.predictability.stable_tab_order,
            );
            let restore_discarded = settings.performance.restore_tabs_discarded;
            let session_tabs = store.session_tabs(restore_discarded);
            let session_active = store.session.active;
            tabs.active_space = store.session.active_space;

            let sampler = Sampler::new();
            app.manage(Core {
                settings: RwLock::new(settings),
                tabs: Mutex::new(tabs),
                store: Mutex::new(store),
                config_dir,
                overlay: AtomicBool::new(false),
                sampler: sampler.clone(),
            });

            // --- window + chrome ---------------------------------------------
            //
            // Native decorations, on purpose. Emerald could draw its own title
            // bar, and most browsers in this style do, but a reimplemented
            // title bar behaves subtly differently from every other window on
            // the machine — snapping, shortcuts, and accessibility tooling all
            // drift. "Predictable, low-surprise" has to mean the window too.
            let window = WindowBuilder::new(app, MAIN_WINDOW)
                .title("Emerald")
                .inner_size(1280.0, 820.0)
                .min_inner_size(560.0, 400.0)
                .decorations(true)
                .build()?;

            trace("window", boot);

            // Must happen before any webview is added: page webviews are
            // reparented into this container so they can be positioned at all.
            #[cfg(target_os = "linux")]
            if let Err(e) = crate::gtk_layout::install(&window) {
                eprintln!("emerald: could not install the layout container: {e}");
            }

            let scale = window.scale_factor().unwrap_or(1.0);
            let size = window.inner_size()?.to_logical::<f64>(scale);
            window.add_child(
                WebviewBuilder::new(CHROME, WebviewUrl::App("index.html".into()))
                    .transparent(false)
                    .zoom_hotkeys_enabled(false)
                    // The moment the chrome has painted is the moment Emerald is
                    // usable, so that is what the startup number in
                    // docs/benchmarks.md reports.
                    .on_page_load(move |_wv, payload| {
                        if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                            trace("chrome-painted", boot);
                        }
                    }),
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(size.width, size.height),
            )?;

            trace("chrome-webview", boot);

            // --- restore the previous session, or open what the CLI asked for --
            let effects = {
                let core = handle.state::<Core>();
                let requested = cli_urls();
                let mut tabs = core.tabs.lock();
                if !requested.is_empty() {
                    let settings = core.settings.read();
                    let mut fx = Vec::new();
                    for raw in requested {
                        fx.extend(tabs.open(settings.resolve_query(&raw), 0, now_ms()).1);
                    }
                    fx
                } else if session_tabs.is_empty() {
                    tabs.open("about:blank".into(), 0, now_ms()).1
                } else {
                    tabs.restore(session_tabs, session_active, restore_discarded)
                }
            };
            apply_effects(&handle, effects);
            trace("tabs-created", boot);

            // --- window events ------------------------------------------------
            let handle_for_resize = handle.clone();
            window.on_window_event(move |event| match event {
                tauri::WindowEvent::Resized(_) | tauri::WindowEvent::ScaleFactorChanged { .. } => {
                    // Insets are stored as pixels and panes as fractions, so a
                    // resize needs no round trip to the chrome.
                    relayout(&handle_for_resize);
                }
                tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed => {
                    let core = handle_for_resize.state::<Core>();
                    core.checkpoint_session();
                    core.flush();
                    core.sampler.stop();
                }
                _ => {}
            });

            spawn_sampler(handle.clone(), sampler);
            trace("ready", boot);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let core = window.state::<Core>();
                core.checkpoint_session();
                core.flush();
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to start Emerald")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let core = app.state::<Core>();
                core.checkpoint_session();
                core.flush();
                core.sampler.stop();
            }
        });
}

impl Core {
    /// Show or hide every page webview, for full-surface chrome overlays.
    /// Returns true when the state actually changed.
    pub fn set_overlay(&self, active: bool) -> bool {
        self.overlay.swap(active, Ordering::SeqCst) != active
    }

    /// Wake the policy thread — called when settings change so a new interval
    /// or a newly enabled policy takes effect immediately rather than after the
    /// old interval expires.
    pub fn poke_sampler(&self) {
        self.sampler.poke();
    }

    /// Keep the tab manager's mirrored policy fields in step with settings.
    pub fn sync_policy(&self) {
        let s = self.settings.read();
        let mut tabs = self.tabs.lock();
        tabs.max_live = s.focus_access.attention.max_live_tabs;
        tabs.stable_order = s.focus_access.predictability.stable_tab_order;
    }
}

/// Convenience for commands that need the window's logical size.
pub fn window_logical_size(app: &tauri::AppHandle) -> Option<(f64, f64)> {
    let window = app.get_window(MAIN_WINDOW)?;
    let scale = window.scale_factor().ok()?;
    let size = window.inner_size().ok()?.to_logical::<f64>(scale);
    Some((size.width, size.height))
}

/// Default insets used before the chrome reports its real layout.
pub const FALLBACK_INSETS: Insets = Insets {
    top: 0.0,
    right: 0.0,
    bottom: 0.0,
    left: 248.0,
};
