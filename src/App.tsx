/*
 * The shell.
 *
 * Holds the single state snapshot from the core, reports the content rect's
 * insets back to it, and owns the keyboard map. Everything else is a child
 * that receives state and calls `ipc`.
 */

import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { createStore, reconcile } from 'solid-js/store';
import { listen } from '@tauri-apps/api/event';
import {
  ipc,
  onIpcFailure,
  onState,
  type IpcFailure,
  type StateSnapshot,
} from './lib/ipc';
import { DEFAULT_SETTINGS, type Settings } from './lib/settings.gen';
import { applyTheme, hasMod } from './lib/theme';
import { Sidebar } from './components/Sidebar';
import { Toolbar } from './components/Toolbar';
import { CommandPalette } from './components/CommandPalette';
import { FocusAccess, type PanelSection } from './components/FocusAccess';
import { Content } from './components/Content';
import { TopTabs } from './components/TopTabs';
import { BookmarksBar } from './components/BookmarksBar';
import { StoreOffer } from './components/StoreOffer';
import { UpdateNotice } from './components/UpdateNotice';

/** Chords Emerald claims. Must stay in step with OWNED in
 * assets/content/emerald.js and OWNED_SHORTCUTS in commands.rs. */
const CLAIMED = new Set([
  'mod+k', 'mod+p', 'mod+t', 'mod+w', 'mod+l', 'mod+r',
  'mod+shift+r', 'mod+shift+a', 'mod+e', 'mod+\\', 'mod+,',
  'mod+1', 'mod+2', 'mod+3', 'mod+4', 'mod+5',
  'mod+6', 'mod+7', 'mod+8', 'mod+9',
]);

const EMPTY: StateSnapshot = {
  settings: DEFAULT_SETTINGS,
  tabs: [],
  active: null,
  panes: [],
  spaces: [],
  active_space: 0,
  bookmarks: [],
  live_count: 0,
  archived_count: 0,
  draft_count: 0,
};

