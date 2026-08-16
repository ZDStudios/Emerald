/*
 * The Chrome Web Store install prompt.
 *
 * Appears while the active tab is sitting on a Web Store listing, offering to
 * install that extension into Emerald. It is drawn by the *chrome*, not
 * injected into the page, and that is the whole security argument: a Web Store
 * listing is still just a web page, and a page that could draw a convincing
 * "Install in Emerald" button could also draw one pointing somewhere else. The
 * id comes from the URL the core reports, never from anything the page says,
 * and the command behind this is chrome-only, so a page cannot reach it.
 *
 * Emerald has no other Web Store integration — no account, no sync, no
 * background updates. This is one download, on one click, and then the
 * extension is an ordinary local folder like any other.
 */

import { createEffect, createSignal, Show } from 'solid-js';
import { ipc, type StateSnapshot } from '../lib/ipc';
import { Close, Puzzle } from '../icons';

interface Props {
  state: StateSnapshot;
}

/** The 32-letter extension id in a Web Store detail URL, or null. */
export function storeExtensionId(url: string | undefined): string | null {
  if (!url) return null;
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return null;
  }
  const host = parsed.hostname;
  const onStore =
    host === 'chromewebstore.google.com' ||
    (host === 'chrome.google.com' && parsed.pathname.startsWith('/webstore'));
  if (!onStore) return null;
  // Both the current and legacy layouts end the detail path with the id.
  const last = parsed.pathname.split('/').filter(Boolean).pop() ?? '';
  return /^[a-p]{32}$/.test(last) ? last : null;
}

export function StoreOffer(props: Props) {
  const [busy, setBusy] = createSignal(false);
  const [done, setDone] = createSignal<string | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [dismissed, setDismissed] = createSignal<string | null>(null);

  const tab = () => props.state.tabs.find((t) => t.id === props.state.active);
  const id = () => storeExtensionId(tab()?.url);
  const supported = () => props.state.settings.extensions.enabled;

  /* A different listing is a different offer — reset rather than carry the
   * previous one's result across. */
  createEffect(() => {
    id();
    setDone(null);
    setError(null);
    setBusy(false);
  });

  const show = () => id() !== null && id() !== dismissed();

  const install = async () => {
    const ext = id();
    if (!ext) return;
    setBusy(true);
    setError(null);
    try {
      const installed = await ipc.installFromStore(ext, tab()?.title ?? undefined);
      setDone(installed.name || installed.id);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Show when={show()}>
      <div class="store-offer" role="region" aria-label="Install extension">
        <Puzzle size={16} />
        <div class="store-offer-text">
          <Show
            when={done()}
            fallback={
              <Show
                when={error()}
                fallback={
                  <>
                    <strong>Install this extension in Emerald?</strong>
                    <span>
                      {supported()
                        ? 'Emerald downloads the package from the Chrome Web Store and keeps it as a local folder. Nothing is synced and it will not update itself.'
                        : 'Extensions are switched off in Settings → Extensions. Emerald will still download and keep this one, but it will not load until you turn them on.'}
                    </span>
                  </>
                }
              >
                <>
                  <strong>That did not install.</strong>
                  <span>{error()}</span>
                </>
              </Show>
            }
          >
            <>
              <strong>Installed {done()}.</strong>
              <span>Settings → Extensions lists what it asked for.</span>
            </>
          </Show>
        </div>

        <Show when={!done()}>
          <button class="btn primary" disabled={busy()} onClick={() => void install()}>
            {busy() ? 'Downloading…' : error() ? 'Try again' : 'Install'}
          </button>
        </Show>

        <button
          class="icon-btn"
          aria-label="Not now"
          onClick={() => setDismissed(id())}
        >
          <Close size={14} />
        </button>
      </div>
    </Show>
  );
}
