/*
 * Vertical tab strip, Zen/Arc shaped.
 *
 * Vertical is Emerald's default because a horizontal strip degrades as tabs
 * accumulate: titles shrink to nothing and the strip becomes a row of
 * indistinguishable favicons exactly when you most need to tell them apart. A
 * vertical list keeps every title readable at any count, and it makes the tab
 * ceiling legible — you can see the shape of what you have open.
 */

import { For, Show } from 'solid-js';
import { ipc, type StateSnapshot, type Tab } from '../lib/ipc';
import type { PanelSection } from './FocusAccess';
import {
  Archive,
  Close,
  Command,
  Gem,
  Pin,
  Plus,
  Resting,
  Settings,
  SpaceIcon,
  Spinner,
} from '../icons';

interface Props {
  state: StateSnapshot;
  onOpenPanel: (s: PanelSection) => void;
  onOpenPalette: () => void;
}

export function Sidebar(props: Props) {
  const inSpace = () =>
    props.state.tabs.filter(
      (t) => t.space === props.state.active_space && t.state !== 'archived'
    );
  const pinned = () => inSpace().filter((t) => t.pinned);
  const loose = () => inSpace().filter((t) => !t.pinned);
  const badges = () => props.state.settings.focus_access.attention.badges;
  const showArchiveCount = () =>
    props.state.settings.focus_access.attention.show_archive_count &&
    props.state.archived_count > 0;

  return (
    <nav class="sidebar" aria-label="Tabs and spaces">
      <div class="spaces" role="tablist" aria-label="Spaces">
        <For each={props.state.spaces}>
          {(space) => (
            <button
              class="space-chip"
              role="tab"
              aria-current={space.id === props.state.active_space}
              aria-label={`Space: ${space.name}`}
              onClick={() => void ipc.switchSpace(space.id)}
            >
              <SpaceIcon name={space.icon} size={14} />
              <span>{space.name}</span>
            </button>
          )}
        </For>
      </div>

      <div class="tablist" role="tablist" aria-label="Open tabs">
        <Show when={pinned().length > 0}>
          <div class="section-label">Pinned</div>
          <For each={pinned()}>{(tab) => <TabRow tab={tab} state={props.state} />}</For>
          <div class="section-label">Tabs</div>
        </Show>
        <For each={loose()}>{(tab) => <TabRow tab={tab} state={props.state} />}</For>
      </div>

      <div class="sidebar-foot">
        <button class="btn ghost start" onClick={() => void ipc.openTab('about:blank')}>
          <Plus size={16} />
          <span>New tab</span>
        </button>

        <button class="btn ghost start" onClick={props.onOpenPalette}>
          <Command size={16} />
          <span>Search</span>
        </button>

        <Show when={showArchiveCount()}>
          <button
            class="btn ghost start"
            onClick={props.onOpenPalette}
            title="Archived tabs are searchable in the command palette"
          >
            <Archive size={16} />
            <span>Archive</span>
            <Show when={badges() !== 'off'}>
              <span class="kind" style={{ 'margin-inline-start': 'auto' }}>
                {badges() === 'count' ? props.state.archived_count : '•'}
              </span>
            </Show>
          </button>
        </Show>

        {/* One click to Focus & Access, from anywhere in the browser. */}
        <button
          class="btn ghost start"
          onClick={() => props.onOpenPanel('attention')}
          title="Focus & Access — Ctrl/Cmd+Shift+A"
        >
          <Gem size={16} />
          <span>Focus &amp; Access</span>
        </button>

        <button class="btn ghost start" onClick={() => props.onOpenPanel('appearance')}>
          <Settings size={16} />
          <span>Settings</span>
        </button>
      </div>
    </nav>
  );
}

function TabRow(props: { tab: Tab; state: StateSnapshot }) {
  const isActive = () => props.tab.id === props.state.active;
  const inPane = () => props.state.panes.some((p) => p.tab === props.tab.id);

  return (
    <div
      class="tab"
      role="tab"
      tabindex={0}
      aria-current={isActive() || inPane()}
      data-state={props.tab.state}
      onClick={() => void ipc.activateTab(props.tab.id)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          void ipc.activateTab(props.tab.id);
        }
      }}
      onAuxClick={(e) => {
        // Middle click closes, as it does everywhere else.
        if (e.button === 1) void ipc.closeTab(props.tab.id);
      }}
    >
      <Show when={props.tab.loading} fallback={<TabGlyph tab={props.tab} />}>
        <Spinner size={16} />
      </Show>

      <span class="title" title={props.tab.url}>
        {props.tab.title || props.tab.url || 'New tab'}
      </span>

      <Show when={props.tab.pinned}>
        <Pin size={13} />
      </Show>

      <button
        class="close"
        aria-label={`Close ${props.tab.title || 'tab'}`}
        onClick={(e) => {
          e.stopPropagation();
          void ipc.closeTab(props.tab.id);
        }}
      >
        <Close size={13} />
      </button>
    </div>
  );
}

function TabGlyph(props: { tab: Tab }) {
  return (
    <Show
      when={props.tab.state === 'discarded'}
      fallback={
        <Show
          when={props.tab.favicon}
          fallback={<Gem size={16} />}
        >
          {(src) => (
            <img
              class="favicon"
              src={src()}
              alt=""
              /* A missing favicon must not leave a broken-image glyph in the
               * strip; fall back to the house mark. */
              onError={(e) => (e.currentTarget.style.display = 'none')}
            />
          )}
        </Show>
      }
    >
      <Resting size={16} title="Resting — this tab's memory has been released" />
    </Show>
  );
}
