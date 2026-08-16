/*
 * "A newer Emerald is out."
 *
 * Checked once when the window opens, never on a timer — a browser that polls
 * a server every few minutes for news about itself is doing the thing this
 * project spent §2 arguing against. One request, at the one moment the answer
 * could matter, and only while Settings → Privacy → check for updates is on.
 *
 * It does not install anything by itself. These builds are unsigned, so there
 * is no signature to verify, and nothing should be replacing an executable
 * without a person in the loop. Download puts the installer in your Downloads
 * folder and opens it; from there it is the same installer you would have
 * fetched by hand.
 */

import { createSignal, onMount, Show } from 'solid-js';
import { ipc, type UpdateInfo } from '../lib/ipc';
import { Close, Leaf } from '../icons';

export function UpdateNotice() {
  const [info, setInfo] = createSignal<UpdateInfo | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [saved, setSaved] = createSignal<string | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [dismissed, setDismissed] = createSignal(false);

  onMount(() => {
    // Failure here is not worth a word to the user: not being able to reach
    // GitHub is the normal state of an offline machine, and a browser that
    // complains about it every launch would be worse than one that says
    // nothing. It is still recorded by the IPC failure log.
    void ipc
      .checkForUpdate()
      .then(setInfo)
      .catch(() => {});
  });

  const get = async () => {
    const u = info();
    if (!u?.asset_url || !u.asset_name) return;
    setBusy(true);
    setError(null);
    try {
      // The core saves it and shows it in the file manager. Revealing rather
      // than launching is deliberate: opening an installer unprompted is
      // exactly the move a browser should not make on someone's behalf.
      setSaved(await ipc.downloadUpdate(u.asset_url, u.asset_name));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Show when={info() && !dismissed()}>
      <div class="update-notice" role="status">
        <Leaf size={16} />
        <div class="update-text">
          <strong>
            Emerald {info()!.version} is out — you have {info()!.current}.
          </strong>
          <Show
            when={saved()}
            fallback={
              <Show
                when={error()}
                fallback={
                  <span>
                    {info()!.asset_name
                      ? `Downloads ${info()!.asset_name} and shows it in your files. Nothing installs on its own.`
                      : 'This release has no installer for your platform — the release page has the files.'}
                  </span>
                }
              >
                <span>{error()}</span>
              </Show>
            }
          >
            <span>Saved to {saved()}. Run it when you are ready.</span>
          </Show>
        </div>

        <Show when={info()!.asset_url && !saved()}>
          <button class="btn primary" disabled={busy()} onClick={() => void get()}>
            {busy() ? 'Downloading…' : error() ? 'Try again' : 'Download'}
          </button>
        </Show>

        {/* Opened as a tab rather than handed to the system browser. Emerald
            is a browser; sending its own release notes somewhere else would be
            a strange thing for it to do. */}
        <button class="btn ghost" onClick={() => void ipc.openTab(info()!.url)}>
          Release notes
        </button>

        <button class="icon-btn" aria-label="Not now" onClick={() => setDismissed(true)}>
          <Close size={14} />
        </button>
      </div>
    </Show>
  );
}
