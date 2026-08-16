/**
 * Typed bridge to the Rust core.
 *
 * The core is authoritative for all browser state. The chrome never keeps its
 * own copy of anything that lives in Rust — it renders a snapshot and sends
 * intents back. That is why every mutating call here returns `void` and the UI
 * updates from the `emerald://state` event instead: there is exactly one
 * source of truth and no reconciliation to get wrong.
 */

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Settings } from './settings.gen';

/* --- failure reporting ----------------------------------------------------
 *
 * Every mutating call below is fired with `void`, which means an unhandled
 * rejection: the call fails, the promise is dropped, and the interface simply
 * does not react. That is the worst failure mode this browser can have — the
 * one where the user cannot tell the difference between "nothing happened" and
 * "something broke", and neither can anyone trying to help them. In a release
 * build there is no console to check either.
 *
 * So every failure is recorded here and surfaced in the chrome. It is the same
 * rule the settings copy follows: say what happened. */

export interface IpcFailure {
  command: string;
  message: string;
  at: number;
}

const failures: IpcFailure[] = [];
const watchers = new Set<(f: IpcFailure[]) => void>();

/** Subscribe to IPC failures. Returns an unsubscribe function. */
export function onIpcFailure(handler: (f: IpcFailure[]) => void) {
  watchers.add(handler);
  if (failures.length) handler([...failures]);
  return () => void watchers.delete(handler);
}

/** The failures so far, oldest first. Capped — a broken IPC layer can fail
 *  on every keystroke, and an unbounded log would be its own leak. */
export function ipcFailures() {
  return [...failures];
}

function record(command: string, error: unknown) {
  const message =
    error instanceof Error ? error.message : typeof error === 'string' ? error : String(error);
  failures.push({ command, message, at: Date.now() });
  if (failures.length > 25) failures.shift();
  // Still log: when devtools *are* open this is the fastest way to see it.
  console.error(`emerald: ${command} failed —`, error);
  for (const w of watchers) w([...failures]);
}

function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return tauriInvoke<T>(command, args).catch((e) => {
    record(command, e);
    throw e;
  });
}

export type TabId = number;
export type SpaceId = number;
export type TabState = 'live' | 'discarded' | 'archived';

export interface Tab {
  id: TabId;
  space: SpaceId;
  url: string;
  title: string;
  favicon: string | null;
  state: TabState;
  pinned: boolean;
  last_active_ms: number;
  opened_ms: number;
  scroll_y: number;
  reader: boolean;
  loading: boolean;
}

