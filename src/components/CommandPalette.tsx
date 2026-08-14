/*
 * Command palette / quick switcher.
 *
 * Searches one index across open tabs, archived tabs, bookmarks, history,
 * recovered drafts, settings sections and browser actions. Ranking happens in
 * Rust (`commands.rs::search_all`) so there is one implementation rather than
 * a Rust one for the address bar and a JS one for here.
 *
 * Keyboard-first: the palette never needs the mouse, and every result row says
 * what kind of thing it is in words rather than relying on an icon colour.
 */

import { createEffect, createSignal, For, onMount, Show } from 'solid-js';
import { ipc, type SearchHit, type Tab } from '../lib/ipc';
import { Search } from '../icons';
import type { PanelSection } from './FocusAccess';

interface Props {
  activeTab: Tab | undefined;
  onClose: () => void;
  onOpenPanel: (s: PanelSection) => void;
}

export function CommandPalette(props: Props) {
  const [query, setQuery] = createSignal('');
  const [hits, setHits] = createSignal<SearchHit[]>([]);
  const [cursor, setCursor] = createSignal(0);
  let input: HTMLInputElement | undefined;
  let listRef: HTMLDivElement | undefined;

  onMount(() => input?.focus());

  createEffect(() => {
    const q = query();
    let cancelled = false;
    void ipc.search(q, 24).then((results) => {
      if (cancelled) return;
      setHits(results);
      setCursor(0);
    });
    return () => {
      cancelled = true;
    };
  });

  const run = (hit: SearchHit | undefined) => {
    if (!hit) return;
    props.onClose();
    switch (hit.action) {
      case 'activate':
        return void ipc.activateTab(Number(hit.arg));
      case 'open':
        return void ipc.openTab(hit.arg);
      case 'settings':
        return props.onOpenPanel(hit.arg as PanelSection);
      case 'action':
        return runAction(hit.arg, props);
    }
  };

  const move = (delta: number) => {
    const list = hits();
    if (list.length === 0) return;
    const next = (cursor() + delta + list.length) % list.length;
    setCursor(next);
    // Keep the selection in view without smooth scrolling, which at speed
    // reads as the list sliding around under you.
    listRef?.children[next]?.scrollIntoView({ block: 'nearest' });
  };

  return (
    <div
      class="scrim"
      onClick={(e) => {
        if (e.target === e.currentTarget) props.onClose();
      }}
    >
      <div class="palette" role="dialog" aria-modal="true" aria-label="Command palette">
        <div class="palette-input">
          <Search size={18} />
          <input
            ref={input}
            type="text"
            spellcheck={false}
            autocomplete="off"
            placeholder="Search tabs, bookmarks, history, settings…"
            value={query()}
            onInput={(e) => setQuery(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === 'ArrowDown') {
                e.preventDefault();
                move(1);
              } else if (e.key === 'ArrowUp') {
                e.preventDefault();
                move(-1);
              } else if (e.key === 'Enter') {
                e.preventDefault();
                const list = hits();
                // Enter on an empty result set navigates to what was typed —
                // the palette doubles as an address bar.
                if (list.length === 0 && query().trim()) {
                  props.onClose();
                  void ipc.openTab(query().trim());
                } else {
                  run(list[cursor()]);
                }
              }
            }}
          />
        </div>

        <div class="palette-results" ref={listRef}>
          <For each={hits()}>
            {(hit, i) => (
              <button
                class="hit"
                data-selected={i() === cursor()}
                onMouseEnter={() => setCursor(i())}
                onClick={() => run(hit)}
              >
                <div class="lines">
                  <div class="h1">{hit.title}</div>
                  <Show when={hit.subtitle}>
                    <div class="h2">{hit.subtitle}</div>
                  </Show>
                </div>
                <span class="kind">{label(hit.kind)}</span>
              </button>
            )}
          </For>
          <Show when={hits().length === 0}>
            <div class="hit" style={{ cursor: 'default' }}>
              <div class="lines">
                <div class="h1">
                  {query().trim() ? `Go to “${query().trim()}”` : 'Type to search'}
                </div>
                <div class="h2">
                  {query().trim() ? 'Enter opens this as an address or a search' : ''}
                </div>
              </div>
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
}

function label(kind: SearchHit['kind']) {
  switch (kind) {
    case 'tab':
      return 'open';
    case 'archived':
      return 'archived';
    case 'bookmark':
      return 'saved';
    case 'history':
      return 'visited';
    case 'draft':
      return 'draft';
    case 'setting':
      return 'setting';
    default:
      return 'action';
  }
}

function runAction(arg: string, props: Props) {
  switch (arg) {
    case 'new-tab':
      return void ipc.openTab('about:blank');
    case 'split':
      return void ipc.splitWith(props.activeTab?.id ?? 0);
    case 'reader':
      return props.activeTab && void ipc.toggleReader(props.activeTab.id);
    case 'discard-idle':
      return void ipc.discardAllIdle();
    case 'drafts':
      return props.onOpenPanel('input');
    case 'archive':
      return props.onOpenPanel('attention');
    default:
      return undefined;
  }
}
