//! On-disk state: spaces, bookmarks, history, form drafts, session.
//!
//! Everything is plain JSON in the user's config directory. No database, no
//! background compaction, no sync service, no daemon. Writes are debounced and
//! atomic (temp file + rename), and every collection is bounded so a long-lived
//! profile cannot grow without limit.

use crate::tabs::{Tab, TabId, TabState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Beyond this, the oldest history entries are dropped. ~10k entries is a few
/// hundred KB and covers months of ordinary use.
const HISTORY_CAP: usize = 10_000;
/// Hard ceiling on stored drafts, independent of the retention window.
const DRAFT_CAP: usize = 5_000;

pub type SpaceId = u32;

// ---------------------------------------------------------------------------
// Spaces
// ---------------------------------------------------------------------------

/// An Arc-style "space": a named set of tabs with its own accent.
///
/// Deliberately lightweight — a space is a tag on a tab plus a name and a
/// colour. There is no separate profile directory, no separate cookie jar by
/// default, and no sync. See `docs/architecture.md` §8 for why.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Space {
    pub id: SpaceId,
    pub name: String,
    /// Name of an icon in Emerald's own set (`src/icons/`), not an emoji or a
    /// path — so a space always renders in the house style.
    pub icon: String,
    /// Catppuccin accent name; falls back to the global accent when empty.
    pub accent: String,
    pub created_ms: u64,
}

