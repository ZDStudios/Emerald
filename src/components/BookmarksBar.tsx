/*
 * Bookmarks bar.
 *
 * Off by default and on in the Chrome-like preset. It is a strip of the
 * bookmarks in the current space — bookmarks belong to a space, so switching
 * from Work to Personal changes the bar, which is the one behaviour here that
 * Chrome does not have and that spaces make obvious.
 *
 * Right-click is not used for removal: a context menu is a discoverability
 * problem and Emerald has a command palette. Each item carries a remove
 * affordance on hover, and the whole set is manageable from Settings.
 */

import { For, Show } from 'solid-js';
import { ipc, type StateSnapshot } from '../lib/ipc';
import { Bookmark as BookmarkIcon, Close } from '../icons';

interface Props {
  state: StateSnapshot;
}

export function BookmarksBar(props: Props) {
  const inSpace = () =>
    props.state.bookmarks.filter((b) => b.space === props.state.active_space);

  return (
    <div class="bookmarks" role="toolbar" aria-label="Bookmarks">
      <Show
        when={inSpace().length > 0}
        fallback={
          <span class="bookmarks-empty">
            No bookmarks in this space yet — the ribbon in the toolbar saves the page you
            are on.
          </span>
        }
      >
        <div class="stagger bookmarks-list">
          <For each={inSpace()}>
            {(bm, i) => (
              <div class="bookmark" style={{ '--i': i() }}>
                <button
                  class="bookmark-open"
                  title={bm.url}
                  onClick={() => void ipc.openTab(bm.url)}
                >
                  <BookmarkIcon size={13} />
                  <span>{bm.title || bm.url}</span>
                </button>
                <button
                  class="bookmark-remove"
                  aria-label={`Remove bookmark ${bm.title}`}
                  onClick={() => void ipc.removeBookmark(bm.id)}
                >
                  <Close size={11} />
                </button>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
