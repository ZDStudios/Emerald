//! Tab model and lifecycle policy.
//!
//! This module deliberately knows nothing about Tauri. It owns the *decisions*
//! — which tab is live, which gets discarded, where each pane sits — and
//! returns them as a list of [`Effect`]s. `runtime.rs` is the only place that
//! touches real webviews, and it does so by applying those effects.
//!
//! The split exists so the part that actually saves memory is unit-testable
//! without a display server. See the tests at the bottom of this file.
//!
//! ## What "discarded" means here
//!
//! A discarded tab has had its webview destroyed. On Linux/macOS that
//! terminates the WebKit web process backing it and returns its resident
//! memory to the OS. The tab keeps its URL, title, favicon and scroll
//! position — a few hundred bytes — and reloads when you click it. This is
//! not a visual collapse; `bench/` measures the difference.

use serde::{Deserialize, Serialize};

pub type TabId = u32;
pub type SpaceId = u32;

/// Lifecycle state of a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabState {
    /// Holds a live webview and a web process.
    Live,
    /// Webview destroyed, process gone, metadata kept. Still in the tab strip.
    Discarded,
    /// Discarded *and* moved out of the strip into the Archive. Reachable from
    /// the command palette; never deleted without being asked.
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: TabId,
    pub space: SpaceId,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub state: TabState,
    /// Pinned tabs are never discarded by policy and never auto-archived.
    pub pinned: bool,
    /// Epoch millis when this tab last had focus.
    pub last_active_ms: u64,
    pub opened_ms: u64,
    /// Restored after a discard/resume round trip.
    pub scroll_y: f64,
    /// Whether reader mode is on for this tab.
    pub reader: bool,
    pub loading: bool,
}

impl Tab {
    /// What the strip shows before a page has reported a title.
    pub fn display_title(&self) -> &str {
        if !self.title.is_empty() {
            &self.title
        } else if self.url.is_empty() || self.url == "about:blank" {
            "New tab"
        } else {
            &self.url
        }
    }
}

/// Chrome insets around the content area, in logical pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Insets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

/// A visible content pane, as a fraction of the content area.
///
/// Fractions rather than pixels so that resizing the window needs no round
/// trip to the chrome: the core recomputes rects from the new window size
/// alone. One pane is the normal case; two is split view.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pane {
    pub tab: TabId,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Pane {
    pub fn full(tab: TabId) -> Self {
        Self { tab, x: 0.0, y: 0.0, w: 1.0, h: 1.0 }
    }
}

/// A side effect the runtime must apply to real webviews.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Build a webview for this tab and load `url`.
    Create { id: TabId, url: String },
    /// Destroy this tab's webview. Its web process dies with it.
    Destroy { id: TabId },
    /// Point an existing webview at a new URL.
    Navigate { id: TabId, url: String },
    /// Recompute and apply every webview's position and size.
    Relayout,
}

/// Ordered, stable tab collection plus the discard policy.
#[derive(Debug)]
pub struct TabManager {
    tabs: Vec<Tab>,
    active: Option<TabId>,
    panes: Vec<Pane>,
    insets: Insets,
    next_id: TabId,
    /// Mirror of `settings.focus_access.attention.max_live_tabs`.
    pub max_live: u16,
    /// Mirror of `settings.focus_access.predictability.stable_tab_order`.
    pub stable_order: bool,
    pub active_space: SpaceId,
}

impl Default for TabManager {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            active: None,
            panes: Vec::new(),
            insets: Insets::default(),
            next_id: 1,
            max_live: 6,
            stable_order: true,
            active_space: 0,
        }
    }
}

impl TabManager {
    /// Build a manager with policy mirrored from settings.
    pub fn new(max_live: u16, stable_order: bool) -> Self {
        Self { max_live, stable_order, ..Default::default() }
    }

    // --- read-only views ---------------------------------------------------

