//! Linux-only: makes multi-webview positioning actually work.
//!
//! ## The problem
//!
//! Emerald's whole window model is "one window, many webviews, positioned by
//! the core" (see `runtime.rs`). On macOS and Windows that works directly.
//! On Linux it silently does not, and the failure is not obvious:
//!
//! `tauri-runtime-wry` builds every child webview into the window's
//! `default_vbox()` — a `GtkBox`. wry's container attachment then branches on
//! the container's type: a `GtkFixed` gets `fixed.put(webview, x, y)`, but a
//! `GtkBox` gets `pack_start(webview, expand: true, fill: true, 0)`. Packed
//! children share the box's height equally, and wry's `set_bounds` only calls
//! `size_allocate` when its internal `is_in_fixed_parent` flag is set — which
//! it is not.
//!
//! The result: `Webview::set_position` and `set_size` return `Ok(())`, report
//! no error, and do nothing. With the chrome plus one page open, each takes
//! half the window height. That is exactly what Emerald did until this module
//! existed, and it is worth knowing that the API lies rather than fails.
//!
//! ## The fix
//!
//! Do what wry's own `gtk_multiwebview` example does — put the webviews in a
//! `GtkFixed` — but after the fact, because Tauri gives no way to choose the
//! container at build time:
//!
//!   1. [`install`] creates a `GtkFixed` and packs it into the window's vbox.
//!   2. [`place`] reparents a webview's widget out of the vbox into that
//!      `GtkFixed` the first time it sees it, then positions it.
//!
//! Positioning goes through `Webview::with_webview`, which hands us the real
//! `webkit2gtk::WebView` on the GTK main thread. The `GtkFixed` is not
//! `Send`, so it cannot be captured into that closure; instead it is found by
//! walking up from the widget to the toplevel window and back down. Slightly
//! roundabout, entirely safe, and it keeps all GTK access on the main thread.
//!
//! If upstream ever builds child webviews into a `GtkFixed`, this module
//! becomes a no-op that can be deleted — [`place`] would still work, because
//! the widget would already be in a fixed parent.

use gtk::prelude::*;
use tauri::Webview;

/// Name we give our container so it can be found again among the vbox's
/// children without depending on child ordering.
const FIXED_NAME: &str = "emerald-webview-fixed";

/// Create the `GtkFixed` that page webviews will live in, and pack it into the
/// window's default vbox.
///
/// Safe to call more than once per window; the second call finds the existing
/// container and does nothing.
pub fn install(window: &tauri::window::Window) -> tauri::Result<()> {
    let vbox = window.default_vbox()?;
    if find_fixed(&vbox).is_some() {
        return Ok(());
    }
    let fixed = gtk::Fixed::new();
    fixed.set_widget_name(FIXED_NAME);
    // expand + fill so the container occupies the whole client area; the
    // webviews inside it are then positioned in absolute coordinates.
    vbox.pack_start(&fixed, true, true, 0);
    fixed.show_all();
    Ok(())
}

/// Position and size a webview, reparenting it into the `GtkFixed` if needed.
///
/// Coordinates are logical pixels relative to the window's client area, matching
/// what `TabManager::rect_for` produces.
pub fn place(webview: &Webview, x: f64, y: f64, w: f64, h: f64) {
    let (x, y, w, h) = (x.round() as i32, y.round() as i32, w.round() as i32, h.round() as i32);
    let _ = webview.with_webview(move |platform| {
        let widget: gtk::Widget = platform.inner().upcast();
        let Some(fixed) = ensure_in_fixed(&widget) else {
            return;
        };
        // `set_size_request` rather than `size_allocate`: GtkFixed honours the
        // child's requested size, and a direct allocation is overwritten on the
        // next layout pass.
        widget.set_size_request(w.max(1), h.max(1));
        fixed.move_(&widget, x, y);
        widget.show();
    });
}

/// Hide a webview without destroying it — used for tabs that are live but not
/// in a visible pane.
pub fn hide(webview: &Webview) {
    let _ = webview.with_webview(move |platform| {
        let widget: gtk::Widget = platform.inner().upcast();
        widget.hide();
    });
}

/// Move `widget` into the window's `GtkFixed` if it is not already there, and
/// return that container.
fn ensure_in_fixed(widget: &gtk::Widget) -> Option<gtk::Fixed> {
    if let Some(parent) = widget.parent() {
        if let Ok(fixed) = parent.clone().downcast::<gtk::Fixed>() {
            return Some(fixed);
        }
        let fixed = find_fixed_from_toplevel(widget)?;
        // A widget must leave its old container before joining a new one, and
        // GTK drops the last reference on removal — so take one first.
        let keep = widget.clone();
        if let Ok(container) = parent.downcast::<gtk::Container>() {
            container.remove(&keep);
        }
        fixed.put(&keep, 0, 0);
        return Some(fixed);
    }
    find_fixed_from_toplevel(widget)
}

fn find_fixed_from_toplevel(widget: &gtk::Widget) -> Option<gtk::Fixed> {
    let toplevel = widget.toplevel()?;
    let container = toplevel.downcast::<gtk::Container>().ok()?;
    find_fixed_deep(&container)
}

fn find_fixed(container: &impl IsA<gtk::Container>) -> Option<gtk::Fixed> {
    container
        .as_ref()
        .children()
        .into_iter()
        .find_map(|child| match child.downcast::<gtk::Fixed>() {
            Ok(fixed) if fixed.widget_name() == FIXED_NAME => Some(fixed),
            _ => None,
        })
}

/// Depth-limited search for our container. The window's child is the vbox and
/// the fixed is inside it, so two levels is enough; the bound exists so a
/// future change to the widget tree cannot turn this into a deep walk.
fn find_fixed_deep(container: &gtk::Container) -> Option<gtk::Fixed> {
    if let Some(found) = find_fixed(container) {
        return Some(found);
    }
    for child in container.children() {
        if let Ok(inner) = child.downcast::<gtk::Container>() {
            if let Some(found) = find_fixed(&inner) {
                return Some(found);
            }
        }
    }
    None
}
