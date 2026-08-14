/*
 * Horizontal tab strip — the Chrome-shaped layout.
 *
 * Emerald's default is the vertical sidebar, for the reason in Sidebar.tsx: a
 * horizontal strip degrades as tabs accumulate until it is a row of
 * indistinguishable favicons. This layout exists anyway, because "looks like
 * the browser I already know" is a real accessibility property — a familiar
 * shape costs nothing to learn, and asking someone to relearn tab management
 * on day one is its own barrier.
 *
 * What it does not do is pretend the degradation is not happening. Tabs shrink
 * to a floor of 5.5em and then scroll, rather than collapsing to slivers.
 */

import { For, Show } from 'solid-js';
import { ipc, type StateSnapshot, type Tab } from '../lib/ipc';
import { Close, Gem, Pin, Plus, Resting, Spinner } from '../icons';

interface Props {
  state: StateSnapshot;
}

export function TopTabs(props: Props) {
  const inSpace = () =>
    props.state.tabs.filter(
      (t) => t.space === props.state.active_space && t.state !== 'archived'
    );

  return (
    <div class="topstrip" role="tablist" aria-label="Open tabs">
      <div class="topstrip-scroll stagger">
        <For each={inSpace()}>
          {(tab, i) => <TopTab tab={tab} state={props.state} index={i()} />}
        </For>
      </div>
      <button
        class="icon-btn topstrip-new"
        aria-label="New tab"
        onClick={() => void ipc.openTab('about:blank')}
      >
        <Plus size={16} />
      </button>
    </div>
  );
}

function TopTab(props: { tab: Tab; state: StateSnapshot; index: number }) {
  const isActive = () => props.tab.id === props.state.active;
  const alwaysClose = () => props.state.settings.appearance.always_show_tab_close;

  return (
    <div
      class="toptab"
      role="tab"
      tabindex={0}
      style={{ '--i': props.index }}
      aria-current={isActive()}
      data-state={props.tab.state}
      data-close={alwaysClose() ? 'always' : 'hover'}
      title={props.tab.url}
      onClick={() => void ipc.activateTab(props.tab.id)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          void ipc.activateTab(props.tab.id);
        }
      }}
      onAuxClick={(e) => {
        if (e.button === 1) void ipc.closeTab(props.tab.id);
      }}
    >
      <Show
        when={props.tab.loading}
        fallback={
          <Show
            when={props.tab.state === 'discarded'}
            fallback={
              <Show when={props.tab.favicon} fallback={<Gem size={15} />}>
                {(src) => (
                  <img
                    class="favicon"
                    src={src()}
                    alt=""
                    onError={(e) => (e.currentTarget.style.display = 'none')}
                  />
                )}
              </Show>
            }
          >
            <Resting size={15} title="Resting — memory released" />
          </Show>
        }
      >
        <Spinner size={15} />
      </Show>

      <span class="title">{props.tab.title || props.tab.url || 'New tab'}</span>

      <Show when={props.tab.pinned}>
        <Pin size={12} />
      </Show>

      <button
        class="close"
        aria-label={`Close ${props.tab.title || 'tab'}`}
        onClick={(e) => {
          e.stopPropagation();
          void ipc.closeTab(props.tab.id);
        }}
      >
        <Close size={12} />
      </button>
    </div>
  );
}