    pub fn all(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn get(&self, id: TabId) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    pub fn get_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    pub fn active(&self) -> Option<TabId> {
        self.active
    }

    pub fn panes(&self) -> &[Pane] {
        &self.panes
    }

    pub fn insets(&self) -> Insets {
        self.insets
    }

    pub fn live_count(&self) -> usize {
        self.tabs.iter().filter(|t| t.state == TabState::Live).count()
    }

    /// Tabs shown in the strip for the current space: everything but archived.
    pub fn visible_in_space(&self, space: SpaceId) -> impl Iterator<Item = &Tab> {
        self.tabs
            .iter()
            .filter(move |t| t.space == space && t.state != TabState::Archived)
    }

    /// True when this tab is on screen right now, so policy must not touch it.
    fn is_showing(&self, id: TabId) -> bool {
        self.active == Some(id) || self.panes.iter().any(|p| p.tab == id)
    }

    // --- mutations ---------------------------------------------------------

    /// Open a new tab and focus it.
    pub fn open(&mut self, url: String, space: SpaceId, now: u64) -> (TabId, Vec<Effect>) {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab {
            id,
            space,
            url: url.clone(),
            title: String::new(),
            favicon: None,
            state: TabState::Live,
            pinned: false,
            last_active_ms: now,
            opened_ms: now,
            scroll_y: 0.0,
            reader: false,
            loading: true,
        });
        let mut fx = vec![Effect::Create { id, url }];
        self.active = Some(id);
        self.panes = vec![Pane::full(id)];
        fx.extend(self.enforce_live_cap());
        fx.push(Effect::Relayout);
        (id, fx)
    }

    /// Focus a tab, resurrecting it if it had been discarded.
    pub fn activate(&mut self, id: TabId, now: u64) -> Vec<Effect> {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) else {
            return Vec::new();
        };
        tab.last_active_ms = now;
        let was = tab.state;
        let url = tab.url.clone();
        let space = tab.space;
        tab.state = TabState::Live;
        if was != TabState::Live {
            tab.loading = true;
        }

        self.active = Some(id);
        self.active_space = space;
        self.panes = vec![Pane::full(id)];