export interface Pane {
  tab: TabId;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Insets {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export interface Space {
  id: SpaceId;
  name: string;
  icon: string;
  accent: string;
  created_ms: number;
}

export interface Bookmark {
  id: number;
  title: string;
  url: string;
  space: SpaceId;
  added_ms: number;
}

export interface Draft {
  key: string;
  origin: string;
  path: string;
  label: string;
  value: string;
  updated_ms: number;
}

export interface StateSnapshot {
  settings: Settings;
  tabs: Tab[];
  active: TabId | null;
  panes: Pane[];
  spaces: Space[];
  active_space: SpaceId;
  bookmarks: Bookmark[];
  live_count: number;
  archived_count: number;
  draft_count: number;
}

export interface SearchHit {
  kind: 'tab' | 'archived' | 'bookmark' | 'history' | 'draft' | 'setting' | 'action';
  title: string;
  subtitle: string;
  action: 'activate' | 'open' | 'settings' | 'action';
  arg: string;
  score: number;
}

export interface InstalledExtension {
  id: string;
  name: string;
  version: string;
  description: string;
  manifest_version: number;
  permissions: string[];
  host_permissions: string[];
  enabled: boolean;
  path: string;
}

export interface ExtensionState {
  /** False on macOS and Linux, whose engines cannot run Chrome extensions. */
  supported: boolean;
  engine: string;
  directory: string;
  installed: InstalledExtension[];
}

export interface ProcessMemory {
  pid: number;
  name: string;
  rss_kb: number;
  pss_kb: number | null;
}

export interface MemorySample {
  rss_kb: number;
  pss_kb: number | null;
  shell_rss_kb: number;
  processes: ProcessMemory[];
  unsupported: boolean;
}

export const ipc = {
  getState: () => invoke<StateSnapshot>('get_state'),
  setSettings: (settings: Settings) => invoke<StateSnapshot>('set_settings', { settings }),
  resetSettings: () => invoke<StateSnapshot>('reset_settings'),

  openTab: (url: string, space?: SpaceId) => invoke<TabId>('open_tab', { url, space }),
  activateTab: (id: TabId) => invoke<void>('activate_tab', { id }),
  closeTab: (id: TabId) => invoke<void>('close_tab', { id }),
  discardTab: (id: TabId) => invoke<void>('discard_tab', { id }),
  archiveTab: (id: TabId) => invoke<void>('archive_tab', { id }),
  discardAllIdle: () => invoke<number>('discard_all_idle'),
  navigateTab: (id: TabId, url: string) => invoke<void>('navigate_tab', { id, url }),
  reloadTab: (id: TabId) => invoke<void>('reload_tab', { id }),
  back: (id: TabId) => invoke<void>('history_back', { id }),
  forward: (id: TabId) => invoke<void>('history_forward', { id }),
  setPinned: (id: TabId, pinned: boolean) => invoke<void>('set_pinned', { id, pinned }),
  reorderTab: (id: TabId, to: number) => invoke<boolean>('reorder_tab', { id, to }),

  setInsets: (insets: Insets) => invoke<void>('set_insets', { insets }),
  setPanes: (panes: Pane[]) => invoke<void>('set_panes', { panes }),
  /** Page webviews stack above the chrome, so a full-surface overlay needs
   *  them hidden or it opens invisibly. See runtime.rs `Core::overlay`. */
  setOverlay: (active: boolean) => invoke<void>('set_overlay', { active }),
  splitWith: (id: TabId) => invoke<void>('split_with', { id }),
  unsplit: () => invoke<void>('unsplit'),
  toggleReader: (id: TabId) => invoke<boolean>('toggle_reader', { id }),
  setZoom: (id: TabId, factor: number) => invoke<void>('set_zoom', { id, factor }),

  addSpace: (name: string, icon: string, accent: string) =>
    invoke<SpaceId>('add_space', { name, icon, accent }),
  removeSpace: (id: SpaceId) => invoke<boolean>('remove_space', { id }),
  switchSpace: (id: SpaceId) => invoke<void>('switch_space', { id }),

  addBookmark: (title: string, url: string) => invoke<number>('add_bookmark', { title, url }),
  removeBookmark: (id: number) => invoke<boolean>('remove_bookmark', { id }),
  clearHistory: () => invoke<void>('clear_history'),

  search: (query: string, limit?: number) => invoke<SearchHit[]>('search_all', { query, limit }),
  resolveQuery: (input: string) => invoke<string>('resolve_query', { input }),
  recentDrafts: (limit?: number) => invoke<Draft[]>('recent_drafts', { limit }),
  memorySample: () => invoke<MemorySample>('memory_sample'),

  listExtensions: () => invoke<ExtensionState>('list_extensions'),
  installExtension: (path: string) => invoke<InstalledExtension>('install_extension', { path }),
  installFromStore: (id: string, name?: string) =>
    invoke<InstalledExtension>('install_from_store', { id, name }),
  setExtensionEnabled: (id: string, enabled: boolean) =>
    invoke<void>('set_extension_enabled', { id, enabled }),
  removeExtension: (id: string) => invoke<void>('remove_extension', { id }),
};

/** Subscribe to core state pushes. Returns an unlisten function. */
export function onState(handler: (state: StateSnapshot) => void) {
  return listen<StateSnapshot>('emerald://state', (event) => handler(event.payload));
}
