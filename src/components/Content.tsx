/*
 * What shows in the content rect when no live webview is covering it.
 *
 * The core positions page webviews over this element, so anything drawn here
 * is visible exactly when there is no page — no tabs at all, or an active tab
 * whose process has been released. That makes it the natural home for the
 * empty states, and it means a resting tab has somewhere honest to explain
 * itself rather than showing a blank rectangle.
 */

import { Show } from 'solid-js';
import { ipc, type StateSnapshot } from '../lib/ipc';
import { EmptyTabs, Resting } from '../icons';
import type { PanelSection } from './FocusAccess';
import { MOD } from '../lib/theme';

interface Props {
  state: StateSnapshot;
  onOpenPanel: (s: PanelSection) => void;
}

export function Content(props: Props) {
  const active = () => props.state.tabs.find((t) => t.id === props.state.active);
  const resting = () => active()?.state === 'discarded';

  return (
    <Show when={active()} fallback={<NoTabs />}>
      <Show when={resting()}>
        <div class="empty">
          <Resting size={72} />
          <h3>This tab is resting</h3>
          <p>
            Emerald released its memory because it had not been used recently. Everything
            about it — the page, your place on it, anything you had typed — comes back when
            you open it.
          </p>
          <button class="btn primary" onClick={() => void ipc.activateTab(active()!.id)}>
            Wake it up
          </button>
          <p style={{ 'font-size': 'var(--text-sm)' }}>
            You can change when this happens in{' '}
            <button
              class="btn ghost"
              style={{ 'min-height': 'auto', padding: '0 4px' }}
              onClick={() => props.onOpenPanel('attention')}
            >
              Focus &amp; Access → Attention
            </button>
          </p>
        </div>
      </Show>
    </Show>
  );
}

function NoTabs() {
  return (
    <div class="empty">
      <EmptyTabs />
      <h3>Nothing open</h3>
      <p>
        Press <kbd>{MOD}T</kbd> for a new tab, or <kbd>{MOD}K</kbd> to search everything you
        have — open tabs, archived tabs, bookmarks, history and settings.
      </p>
    </div>
  );
}