        let mut fx = Vec::new();
        if was != TabState::Live {
            fx.push(Effect::Create { id, url });
        }
        fx.extend(self.enforce_live_cap());
        fx.push(Effect::Relayout);
        fx
    }

    /// Close a tab for good.
    pub fn close(&mut self, id: TabId) -> Vec<Effect> {
        let Some(idx) = self.tabs.iter().position(|t| t.id == id) else {
            return Vec::new();
        };
        let was_live = self.tabs[idx].state == TabState::Live;
        let space = self.tabs[idx].space;
        self.tabs.remove(idx);

        let mut fx = Vec::new();
        if was_live {
            fx.push(Effect::Destroy { id });
        }
        self.panes.retain(|p| p.tab != id);

        if self.active == Some(id) {
            // Prefer the tab that took this one's slot, then its left neighbour.
            let next = self
                .tabs
                .iter()
                .filter(|t| t.space == space && t.state != TabState::Archived)
                .map(|t| t.id)
                .nth(idx)
                .or_else(|| {
                    self.tabs
                        .iter()
                        .filter(|t| t.space == space && t.state != TabState::Archived)
                        .map(|t| t.id)
                        .next_back()
                });
            self.active = None;
            if let Some(next) = next {
                // `now` is irrelevant here; activate refreshes it on real focus.
                let last = self.get(next).map(|t| t.last_active_ms).unwrap_or(0);
                fx.extend(self.activate(next, last));
                return fx;
            }
            self.panes.clear();
        } else if self.panes.len() == 1 {
            self.panes = self.active.map(Pane::full).into_iter().collect();
        }
        fx.push(Effect::Relayout);
        fx
    }

    /// Discard a tab's webview, keeping the tab. This is the memory lever.
    pub fn discard(&mut self, id: TabId) -> Vec<Effect> {
        if self.is_showing(id) {
            return Vec::new(); // never discard something on screen
        }
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) else {
            return Vec::new();
        };
        if tab.state != TabState::Live {
            return Vec::new();
        }
        tab.state = TabState::Discarded;
        tab.loading = false;
        vec![Effect::Destroy { id }]
    }

    /// Discard and remove from the strip.
    pub fn archive(&mut self, id: TabId) -> Vec<Effect> {
        let mut fx = self.discard(id);
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            if !self.panes.iter().any(|p| p.tab == id) && self.active != Some(id) {
                tab.state = TabState::Archived;
            }
        }
        if !fx.is_empty() {
            fx.push(Effect::Relayout);
        }
        fx
    }

    pub fn navigate(&mut self, id: TabId, url: String) -> Vec<Effect> {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) else {
            return Vec::new();
        };
        tab.url = url.clone();
        tab.scroll_y = 0.0;
        tab.loading = true;
        match tab.state {
            TabState::Live => vec![Effect::Navigate { id, url }],
            _ => {
                tab.state = TabState::Live;
                vec![Effect::Create { id, url }, Effect::Relayout]
            }
        }
    }

    pub fn set_pinned(&mut self, id: TabId, pinned: bool) {
        if let Some(tab) = self.get_mut(id) {
            tab.pinned = pinned;
        }
    }

    /// Reorder a tab. Refused when `stable_tab_order` is on and the move was
    /// not user-initiated — the caller passes `user_initiated` for drags.
    pub fn reorder(&mut self, id: TabId, to: usize, user_initiated: bool) -> bool {
        if self.stable_order && !user_initiated {
            return false;
        }
        let Some(from) = self.tabs.iter().position(|t| t.id == id) else {
            return false;
        };
        let to = to.min(self.tabs.len().saturating_sub(1));
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        true
    }

    // --- layout ------------------------------------------------------------

    pub fn set_insets(&mut self, insets: Insets) -> Vec<Effect> {
        if self.insets == insets {
            return Vec::new();
        }
        self.insets = insets;
        vec![Effect::Relayout]
    }

    /// Replace the visible pane set. Any pane referencing a discarded tab
    /// brings it back to life first.
    pub fn set_panes(&mut self, panes: Vec<Pane>, now: u64) -> Vec<Effect> {
        let mut fx = Vec::new();
        for pane in &panes {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == pane.tab) {
                if tab.state != TabState::Live {
                    tab.state = TabState::Live;
                    tab.loading = true;
                    tab.last_active_ms = now;
                    fx.push(Effect::Create { id: pane.tab, url: tab.url.clone() });
                }
            }
        }
        self.panes = panes;
        if let Some(first) = self.panes.first().map(|p| p.tab) {
            if !self.panes.iter().any(|p| Some(p.tab) == self.active) {
                self.active = Some(first);
            }
        }
        fx.extend(self.enforce_live_cap());
        fx.push(Effect::Relayout);
        fx
    }

    /// Split the current view with `other` beside the active tab.
    pub fn split_with(&mut self, other: TabId, now: u64) -> Vec<Effect> {
        let Some(active) = self.active else {
            return Vec::new();
        };
        if active == other {
            return Vec::new();
        }
        self.set_panes(
            vec![
                Pane { tab: active, x: 0.0, y: 0.0, w: 0.5, h: 1.0 },
                Pane { tab: other, x: 0.5, y: 0.0, w: 0.5, h: 1.0 },
            ],
            now,
        )
    }

    pub fn unsplit(&mut self) -> Vec<Effect> {
        let Some(active) = self.active else {
            return Vec::new();
        };
        self.panes = vec![Pane::full(active)];
        vec![Effect::Relayout]
    }

    /// Pixel rect for a pane given the window's logical size.
    pub fn rect_for(&self, pane: &Pane, win_w: f64, win_h: f64) -> (f64, f64, f64, f64) {
        let cw = (win_w - self.insets.left - self.insets.right).max(0.0);
        let ch = (win_h - self.insets.top - self.insets.bottom).max(0.0);
        (
            self.insets.left + pane.x * cw,
            self.insets.top + pane.y * ch,
            (pane.w * cw).max(1.0),
            (pane.h * ch).max(1.0),
        )
    }

    // --- policy ------------------------------------------------------------

    /// Discard least-recently-used tabs until the live count is within
    /// `max_live`. Never touches pinned tabs or anything on screen.
    pub fn enforce_live_cap(&mut self) -> Vec<Effect> {
        let mut fx = Vec::new();
        loop {
            if self.live_count() <= self.max_live as usize {
                break;
            }
            let victim = self
                .tabs
                .iter()
                .filter(|t| {
                    t.state == TabState::Live && !t.pinned && !self.is_showing(t.id)
                })
                .min_by_key(|t| t.last_active_ms)
                .map(|t| t.id);
            match victim {
                // Everything left is pinned or on screen. The cap is a policy,
                // not a promise to break the user's explicit choices.
                None => break,
                Some(id) => fx.extend(self.discard(id)),
            }
        }
        fx
    }

    /// Time-based policy pass. Called by the one background timer Emerald runs.
    /// `suspend_after` / `archive_after` of `0` disable that half.
    pub fn sweep(&mut self, now: u64, suspend_after_ms: u64, archive_after_ms: u64) -> Vec<Effect> {
        let mut fx = Vec::new();

        if archive_after_ms > 0 {
            let stale: Vec<TabId> = self
                .tabs
                .iter()
                .filter(|t| {
                    t.state != TabState::Archived
                        && !t.pinned
                        && !self.is_showing(t.id)
                        && now.saturating_sub(t.last_active_ms) >= archive_after_ms
                })
                .map(|t| t.id)
                .collect();
            for id in stale {
                fx.extend(self.archive(id));
            }
        }

        if suspend_after_ms > 0 {
            let idle: Vec<TabId> = self
                .tabs
                .iter()
                .filter(|t| {
                    t.state == TabState::Live
                        && !t.pinned
                        && !self.is_showing(t.id)
                        && now.saturating_sub(t.last_active_ms) >= suspend_after_ms
                })
                .map(|t| t.id)
                .collect();
            for id in idle {
                fx.extend(self.discard(id));
            }
        }

        fx
    }

    /// Discard LRU tabs until `count` have been released, for memory pressure.
    /// Returns the effects and how many tabs were actually freed.
    pub fn relieve_pressure(&mut self, count: usize) -> (Vec<Effect>, usize) {
        let mut fx = Vec::new();
        let mut freed = 0;
        for _ in 0..count {
            let victim = self
                .tabs
                .iter()
                .filter(|t| t.state == TabState::Live && !t.pinned && !self.is_showing(t.id))
                .min_by_key(|t| t.last_active_ms)
                .map(|t| t.id);
            match victim {
                None => break,
                Some(id) => {
                    let e = self.discard(id);
                    if !e.is_empty() {
                        freed += 1;
                    }
                    fx.extend(e);
                }
            }
        }
        (fx, freed)
    }

    // --- session persistence ----------------------------------------------

    /// Rehydrate from a saved session. When `discarded` is true every tab
    /// comes back as a placeholder, so startup cost is independent of how many
    /// tabs were open last time.
    pub fn restore(&mut self, tabs: Vec<Tab>, active: Option<TabId>, discarded: bool) -> Vec<Effect> {
        self.next_id = tabs.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        self.tabs = tabs;
        for tab in &mut self.tabs {
            tab.loading = false;
            if discarded && tab.state == TabState::Live {
                tab.state = TabState::Discarded;
            }
        }
        self.active = None;
        self.panes.clear();

        let target = active
            .filter(|id| self.tabs.iter().any(|t| t.id == *id))
            .or_else(|| {
                self.tabs
                    .iter()
                    .filter(|t| t.state != TabState::Archived)
                    .max_by_key(|t| t.last_active_ms)
                    .map(|t| t.id)
            });
        match target {
            // The active tab always loads. Everything else stays a placeholder.
            Some(id) => {
                let last = self.get(id).map(|t| t.last_active_ms).unwrap_or(0);
                self.active_space = self.get(id).map(|t| t.space).unwrap_or(0);
                self.activate(id, last)
            }
            None => vec![Effect::Relayout],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;

    fn mgr(max_live: u16) -> TabManager {
        TabManager { max_live, ..Default::default() }
    }

    fn open_n(m: &mut TabManager, n: usize, now: u64) -> Vec<TabId> {
        (0..n)
            .map(|i| m.open(format!("https://example.test/{i}"), 0, now + i as u64).0)
            .collect()
    }

    #[test]
    fn cap_discards_least_recently_used_only() {
        let mut m = mgr(3);
        let ids = open_n(&mut m, 3, 1_000);
        assert_eq!(m.live_count(), 3);

        // Touch the oldest so it is no longer the LRU.
        m.activate(ids[0], 9_000);

        let (_id4, _) = m.open("https://example.test/4".into(), 0, 10_000);
        assert_eq!(m.live_count(), 3, "cap held");
        // ids[1] was the least recently used, so it is the one that went.
        assert_eq!(m.get(ids[1]).unwrap().state, TabState::Discarded);
        assert_eq!(m.get(ids[0]).unwrap().state, TabState::Live);
    }

    #[test]
    fn discard_emits_destroy_so_the_process_actually_dies() {
        let mut m = mgr(2);
        let ids = open_n(&mut m, 3, 1_000);
        // Opening the third pushed the first out.
        let fx = m.open("https://example.test/x".into(), 0, 2_000).1;
        assert!(
            fx.iter().any(|e| matches!(e, Effect::Destroy { .. })),
            "cap enforcement must destroy a webview, not just hide it: {fx:?}"
        );
        assert!(ids.iter().any(|id| m.get(*id).unwrap().state == TabState::Discarded));
    }

    #[test]
    fn pinned_tabs_survive_the_cap() {
        let mut m = mgr(2);
        let ids = open_n(&mut m, 2, 1_000);
        m.set_pinned(ids[0], true);
        m.open("https://example.test/new".into(), 0, 5_000);
        assert_eq!(m.get(ids[0]).unwrap().state, TabState::Live, "pinned kept");
        assert_eq!(m.get(ids[1]).unwrap().state, TabState::Discarded);
    }

    #[test]
    fn on_screen_tabs_are_never_discarded_even_over_cap() {
        let mut m = mgr(1);
        let ids = open_n(&mut m, 2, 1_000);
        m.split_with(ids[0], 2_000);
        // Both panes are visible; the cap of 1 cannot be met without breaking
        // the split, so it yields.
        assert_eq!(m.live_count(), 2);
        assert!(m.discard(ids[0]).is_empty(), "visible pane refused discard");
    }

    #[test]
    fn activating_a_discarded_tab_recreates_it_with_its_url() {
        let mut m = mgr(1);
        let ids = open_n(&mut m, 2, 1_000);
        assert_eq!(m.get(ids[0]).unwrap().state, TabState::Discarded);

        let fx = m.activate(ids[0], 3_000);
        assert!(fx.contains(&Effect::Create {
            id: ids[0],
            url: "https://example.test/0".into()
        }));
        assert_eq!(m.get(ids[0]).unwrap().state, TabState::Live);
    }

    #[test]
    fn sweep_suspends_idle_and_archives_stale() {
        let mut m = mgr(64);
        let ids = open_n(&mut m, 3, 0);
        m.activate(ids[2], 0);

        // 25 minutes later: suspend after 20, no archiving.
        let fx = m.sweep(25 * MIN, 20 * MIN, 0);
        assert!(fx.iter().any(|e| matches!(e, Effect::Destroy { .. })));
        assert_eq!(m.get(ids[0]).unwrap().state, TabState::Discarded);
        assert_eq!(m.get(ids[1]).unwrap().state, TabState::Discarded);
        assert_eq!(m.get(ids[2]).unwrap().state, TabState::Live, "active untouched");

        // Two hours later with archiving on.
        m.sweep(120 * MIN, 20 * MIN, 60 * MIN);
        assert_eq!(m.get(ids[0]).unwrap().state, TabState::Archived);
        assert_eq!(m.get(ids[2]).unwrap().state, TabState::Live);
    }

    #[test]
    fn sweep_is_a_no_op_when_both_policies_are_disabled() {
        let mut m = mgr(64);
        open_n(&mut m, 4, 0);
        assert!(m.sweep(999 * MIN, 0, 0).is_empty());
        assert_eq!(m.live_count(), 4);
    }

    #[test]
    fn closing_the_active_tab_focuses_a_neighbour() {
        let mut m = mgr(8);
        let ids = open_n(&mut m, 3, 1_000);
        m.activate(ids[1], 2_000);
        m.close(ids[1]);
        assert_eq!(m.all().len(), 2);
        assert!(m.active().is_some());
        assert_ne!(m.active(), Some(ids[1]));
    }

    #[test]
    fn closing_the_last_tab_leaves_no_active_and_no_panes() {
        let mut m = mgr(8);
        let ids = open_n(&mut m, 1, 1_000);
        m.close(ids[0]);
        assert!(m.active().is_none());
        assert!(m.panes().is_empty());
    }

    #[test]
    fn stable_order_refuses_automatic_reordering() {
        let mut m = mgr(8);
        let ids = open_n(&mut m, 3, 1_000);
        assert!(!m.reorder(ids[2], 0, false), "policy blocks automatic moves");
        assert_eq!(m.all()[0].id, ids[0]);
        assert!(m.reorder(ids[2], 0, true), "a drag is still allowed");
        assert_eq!(m.all()[0].id, ids[2]);
    }

    #[test]
    fn restore_brings_tabs_back_as_placeholders() {
        let mut m = mgr(8);
        let saved: Vec<Tab> = (1..=10)
            .map(|i| Tab {
                id: i,
                space: 0,
                url: format!("https://example.test/{i}"),
                title: format!("Tab {i}"),
                favicon: None,
                state: TabState::Live,
                pinned: false,
                last_active_ms: i as u64,
                opened_ms: 0,
                scroll_y: 0.0,
                reader: false,
                loading: false,
            })
            .collect();

        let fx = m.restore(saved, None, true);
        // Exactly one Create: the tab being focused. Nine stay discarded.
        let creates = fx.iter().filter(|e| matches!(e, Effect::Create { .. })).count();
        assert_eq!(creates, 1, "startup cost is independent of tab count");
        assert_eq!(m.live_count(), 1);
        assert_eq!(m.active(), Some(10), "most recently used tab wins");
    }

    #[test]
    fn layout_rects_respect_insets_and_split_fractions() {
        let mut m = mgr(8);
        let ids = open_n(&mut m, 2, 0);
        m.set_insets(Insets { top: 40.0, left: 200.0, right: 0.0, bottom: 0.0 });
        m.split_with(ids[0], 1_000);

        let panes = m.panes().to_vec();
        assert_eq!(panes.len(), 2);
        let (x, y, w, h) = m.rect_for(&panes[0], 1200.0, 800.0);
        assert_eq!((x, y), (200.0, 40.0));
        assert_eq!((w, h), (500.0, 760.0)); // (1200-200)/2, 800-40
        let (x2, _, w2, _) = m.rect_for(&panes[1], 1200.0, 800.0);
        assert_eq!((x2, w2), (700.0, 500.0));
    }

    #[test]
    fn relieve_pressure_stops_when_nothing_is_left_to_free() {
        let mut m = mgr(64);
        let ids = open_n(&mut m, 3, 0);
        m.set_pinned(ids[0], true);
        m.activate(ids[2], 1_000);
        // ids[1] is the only eligible victim.
        let (_, freed) = m.relieve_pressure(10);
        assert_eq!(freed, 1);
        assert_eq!(m.get(ids[0]).unwrap().state, TabState::Live);
        assert_eq!(m.get(ids[2]).unwrap().state, TabState::Live);
    }
}
