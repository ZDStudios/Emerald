//! Emerald browser core.
//!
//! Layering, outermost first:
//!
//! ```text
//!   commands.rs   IPC surface exposed to the chrome webview
//!   runtime.rs    the only module that touches real Tauri webviews
//!   ── everything below is plain Rust with no Tauri types ──
//!   tabs.rs       tab model + discard policy (unit-tested)
//!   store.rs      spaces, bookmarks, history, drafts, session (unit-tested)
//!   settings.rs   the settings schema, single source of truth (unit-tested)
//!   inject.rs     builds the CSS/JS handed to page webviews
//!   metrics.rs    resident-memory sampling for the budget policy + benchmarks
//! ```

pub mod commands;
pub mod extensions;
#[cfg(target_os = "linux")]
pub mod gtk_layout;
pub mod inject;
pub mod metrics;
pub mod runtime;
pub mod settings;
pub mod store;
pub mod tabs;

pub use runtime::run;