impl Space {
    pub fn scratch() -> Self {
        Self {
            id: 0,
            name: "Personal".into(),
            icon: "leaf".into(),
            accent: String::new(),
            created_ms: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Bookmarks / history
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bookmark {
    pub id: u32,
    pub title: String,
    pub url: String,
    pub space: SpaceId,
    pub added_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub last_visit_ms: u64,
    pub visits: u32,
}

// ---------------------------------------------------------------------------
// Drafts
// ---------------------------------------------------------------------------

/// A snapshot of one form field's contents.
///
/// Drafts are the reason Emerald can promise that text typed into a web form is
/// never lost to a crash, a misclick, or a session timeout. They are stored
/// locally, never transmitted, and pruned on a schedule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Draft {
    /// `origin + path + field identity`, stable across reloads.
    pub key: String,
    /// Shown in the recovery UI so a draft is recognisable.
    pub origin: String,
    pub path: String,
    /// Human label: the field's `<label>`, `aria-label`, `name`, or placeholder.
    pub label: String,
    pub value: String,
    pub updated_ms: u64,
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Session {
    pub tabs: Vec<Tab>,
    pub active: Option<TabId>,
    pub active_space: SpaceId,
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Store {
    pub spaces: Vec<Space>,
    pub bookmarks: Vec<Bookmark>,
    pub history: Vec<HistoryEntry>,
    pub drafts: HashMap<String, Draft>,
    pub session: Session,
    #[serde(skip)]
    dirty: bool,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            spaces: vec![Space::scratch()],
            bookmarks: Vec::new(),
            history: Vec::new(),
            drafts: HashMap::new(),
            session: Session::default(),
            dirty: false,
        }
    }
}

impl Store {
    pub fn path(dir: &Path) -> PathBuf {
        dir.join("state.json")
    }

    pub fn load(dir: &Path) -> Self {
        let path = Self::path(dir);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        match serde_json::from_str::<Self>(&raw) {
            Ok(mut s) => {
                if s.spaces.is_empty() {
                    s.spaces.push(Space::scratch());
                }
                s
            }
            Err(err) => {
                eprintln!("emerald: state.json is not readable ({err}); starting fresh");
                let _ = std::fs::rename(&path, path.with_extension("json.bak"));
                Self::default()
            }
        }
    }

    /// Write only when something changed. Called on a debounce and at exit.
    pub fn save_if_dirty(&mut self, dir: &Path) -> std::io::Result<bool> {
        if !self.dirty {
            return Ok(false);
        }
        std::fs::create_dir_all(dir)?;
        let path = Self::path(dir);
        let tmp = path.with_extension("json.tmp");
        let body = serde_json::to_string(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, &path)?;
        self.dirty = false;
        Ok(true)
    }

    pub fn touch(&mut self) {
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    // --- spaces ------------------------------------------------------------

    pub fn add_space(&mut self, name: String, icon: String, accent: String, now: u64) -> SpaceId {
        let id = self.spaces.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        self.spaces.push(Space { id, name, icon, accent, created_ms: now });
        self.dirty = true;
        id
    }

    /// Remove a space. Space `0` is permanent — there is always somewhere for a
    /// tab to live.
    pub fn remove_space(&mut self, id: SpaceId) -> bool {
        if id == 0 {
            return false;
        }
        let before = self.spaces.len();
        self.spaces.retain(|s| s.id != id);
        self.dirty |= self.spaces.len() != before;
        self.spaces.len() != before
    }

    // --- bookmarks ---------------------------------------------------------

    pub fn add_bookmark(&mut self, title: String, url: String, space: SpaceId, now: u64) -> u32 {
        if let Some(existing) = self.bookmarks.iter().find(|b| b.url == url && b.space == space) {
            return existing.id;
        }
        let id = self.bookmarks.iter().map(|b| b.id).max().unwrap_or(0) + 1;
        self.bookmarks.push(Bookmark { id, title, url, space, added_ms: now });
        self.dirty = true;
        id
    }

    pub fn remove_bookmark(&mut self, id: u32) -> bool {
        let before = self.bookmarks.len();
        self.bookmarks.retain(|b| b.id != id);
        self.dirty |= self.bookmarks.len() != before;
        self.bookmarks.len() != before
    }

    pub fn is_bookmarked(&self, url: &str) -> bool {
        self.bookmarks.iter().any(|b| b.url == url)
    }

    // --- history -----------------------------------------------------------

    pub fn record_visit(&mut self, url: &str, title: &str, now: u64) {
        if url.is_empty() || url.starts_with("about:") || url.starts_with("data:") {
            return;
        }
        match self.history.iter_mut().find(|h| h.url == url) {
            Some(entry) => {
                entry.visits += 1;
                entry.last_visit_ms = now;
                if !title.is_empty() {
                    entry.title = title.to_string();
                }
            }
            None => self.history.push(HistoryEntry {
                url: url.to_string(),
                title: title.to_string(),
                last_visit_ms: now,
                visits: 1,
            }),
        }
        if self.history.len() > HISTORY_CAP {
            self.history.sort_by_key(|h| std::cmp::Reverse(h.last_visit_ms));
            self.history.truncate(HISTORY_CAP);
        }
        self.dirty = true;
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.dirty = true;
    }

    /// Ranked lookup for the address bar and command palette.
    ///
    /// Scores by substring position, then by a frecency-ish blend of visit
    /// count and recency. Linear over history, which is fine at 10k entries and
    /// avoids carrying an index we would have to keep coherent.
    pub fn search_history(&self, query: &str, limit: usize, now: u64) -> Vec<&HistoryEntry> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            let mut recent: Vec<&HistoryEntry> = self.history.iter().collect();
            recent.sort_by_key(|h| std::cmp::Reverse(h.last_visit_ms));
            recent.truncate(limit);
            return recent;
        }
        let mut scored: Vec<(i64, &HistoryEntry)> = self
            .history
            .iter()
            .filter_map(|h| {
                let url = h.url.to_lowercase();
                let title = h.title.to_lowercase();
                let pos = url.find(&q).or_else(|| title.find(&q))?;
                let age_days = now.saturating_sub(h.last_visit_ms) / 86_400_000;
                let score = (h.visits as i64 * 12)
                    - (pos as i64)
                    - (age_days as i64 * 2)
                    + if title.starts_with(&q) || url.contains(&format!("//{q}")) { 40 } else { 0 };
                Some((score, h))
            })
            .collect();
        scored.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
        scored.into_iter().take(limit).map(|(_, h)| h).collect()
    }

    // --- drafts ------------------------------------------------------------

    pub fn put_draft(&mut self, draft: Draft) {
        if draft.value.is_empty() {
            self.drafts.remove(&draft.key);
        } else {
            self.drafts.insert(draft.key.clone(), draft);
        }
        if self.drafts.len() > DRAFT_CAP {
            let mut by_age: Vec<(String, u64)> =
                self.drafts.iter().map(|(k, d)| (k.clone(), d.updated_ms)).collect();
            by_age.sort_by_key(|(_, ms)| *ms);
            for (key, _) in by_age.into_iter().take(self.drafts.len() - DRAFT_CAP) {
                self.drafts.remove(&key);
            }
        }
        self.dirty = true;
    }

    /// Every draft belonging to one page, for the restore bar.
    pub fn drafts_for(&self, origin: &str, path: &str) -> Vec<&Draft> {
        let mut found: Vec<&Draft> = self
            .drafts
            .values()
            .filter(|d| d.origin == origin && d.path == path)
            .collect();
        found.sort_by_key(|d| d.key.clone());
        found
    }

    pub fn drop_drafts_for(&mut self, origin: &str, path: &str) -> usize {
        let before = self.drafts.len();
        self.drafts.retain(|_, d| !(d.origin == origin && d.path == path));
        self.dirty |= self.drafts.len() != before;
        before - self.drafts.len()
    }

    /// Delete drafts past the retention window. `0` days means keep forever.
    pub fn prune_drafts(&mut self, now: u64, retention_days: u16) -> usize {
        if retention_days == 0 {
            return 0;
        }
        let cutoff = now.saturating_sub(retention_days as u64 * 86_400_000);
        let before = self.drafts.len();
        self.drafts.retain(|_, d| d.updated_ms >= cutoff);
        let removed = before - self.drafts.len();
        self.dirty |= removed > 0;
        removed
    }

    /// Most recent drafts, newest first, for the command palette.
    pub fn recent_drafts(&self, limit: usize) -> Vec<&Draft> {
        let mut all: Vec<&Draft> = self.drafts.values().collect();
        all.sort_by_key(|d| std::cmp::Reverse(d.updated_ms));
        all.truncate(limit);
        all
    }

    // --- session -----------------------------------------------------------

    pub fn save_session(&mut self, tabs: &[Tab], active: Option<TabId>, space: SpaceId) {
        self.session = Session {
            tabs: tabs
                .iter()
                .filter(|t| !t.url.is_empty() && t.url != "about:blank")
                .cloned()
                .collect(),
            active,
            active_space: space,
        };
        self.dirty = true;
    }

    /// Session tabs, optionally forced to come back discarded.
    pub fn session_tabs(&self, discarded: bool) -> Vec<Tab> {
        self.session
            .tabs
            .iter()
            .cloned()
            .map(|mut t| {
                if discarded && t.state == TabState::Live {
                    t.state = TabState::Discarded;
                }
                t
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 86_400_000;

    fn draft(key: &str, value: &str, updated_ms: u64) -> Draft {
        Draft {
            key: key.into(),
            origin: "https://forum.test".into(),
            path: "/new".into(),
            label: "Message".into(),
            value: value.into(),
            updated_ms,
        }
    }

    #[test]
    fn drafts_are_replaced_not_appended() {
        let mut s = Store::default();
        s.put_draft(draft("a", "hello", 1));
        s.put_draft(draft("a", "hello there", 2));
        assert_eq!(s.drafts.len(), 1);
        assert_eq!(s.drafts["a"].value, "hello there");
    }

    #[test]
    fn clearing_a_field_removes_its_draft() {
        let mut s = Store::default();
        s.put_draft(draft("a", "typed", 1));
        s.put_draft(draft("a", "", 2));
        assert!(s.drafts.is_empty(), "an emptied field should not linger");
    }

    #[test]
    fn prune_respects_retention_and_keep_forever() {
        let mut s = Store::default();
        s.put_draft(draft("old", "x", 0));
        s.put_draft(draft("new", "y", 20 * DAY));
        assert_eq!(s.prune_drafts(20 * DAY, 14), 1);
        assert!(s.drafts.contains_key("new"));

        s.put_draft(draft("ancient", "z", 0));
        assert_eq!(s.prune_drafts(999 * DAY, 0), 0, "0 days means keep forever");
        assert_eq!(s.drafts.len(), 2);
    }

    #[test]
    fn drafts_for_page_groups_by_origin_and_path() {
        let mut s = Store::default();
        s.put_draft(draft("a", "one", 1));
        s.put_draft(draft("b", "two", 2));
        let mut other = draft("c", "three", 3);
        other.path = "/edit/9".into();
        s.put_draft(other);

        assert_eq!(s.drafts_for("https://forum.test", "/new").len(), 2);
        assert_eq!(s.drop_drafts_for("https://forum.test", "/new"), 2);
        assert_eq!(s.drafts.len(), 1);
    }

    #[test]
    fn history_dedupes_and_counts_visits() {
        let mut s = Store::default();
        s.record_visit("https://a.test/", "A", 1_000);
        s.record_visit("https://a.test/", "A v2", 2_000);
        assert_eq!(s.history.len(), 1);
        assert_eq!(s.history[0].visits, 2);
        assert_eq!(s.history[0].title, "A v2");
    }

    #[test]
    fn history_ignores_internal_urls() {
        let mut s = Store::default();
        s.record_visit("about:blank", "", 1);
        s.record_visit("data:text/html,hi", "", 1);
        assert!(s.history.is_empty());
    }

    #[test]
    fn history_search_prefers_frequent_and_recent() {
        let mut s = Store::default();
        let now = 100 * DAY;
        s.history.push(HistoryEntry {
            url: "https://docs.test/guide".into(),
            title: "Guide".into(),
            last_visit_ms: now,
            visits: 30,
        });
        s.history.push(HistoryEntry {
            url: "https://old.test/guide".into(),
            title: "Guide archive".into(),
            last_visit_ms: 0,
            visits: 1,
        });
        let hits = s.search_history("guide", 5, now);
        assert_eq!(hits[0].url, "https://docs.test/guide");
    }

    #[test]
    fn space_zero_cannot_be_removed() {
        let mut s = Store::default();
        assert!(!s.remove_space(0));
        let id = s.add_space("Work".into(), "grid".into(), "sapphire".into(), 0);
        assert!(s.remove_space(id));
    }

    #[test]
    fn bookmarks_dedupe_by_url_within_a_space() {
        let mut s = Store::default();
        let a = s.add_bookmark("T".into(), "https://x.test".into(), 0, 0);
        let b = s.add_bookmark("T again".into(), "https://x.test".into(), 0, 1);
        assert_eq!(a, b);
        assert_eq!(s.bookmarks.len(), 1);
    }

    #[test]
    fn save_is_skipped_when_nothing_changed() {
        let dir = std::env::temp_dir().join(format!("emerald-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut s = Store::default();
        assert!(!s.save_if_dirty(&dir).unwrap(), "clean store writes nothing");
        s.record_visit("https://a.test/", "A", 1);
        assert!(s.save_if_dirty(&dir).unwrap());
        assert!(!s.save_if_dirty(&dir).unwrap(), "second save is a no-op");

        let reloaded = Store::load(&dir);
        assert_eq!(reloaded.history.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
