/*
 * Top toolbar: navigation and the address bar.
 *
 * Horizontal, above the content, with back/forward/reload on the left — the
 * one place Emerald keeps Chrome's shape rather than Arc's. It is the layout
 * every user already has muscle memory for, and it is the same on every
 * platform. The novelty budget is spent on the sidebar.
 */

import { createEffect, createSignal, Show } from 'solid-js';
import { ipc, type StateSnapshot } from '../lib/ipc';
import { Back, Bookmark, Command, Forward, FocusMode, Gem, Reading, Reload, Settings, Split } from '../icons';
import type { PanelSection } from './FocusAccess';
import { MOD } from '../lib/theme';

interface Props {
  state: StateSnapshot;
  focusRequest: number;
  onOpenPalette: () => void;
  onCycleFocus: () => void;
  onOpenPanel: (s: PanelSection) => void;
  /** True when the sidebar is hidden, so the toolbar has to carry the routes
   *  into settings that normally live in the sidebar footer. Focus & Access
   *  must stay one click away from anywhere — that is a promise, not a
   *  layout detail. */
  showMenu: boolean;
}

export function Toolbar(props: Props) {
  const [draft, setDraft] = createSignal<string | null>(null);
  let input: HTMLInputElement | undefined;

  const tab = () => props.state.tabs.find((t) => t.id === props.state.active);
  const shown = () => draft() ?? displayUrl(tab()?.url ?? '');
  const bookmarked = () =>
    props.state.bookmarks.some((b) => b.url === tab()?.url);

  createEffect(() => {
    // Ctrl/Cmd+L: select the whole address, the way every browser does.
    if (props.focusRequest > 0) {
      input?.focus();
      input?.select();
    }
  });

  /* Clear what was typed when the *tab* changes, and only then.
   *
   * It used to clear on blur, which is wrong in this browser specifically.
   * Page webviews are native widgets that get shown, hidden and repositioned
   * above the chrome whenever the core relayouts, and on Windows that moves
   * focus for reasons the user had nothing to do with. The address bar would
   * blur mid-sentence and silently revert to the current URL — indistinguishable
   * from an address bar that ignores the keyboard, which is exactly how it was
   * reported. Emerald promises elsewhere that nothing is lost mid-task; the
   * address bar is not exempt from that. */
  let lastTab: number | null = null;
  createEffect(() => {
    const id = props.state.active;
    if (id !== lastTab) {
      lastTab = id;
      setDraft(null);
    }
  });

  /** What Enter will do, so the bar can say so before it is pressed. */
  const willSearch = () => {
    const v = draft();
    if (v == null) return false;
    const t = v.trim();
    if (!t) return false;
    if (/^[a-z][a-z0-9+.-]*:/i.test(t)) return false;
    return t.includes(' ') || !/^[^\s/]+\.[^\s/]/.test(t.split('/')[0] ?? t);
  };

  const commit = () => {
    const value = draft();
    const id = props.state.active;
    if (value == null || id == null) return;
    setDraft(null);
    void ipc.navigateTab(id, value);
  };

  return (
    <div class="toolbar">
      <div class="nav">
        <button
          class="icon-btn"
          aria-label="Back"
          disabled={!tab()}
          onClick={() => tab() && void ipc.back(tab()!.id)}
        >
          <Back size={17} />
        </button>
        <button
          class="icon-btn"
          aria-label="Forward"
          disabled={!tab()}
          onClick={() => tab() && void ipc.forward(tab()!.id)}
        >
          <Forward size={17} />
        </button>
        <button
          class="icon-btn"
          aria-label="Reload"
          disabled={!tab()}
          onClick={() => tab() && void ipc.reloadTab(tab()!.id)}
        >
          <Reload size={17} />
        </button>
      </div>

      <div class="omnibox">
        <Show when={tab()?.url?.startsWith('https://')}>
          <span class="scheme" title="Encrypted connection">
            https
          </span>
        </Show>
        <input
          ref={input}
          type="text"
          spellcheck={false}
          autocomplete="off"
          aria-label="Address and search"
          placeholder={`Search or enter address  ·  ${MOD}K for anything`}
          value={shown()}
          onInput={(e) => setDraft(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') commit();
            if (e.key === 'Escape') {
              setDraft(null);
              e.currentTarget.blur();
            }
          }}
        />
        <Show when={willSearch()}>
          <span class="omni-hint" aria-hidden="true">
            search
          </span>
        </Show>
      </div>

      <button
        class="icon-btn"
        aria-label={bookmarked() ? 'Bookmarked' : 'Bookmark this page'}
        aria-pressed={bookmarked()}
        style={bookmarked() ? { color: 'var(--accent)' } : undefined}
        disabled={!tab()}
        onClick={() => {
          const t = tab();
          if (!t) return;
          const existing = props.state.bookmarks.find((b) => b.url === t.url);
          if (existing) void ipc.removeBookmark(existing.id);
          else void ipc.addBookmark(t.title || t.url, t.url);
        }}
      >
        <Bookmark size={17} />
      </button>

      <button
        class="icon-btn"
        aria-label="Reader mode"
        aria-pressed={tab()?.reader ?? false}
        style={tab()?.reader ? { color: 'var(--accent)' } : undefined}
        disabled={!tab()}
        onClick={() => tab() && void ipc.toggleReader(tab()!.id)}
      >
        <Reading size={17} />
      </button>

      <button
        class="icon-btn"
        aria-label="Split view"
        aria-pressed={props.state.panes.length > 1}
        onClick={() => {
          if (props.state.panes.length > 1) return void ipc.unsplit();
          const other = props.state.tabs.find(
            (t) => t.id !== props.state.active && t.state !== 'archived'
          );
          if (other) void ipc.splitWith(other.id);
        }}
      >
        <Split size={17} />
      </button>

      <button
        class="icon-btn"
        aria-label="Focus mode"
        aria-pressed={props.state.settings.focus_access.attention.focus_mode !== 'off'}
        style={
          props.state.settings.focus_access.attention.focus_mode !== 'off'
            ? { color: 'var(--accent)' }
            : undefined
        }
        title={`Focus mode: ${props.state.settings.focus_access.attention.focus_mode} (${MOD}E)`}
        onClick={props.onCycleFocus}
      >
        <FocusMode size={17} />
      </button>

      <button class="icon-btn" aria-label="Command palette" onClick={props.onOpenPalette}>
        <Command size={17} />
      </button>

      <Show when={props.showMenu}>
        <button
          class="icon-btn"
          aria-label="Focus & Access"
          title={`Focus & Access — ${MOD}+Shift+A`}
          onClick={() => props.onOpenPanel('attention')}
        >
          <Gem size={17} />
        </button>
        <button
          class="icon-btn"
          aria-label="Settings"
          onClick={() => props.onOpenPanel('appearance')}
        >
          <Settings size={17} />
        </button>
      </Show>
    </div>
  );
}

/** Trim the noise from a URL without hiding anything that matters. */
function displayUrl(url: string) {
  if (!url || url === 'about:blank') return '';
  return url;
}
