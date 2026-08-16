/// Declaring an app ACL manifest turns on access control for Emerald's *own*
/// commands, not just plugin commands. That is deliberate and load-bearing.
///
/// Emerald renders untrusted web pages in sibling webviews inside the same
/// window as its chrome. Tauri always ACL-checks commands invoked from remote
/// origins, so without a manifest the page commands below would simply be
/// denied and form-draft recovery would silently never work. With a manifest,
/// every command has to be granted explicitly, and `capabilities/` becomes an
/// auditable list of exactly which webview may call what:
///
///   * `capabilities/chrome.json` — the `chrome` webview, local origin only.
///   * `capabilities/pages.json`  — `tab:*` webviews, six narrow
///     commands and nothing else.
///
/// Adding a command here without granting it in a capability makes it callable
/// by nobody, which is the correct default. `commands.rs` additionally checks
/// the calling webview's label in Rust, so the two layers have to both fail
/// before a page can reach privileged state.
fn main() {
    let attributes = tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            // --- chrome only ---
            "get_state",
            "set_settings",
            "reset_settings",
            "open_tab",
            "activate_tab",
            "close_tab",
            "discard_tab",
            "archive_tab",
            "navigate_tab",
            "reload_tab",
            "history_back",
            "history_forward",
            "set_pinned",
            "reorder_tab",
            "set_insets",
            "set_panes",
            "set_overlay",
            "split_with",
            "unsplit",
            "toggle_reader",
            "set_zoom",
            "add_space",
            "remove_space",
            "switch_space",
            "add_bookmark",
            "remove_bookmark",
            "clear_history",
            "search_all",
            "resolve_query",
            "memory_sample",
            "list_extensions",
            "install_extension",
            "install_from_store",
            "check_for_update",
            "download_update",
            "set_extension_enabled",
            "remove_extension",
            "recent_drafts",
            "discard_all_idle",
            // --- callable by rendered web pages ---
            "page_draft",
            "page_drafts",
            "page_clear_drafts",
            "page_scroll",
            "page_favicon",
            "page_shortcut",
        ]),
    );

    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