export function App() {
  const [state, setState] = createStore<StateSnapshot>(EMPTY);
  const [paletteOpen, setPaletteOpen] = createSignal(false);
  const [panel, setPanel] = createSignal<PanelSection | null>(null);
  const [omniFocus, setOmniFocus] = createSignal(0);
  const [fatal, setFatal] = createSignal<string | null>(null);
  const [failures, setFailures] = createSignal<IpcFailure[]>([]);
  const [faultSeen, setFaultSeen] = createSignal(false);

  /* Shown until dismissed. Re-arms on a *new* failure, so acknowledging one
   * problem does not silence the next.
   *
   * A memo, not a plain accessor, and that is load-bearing. The overlay effect
   * below reads this and calls `set_overlay` — itself an IPC call. If that call
   * fails it records a failure, which changes `failures()`, which would re-run
   * the effect, which would call it again: a tight infinite loop that only
   * triggers once IPC is already broken, i.e. exactly when this code matters.
   * A memo compares by value, so a second failure while the banner is already
   * up does not re-notify. */
  const showFault = createMemo(
    () => !faultSeen() && (fatal() !== null || failures().length > 0)
  );

  /* A browser that cannot reach its own core has to say so. Without this the
   * only symptom is an interface that ignores you. */
  onMount(() =>
    onCleanup(
      onIpcFailure((f) => {
        setFailures(f);
        setFaultSeen(false);
      })
    )
  );

  let contentRef: HTMLDivElement | undefined;

  const activeTab = () => state.tabs.find((t) => t.id === state.active);
  const focusMode = () => state.settings.focus_access.attention.focus_mode;
  const layout = () => state.settings.appearance.tab_layout;
  /* Solo focus mode removes the tab strip whichever layout is in use — that is
   * the entire point of "one thing at a time". */
  const tabsHidden = () => layout() === 'hidden' || focusMode() === 'solo';
  const showSidebar = () => layout() === 'sidebar' && !tabsHidden();
  const showTopTabs = () => layout() === 'top' && !tabsHidden();
  const showBookmarks = () =>
    state.settings.appearance.show_bookmarks_bar && focusMode() !== 'solo';

  /* `reconcile` so Solid patches the existing store rather than replacing it:
   * a tab row whose title changed must not remount, or the row you were
   * hovering jumps out from under the pointer. */
  const receive = (next: StateSnapshot) => setState(reconcile(next, { key: 'id' }));

  /* Subscribe *before* asking for the first snapshot, and never let one failed
   * call take the subscription with it.
   *
   * The previous order was `receive(await ipc.getState())` and then `listen`.
   * If that first call rejected — or never settled — the await threw, the
   * listener was never registered, and the chrome stayed on its empty initial
   * store forever: a complete, correct-looking browser with no tabs that
   * ignored every click, because the one line that would have connected it to
   * the core had been skipped. Silent, permanent, and indistinguishable from a
   * frozen UI. Subscribing first means a failed snapshot costs one snapshot. */
  onMount(async () => {
    try {
      const unlisten = await onState(receive);
      onCleanup(() => void unlisten());
    } catch (e) {
      setFatal(
        `Emerald could not subscribe to its own core (${String(e)}). The window will not update.`
      );
      return;
    }
    try {
      receive(await ipc.getState());
    } catch {
      // Already recorded and shown by the failure banner; the listener is live,
      // so the next push from the core will fill the window in anyway.
    }
  });

  createEffect(() => applyTheme(state.settings as Settings));

  /* Page webviews are native widgets layered above the chrome webview, so the
   * palette and the settings panel would render behind the page. Tell the core
   * to stand the pages down while either is open. */
  createEffect(() => void ipc.setOverlay(paletteOpen() || panel() !== null || showFault()));

  /* --- insets -------------------------------------------------------------
   * The core positions page webviews into the rect this element occupies. It
   * is measured rather than computed from settings so that any layout change
   * — sidebar width, focus mode, a future toolbar — is reported without this
   * code having to know about it. */
  onMount(() => {
    if (!contentRef) return;
    const report = () => {
      const r = contentRef!.getBoundingClientRect();
      void ipc.setInsets({
        top: Math.round(r.top),
        left: Math.round(r.left),
        right: Math.round(window.innerWidth - r.right),
        bottom: Math.round(window.innerHeight - r.bottom),
      });
    };
    const ro = new ResizeObserver(report);
    ro.observe(contentRef);
    window.addEventListener('resize', report);
    report();
    onCleanup(() => {
      ro.disconnect();
      window.removeEventListener('resize', report);
    });
  });

  /* --- keyboard -----------------------------------------------------------
   *
   * One implementation, two entry points. A chord pressed while the chrome has
   * focus arrives through the local listener; a chord pressed while a *page*
   * has focus is delivered to that page's webview instead — the chrome is a
   * sibling, not an ancestor, and never sees it — so the content script
   * forwards it and it arrives as `emerald://shortcut`.
   *
   * Both paths call `runShortcut`, so a shortcut cannot work in one context
   * and quietly not the other. */
  const runShortcut = (combo: string) => {
    switch (combo) {
      case 'mod+shift+a':
        // The Focus & Access promise: reachable from anywhere, no clicks.
        return setPanel('attention');
      case 'mod+k':
      case 'mod+p':
        return setPaletteOpen((v) => !v);
      case 'mod+,':
        return setPanel('appearance');
      case 'mod+t':
        return void ipc.openTab('about:blank');
      case 'mod+w': {
        const id = state.active;
        return id != null ? void ipc.closeTab(id) : undefined;
      }
      case 'mod+l':
        return setOmniFocus((n) => n + 1);
      case 'mod+r': {
        const id = state.active;
        return id != null ? void ipc.reloadTab(id) : undefined;
      }
      case 'mod+shift+r': {
        const id = state.active;
        return id != null ? void ipc.toggleReader(id) : undefined;
      }
      case 'mod+\\': {
        if (state.panes.length > 1) return void ipc.unsplit();
        const other = state.tabs.find((t) => t.id !== state.active && t.state !== 'archived');
        return other ? void ipc.splitWith(other.id) : undefined;
      }
      case 'mod+e':
        return cycleFocusMode();
      default: {
        const m = /^mod\+([1-9])$/.exec(combo);
        if (!m) return;
        const visible = state.tabs.filter(
          (t) => t.space === state.active_space && t.state !== 'archived'
        );
        // 9 means "last tab", as it does in every other browser.
        const target = m[1] === '9' ? visible[visible.length - 1] : visible[Number(m[1]) - 1];
        if (target) void ipc.activateTab(target.id);
      }
    }
  };

  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (paletteOpen()) return setPaletteOpen(false);
        if (panel()) return setPanel(null);
        return;
      }
      if (!hasMod(e)) return;
      const combo = `mod${e.shiftKey ? '+shift' : ''}${e.altKey ? '+alt' : ''}+${e.key.toLowerCase()}`;
      const before = combo;
      runShortcut(combo);
      // Only swallow the event for chords Emerald actually claims, so the
      // webview's own editing shortcuts keep working.
      if (CLAIMED.has(before)) e.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    onCleanup(() => window.removeEventListener('keydown', onKey));
  });

  onMount(async () => {
    const unlisten = await listen<string>('emerald://shortcut', (e) => runShortcut(e.payload));
    onCleanup(() => void unlisten());
  });

  const update = (next: Settings) => {
    // Optimistic: the panel's sliders must track the pointer without waiting
    // for a round trip. The core's reply is authoritative and arrives via the
    // state event, clamped.
    setState('settings', next);
    void ipc.setSettings(next);
  };

  const cycleFocusMode = () => {
    const order = ['off', 'dim', 'solo'] as const;
    const now = state.settings.focus_access.attention.focus_mode;
    const next = order[(order.indexOf(now) + 1) % order.length]!;
    update({
      ...(state.settings as Settings),
      focus_access: {
        ...state.settings.focus_access,
        attention: { ...state.settings.focus_access.attention, focus_mode: next },
      },
    });
  };

  return (
    <div
      class="shell"
      data-layout={showSidebar() ? 'sidebar' : 'top'}
      data-focus={focusMode()}
    >
      <Show when={showFault()}>
        <div class="ipc-fault" role="alert">
          <strong>Emerald cannot reach its own core.</strong>
          <p>
            {fatal() ??
              'The window is drawing, but the commands behind it are failing, so nothing you click will take effect.'}
          </p>
          <Show when={failures().length}>
            <ul>
              <For each={failures().slice(-6)}>
                {(f) => (
                  <li>
                    <code>{f.command}</code> — {f.message}
                  </li>
                )}
              </For>
            </ul>
          </Show>
          <p class="ipc-fault-help">
            Please report this with the lines above at{' '}
            <code>github.com/ZDStudios/Emerald/issues</code>. Press F12 for the full console.
          </p>
          <button class="btn" onClick={() => setFaultSeen(true)}>
            Dismiss
          </button>
        </div>
      </Show>

      <Show when={showSidebar()}>
        <Sidebar
          state={state}
          onOpenPanel={setPanel}
          onOpenPalette={() => setPaletteOpen(true)}
        />
      </Show>

      <div class="main">
        <Show when={showTopTabs()}>
          <TopTabs state={state} />
        </Show>
        <Toolbar
          state={state}
          focusRequest={omniFocus()}
          onOpenPalette={() => setPaletteOpen(true)}
          onCycleFocus={cycleFocusMode}
          onOpenPanel={setPanel}
          showMenu={!showSidebar()}
        />
        <UpdateNotice />
        <StoreOffer state={state} />

        <Show when={showBookmarks()}>
          <BookmarksBar state={state} />
        </Show>
        <div class="content" ref={contentRef}>
          <Content state={state} onOpenPanel={setPanel} />
          <Show when={panel()}>
            {(section) => (
              <FocusAccess
                state={state}
                section={section()}
                onSection={setPanel}
                onChange={update}
                onClose={() => setPanel(null)}
              />
            )}
          </Show>
        </div>
      </div>

      <Show when={paletteOpen()}>
        <CommandPalette
          activeTab={activeTab()}
          onClose={() => setPaletteOpen(false)}
          onOpenPanel={setPanel}
        />
      </Show>
    </div>
  );
}
