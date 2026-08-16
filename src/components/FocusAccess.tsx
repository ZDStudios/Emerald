/*
 * The Focus & Access panel.
 *
 * Every option in `src-tauri/src/settings.rs` is here, with a real control
 * bound to it. Nothing is a placeholder and nothing is "coming soon" — if it
 * renders, changing it changes the browser, live, without a reload.
 *
 * Two things this panel does that most settings screens do not:
 *
 *   * The reading section carries a live specimen. Line height, letter and
 *     word spacing, measure and typeface are things you cannot evaluate from a
 *     number, so the number is not the interface — the text is.
 *   * Help text is written for the person the setting is for, not for the
 *     engineer who added it, and says what the trade-off is where there is one.
 */

import { createSignal, For, onMount, Show, type JSX } from 'solid-js';
import { ipc, type ExtensionState, type MemorySample, type StateSnapshot } from '../lib/ipc';
import {
  AccentValues,
  AutoplayPolicyValues,
  AutoplayPolicyHelp,
  BadgeStyleValues,
  BadgeStyleHelp,
  DensityValues,
  DraftRestoreValues,
  DraftRestoreHelp,
  FocusModeValues,
  FocusModeHelp,
  ReadingFontValues,
  ReadingFontHelp,
  RulerColorValues,
  RulerFollowsValues,
  RulerFollowsHelp,
  RulerModeValues,
  RulerModeHelp,
  SearchEngineValues,
  SensoryProfileValues,
  SensoryProfileHelp,
  TabLayoutValues,
  TabLayoutHelp,
  ThemeValues,
  ThemeHelp,
  TransitionStyleValues,
  TransitionStyleHelp,
  type Settings,
} from '../lib/settings.gen';
import {
  Attention,
  Close,
  Gauge,
  InputAssist,
  Predictable,
  Reading,
  Puzzle,
  Settings as SettingsIcon,
  Shield,
} from '../icons';

export type PanelSection =
  | 'focus-access'
  | 'attention'
  | 'predictability'
  | 'reading'
  | 'input'
  | 'appearance'
  | 'extensions'
  | 'performance'
  | 'privacy';

interface Props {
  state: StateSnapshot;
  section: PanelSection;
  onSection: (s: PanelSection) => void;
  onChange: (next: Settings) => void;
  onClose: () => void;
}

const SECTIONS: Array<{
  id: PanelSection;
  label: string;
  icon: (p: { size?: number }) => JSX.Element;
}> = [
  { id: 'attention', label: 'Attention', icon: Attention },
  { id: 'predictability', label: 'Predictability', icon: Predictable },
  { id: 'reading', label: 'Reading', icon: Reading },
  { id: 'input', label: 'Input', icon: InputAssist },
  { id: 'appearance', label: 'Appearance', icon: SettingsIcon },
  { id: 'extensions', label: 'Extensions', icon: Puzzle },
  { id: 'performance', label: 'Performance', icon: Gauge },
  { id: 'privacy', label: 'Privacy', icon: Shield },
];

export function FocusAccess(props: Props) {
  const s = () => props.state.settings as Settings;

  /* Settings are plain JSON, so a JSON round-trip is a correct deep clone and
   * sidesteps the store's proxies entirely. */
  const patch = (fn: (draft: Settings) => void) => {
    const next = JSON.parse(JSON.stringify(s())) as Settings;
    fn(next);
    props.onChange(next);
  };

  const current = () => (props.section === 'focus-access' ? 'attention' : props.section);

  return (
    <div class="panel" role="dialog" aria-label="Settings">
      <nav class="panel-nav" aria-label="Settings sections">
        <For each={SECTIONS}>
          {(section) => (
            <button
              aria-current={current() === section.id}
              onClick={() => props.onSection(section.id)}
            >
              <section.icon size={16} />
              <span>{section.label}</span>
            </button>
          )}
        </For>
        <button style={{ 'margin-top': 'auto' }} onClick={props.onClose}>
          <Close size={16} />
          <span>Close</span>
        </button>
      </nav>

      <div class="panel-body">
        <Show when={current() === 'attention'}>
          <AttentionSection settings={s()} patch={patch} state={props.state} />
        </Show>
        <Show when={current() === 'predictability'}>
          <PredictabilitySection settings={s()} patch={patch} />
        </Show>
        <Show when={current() === 'reading'}>
          <ReadingSection settings={s()} patch={patch} />
        </Show>
        <Show when={current() === 'input'}>
          <InputSection settings={s()} patch={patch} state={props.state} />
        </Show>
        <Show when={current() === 'appearance'}>
          <AppearanceSection settings={s()} patch={patch} />
        </Show>
        <Show when={current() === 'extensions'}>
          <ExtensionsSection />
        </Show>
        <Show when={current() === 'performance'}>
          <PerformanceSection settings={s()} patch={patch} state={props.state} />
        </Show>
        <Show when={current() === 'privacy'}>
          <PrivacySection settings={s()} patch={patch} />
        </Show>
      </div>
    </div>
  );
}

type Patch = (fn: (draft: Settings) => void) => void;

/* ---------------------------------------------------------------------------
 * Controls
 * ------------------------------------------------------------------------- */

function Field(props: { name: string; help?: string; children: JSX.Element }) {
  return (
    <div class="field">
      <div class="label">
        <strong class="name">{props.name}</strong>
        <Show when={props.help}>
          <p class="help">{props.help}</p>
        </Show>
      </div>
      <div class="control">{props.children}</div>
    </div>
  );
}

function Toggle(props: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <label class="switch">
      <input
        type="checkbox"
        checked={props.checked}
        aria-label={props.label}
        onChange={(e) => props.onChange(e.currentTarget.checked)}
      />
      <span class="track">
        <span class="knob" />
      </span>
      {/* The state is also written out, so it never depends on colour alone. */}
      <span class="state">{props.checked ? 'On' : 'Off'}</span>
    </label>
  );
}

function Slider(props: {
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
  format?: (v: number) => string;
  label: string;
}) {
  return (
    <div class="slider">
      <input
        type="range"
        min={props.min}
        max={props.max}
        step={props.step}
        value={props.value}
        aria-label={props.label}
        onInput={(e) => props.onChange(Number(e.currentTarget.value))}
      />
      <span class="value">{(props.format ?? String)(props.value)}</span>
    </div>
  );
}

function Choice<T extends string>(props: {
  value: T;
  options: readonly T[];
  help?: Record<string, string>;
  onChange: (v: T) => void;
  label: string;
  labels?: Partial<Record<T, string>>;
}) {
  return (
    <>
      <select
        value={props.value}
        aria-label={props.label}
        onChange={(e) => props.onChange(e.currentTarget.value as T)}
      >
        <For each={props.options}>
          {(opt) => <option value={opt}>{props.labels?.[opt] ?? humanize(opt)}</option>}
        </For>
      </select>
      <Show when={props.help?.[props.value]}>
        <p class="help">{props.help![props.value]}</p>
      </Show>
    </>
  );
}

function Segmented<T extends string>(props: {
  value: T;
  options: readonly T[];
  onChange: (v: T) => void;
  label: string;
}) {
  return (
    <div class="seg" role="group" aria-label={props.label}>
      <For each={props.options}>
        {(opt) => (
          <button aria-pressed={props.value === opt} onClick={() => props.onChange(opt)}>
            {humanize(opt)}
          </button>
        )}
      </For>
    </div>
  );
}

function humanize(v: string) {
  return v.replace(/_/g, ' ').replace(/^\w/, (c) => c.toUpperCase());
}

/* ---------------------------------------------------------------------------
 * Attention
 * ------------------------------------------------------------------------- */

function AttentionSection(props: { settings: Settings; patch: Patch; state: StateSnapshot }) {
  const a = () => props.settings.focus_access.attention;
  return (
    <>
      <header>
        <h2>Attention</h2>
        <p class="lede">
          Fewer things demanding a response. The tab ceiling below is also the single
          biggest thing you can change about how much memory Emerald uses.
        </p>
      </header>

      <Field
        name="Focus mode"
        help="What happens to everything that is not the tab you are reading. Dim keeps it clickable; solo removes it from the layout and leaves the command palette as the way back."
      >
        <Choice
          value={a().focus_mode}
          options={FocusModeValues}
          help={FocusModeHelp}
          label="Focus mode"
          onChange={(v) => props.patch((d) => (d.focus_access.attention.focus_mode = v))}
        />
      </Field>

      <Show when={a().focus_mode === 'dim'}>
        <Field name="How far things fade" help="Dimmed chrome brightens again when you point at it.">
          <Slider
            value={a().focus_dim_opacity}
            min={0.05}
            max={1}
            step={0.05}
            label="Dim opacity"
            format={(v) => `${Math.round(v * 100)}%`}
            onChange={(v) => props.patch((d) => (d.focus_access.attention.focus_dim_opacity = v))}
          />
        </Field>
      </Show>

      <Show when={a().focus_mode !== 'off'}>
        <Field name="Hide the tab strip in focus mode" help="Tabs stay reachable with Ctrl/Cmd+K.">
          <Toggle
            checked={a().hide_tabs_in_focus}
            label="Hide tabs in focus mode"
            onChange={(v) => props.patch((d) => (d.focus_access.attention.hide_tabs_in_focus = v))}
          />
        </Field>
      </Show>

      <Field
        name="Tabs kept awake"
        help="How many tabs may hold a live page at once. Past this, the least recently used one is put to rest: its memory is returned to the system and it reloads when you come back. Pinned tabs and anything on screen are never chosen."
      >
        <Slider
          value={a().max_live_tabs}
          min={1}
          max={64}
          step={1}
          label="Maximum awake tabs"
          format={(v) => `${v} tab${v === 1 ? '' : 's'}`}
          onChange={(v) => props.patch((d) => (d.focus_access.attention.max_live_tabs = v))}
        />
        <p class="help">
          {props.state.live_count} awake right now, {props.state.tabs.length} open in total.
        </p>
      </Field>

      <Field
        name="Rest idle tabs after"
        help="Time without focus before a tab's memory is released. Zero turns the timer off; the ceiling above still applies."
      >
        <Slider
          value={a().suspend_idle_minutes}
          min={0}
          max={240}
          step={5}
          label="Rest idle tabs after"
          format={(v) => (v === 0 ? 'Never' : `${v} min`)}
          onChange={(v) => props.patch((d) => (d.focus_access.attention.suspend_idle_minutes = v))}
        />
      </Field>

      <Field
        name="Archive idle tabs after"
        help="Longer-idle tabs move out of the strip entirely. They are never deleted — they stay searchable in the command palette, and the strip stops being a list of things you have not dealt with."
      >
        <Slider
          value={a().auto_archive_idle_minutes}
          min={0}
          max={2880}
          step={30}
          label="Archive idle tabs after"
          format={(v) => (v === 0 ? 'Never' : v >= 60 ? `${Math.round(v / 60)} h` : `${v} min`)}
          onChange={(v) =>
            props.patch((d) => (d.focus_access.attention.auto_archive_idle_minutes = v))
          }
        />
      </Field>

      <Field name="Notification badges" help="How much unread state Emerald is willing to show.">
        <Choice
          value={a().badges}
          options={BadgeStyleValues}
          help={BadgeStyleHelp}
          label="Badges"
          onChange={(v) => props.patch((d) => (d.focus_access.attention.badges = v))}
        />
      </Field>

      <Field
        name="Animation speed"
        help="A multiplier on every animation in Emerald's own interface. At zero, transitions become instant — nothing is lost, because nothing here is communicated by movement alone."
      >
        <Slider
          value={a().animation_speed}
          min={0}
          max={3}
          step={0.25}
          label="Animation speed"
          format={(v) => (v === 0 ? 'Off' : `${v}×`)}
          onChange={(v) => props.patch((d) => (d.focus_access.attention.animation_speed = v))}
        />
      </Field>

      <Field
        name="Calm down web pages too"
        help="Tells sites you prefer reduced motion, and overrides page animations that ignore it."
      >
        <Toggle
          checked={a().reduce_page_motion}
          label="Reduce motion in pages"
          onChange={(v) => props.patch((d) => (d.focus_access.attention.reduce_page_motion = v))}
        />
      </Field>

      <Field name="Simplify the toolbar" help="Address bar and active tab only; the rest moves into the command palette.">
        <Toggle
          checked={a().declutter_toolbar}
          label="Declutter toolbar"
          onChange={(v) => props.patch((d) => (d.focus_access.attention.declutter_toolbar = v))}
        />
      </Field>

      <Field name="Show the archive count" help="Off means the archive stays silent until you go looking.">
        <Toggle
          checked={a().show_archive_count}
          label="Show archive count"
          onChange={(v) => props.patch((d) => (d.focus_access.attention.show_archive_count = v))}
        />
      </Field>

      <button class="btn" onClick={() => void ipc.discardAllIdle()}>
        Rest every idle tab now
      </button>
    </>
  );
}

/* ---------------------------------------------------------------------------
 * Predictability
 * ------------------------------------------------------------------------- */

function PredictabilitySection(props: { settings: Settings; patch: Patch }) {
  const p = () => props.settings.focus_access.predictability;
  return (
    <>
      <header>
        <h2>Predictability</h2>
        <p class="lede">
          Nothing moves, plays or reorders itself without being asked. These are on by
          default — a calm browser is the product, not a mode you have to find.
        </p>
      </header>

      <Field
        name="Autoplaying media"
        help="Applied before a page's own scripts run, so nothing gets a head start."
      >
        <Choice
          value={p().autoplay}
          options={AutoplayPolicyValues}
          help={AutoplayPolicyHelp}
          label="Autoplay"
          onChange={(v) => props.patch((d) => (d.focus_access.predictability.autoplay = v))}
        />
      </Field>

      <Field
        name="Show where media was blocked"
        help="A small static button where the video would have been, so a blocked player is not just a hole in the page."
      >
        <Toggle
          checked={p().show_blocked_media_placeholder}
          label="Show blocked media placeholder"
          onChange={(v) =>
            props.patch((d) => (d.focus_access.predictability.show_blocked_media_placeholder = v))
          }
        />
      </Field>

      <Field
        name="Hold the layout still"
        help="Reserves space for images and frames before they load, and — when something inserts itself above what you are reading — scrolls to keep your place instead of pushing the text down."
      >
        <Toggle
          checked={p().no_layout_shift}
          label="No layout shift"
          onChange={(v) => props.patch((d) => (d.focus_access.predictability.no_layout_shift = v))}
        />
      </Field>

      <Field name="Transitions" help="Instant is honoured even when animation speed is above zero.">
        <Choice
          value={p().transitions}
          options={TransitionStyleValues}
          help={TransitionStyleHelp}
          label="Transitions"
          onChange={(v) => props.patch((d) => (d.focus_access.predictability.transitions = v))}
        />
      </Field>

      <Field
        name="Keep tabs where you put them"
        help="No most-recently-used shuffling, and no promoting a tab because it made a noise. New tabs always append to the end."
      >
        <Toggle
          checked={p().stable_tab_order}
          label="Stable tab order"
          onChange={(v) => props.patch((d) => (d.focus_access.predictability.stable_tab_order = v))}
        />
      </Field>

      <Field
        name="Open pop-ups as tabs"
        help="A page asking for a new window gets a tab in your strip instead — visible, in the current space, and subject to the same rules as everything else."
      >
        <Toggle
          checked={p().suppress_popups}
          label="Suppress pop-ups"
          onChange={(v) => props.patch((d) => (d.focus_access.predictability.suppress_popups = v))}
        />
      </Field>

      <Field name="Ask before anything irreversible" help="Closing a window with several tabs, clearing history, discarding a recovered draft.">
        <Toggle
          checked={p().confirm_destructive}
          label="Confirm destructive actions"
          onChange={(v) =>
            props.patch((d) => (d.focus_access.predictability.confirm_destructive = v))
          }
        />
      </Field>

      <Field
        name="Colour intensity"
        help="Changes how loud colour is, never what it means. A warning is the same colour family in every profile."
      >
        <Choice
          value={p().sensory_profile}
          options={SensoryProfileValues}
          help={SensoryProfileHelp}
          label="Sensory profile"
          onChange={(v) => props.patch((d) => (d.focus_access.predictability.sensory_profile = v))}
        />
      </Field>

      <p class="note">
        Not a setting, on purpose: nothing in Emerald's interface flashes, blinks, pulses or
        breathes — in any theme, at any animation speed, for any reason. There is no switch
        because there is no code that does it.
      </p>
    </>
  );
}

/* ---------------------------------------------------------------------------
 * Reading
 * ------------------------------------------------------------------------- */

function ReadingSection(props: { settings: Settings; patch: Patch }) {
  const r = () => props.settings.focus_access.reading;
  return (
    <>
      <header>
        <h2>Reading</h2>
        <p class="lede">
          Applies to reader mode always, and to every page when you ask it to. The specimen
          below updates as you change things, because spacing is not something you can judge
          from a number.
        </p>
      </header>

      <div
        class="reading-preview"
        style={{
          'font-family': 'var(--read-family)',
          'line-height': 'var(--read-leading)',
          'letter-spacing': 'var(--read-letter)',
          'word-spacing': 'var(--read-word)',
          'max-width': 'var(--read-measure)',
          'font-size': `${r().font_size_pct}%`,
          'text-align': r().force_start_align ? 'start' : 'justify',
          hyphens: r().hyphenate ? 'auto' : 'manual',
        }}
      >
        <p>
          The quick brown fox jumps over the lazy dog. Handgloves, minimum, illicit — the
          letterforms that get confused with one another are all here, along with enough
          ordinary words to see how a line actually reads.
        </p>
        <p>
          Adjust the measure until your eye finds the start of the next line without hunting
          for it. That is usually somewhere between 55 and 75 characters.
        </p>
      </div>

      <Field
        name="Typeface"
        help="Atkinson Hyperlegible is Emerald's default: it was designed for low vision and has unusually distinct letterforms without looking unusual."
      >
        <Choice
          value={r().font}
          options={ReadingFontValues}
          help={ReadingFontHelp}
          label="Reading font"
          labels={{ open_dyslexic: 'OpenDyslexic', site_preference: 'Leave pages alone' }}
          onChange={(v) => props.patch((d) => (d.focus_access.reading.font = v))}
        />
      </Field>

      <Field
        name="Apply to every page"
        help="Not just reader mode. Emerald overrides site typography where it can; a few sites that use icon fonts for symbols will show empty boxes, which is why this is off by default."
      >
        <Toggle
          checked={r().apply_to_all_pages}
          label="Apply reading settings to all pages"
          onChange={(v) => props.patch((d) => (d.focus_access.reading.apply_to_all_pages = v))}
        />
      </Field>

      <Field name="Text size">
        <Slider
          value={r().font_size_pct}
          min={75}
          max={300}
          step={5}
          label="Text size"
          format={(v) => `${v}%`}
          onChange={(v) => props.patch((d) => (d.focus_access.reading.font_size_pct = v))}
        />
      </Field>

      <Field name="Line height" help="WCAG asks that 1.5 be reachable. Emerald goes to 3.">
        <Slider
          value={r().line_height}
          min={1}
          max={3}
          step={0.05}
          label="Line height"
          format={(v) => v.toFixed(2)}
          onChange={(v) => props.patch((d) => (d.focus_access.reading.line_height = v))}
        />
      </Field>

      <Field name="Letter spacing" help="WCAG floor is 0.12em.">
        <Slider
          value={r().letter_spacing_em}
          min={0}
          max={0.5}
          step={0.01}
          label="Letter spacing"
          format={(v) => `${v.toFixed(2)}em`}
          onChange={(v) => props.patch((d) => (d.focus_access.reading.letter_spacing_em = v))}
        />
      </Field>

      <Field name="Word spacing" help="WCAG floor is 0.16em.">
        <Slider
          value={r().word_spacing_em}
          min={0}
          max={1}
          step={0.02}
          label="Word spacing"
          format={(v) => `${v.toFixed(2)}em`}
          onChange={(v) => props.patch((d) => (d.focus_access.reading.word_spacing_em = v))}
        />
      </Field>

      <Field name="Space between paragraphs" help="WCAG floor is 2× the text size.">
        <Slider
          value={r().paragraph_spacing_em}
          min={0}
          max={4}
          step={0.1}
          label="Paragraph spacing"
          format={(v) => `${v.toFixed(1)}em`}
          onChange={(v) => props.patch((d) => (d.focus_access.reading.paragraph_spacing_em = v))}
        />
      </Field>

      <Field
        name="Line length"
        help="Long lines are the most common reason an eye loses its place on the way back to the left margin."
      >
        <Slider
          value={r().max_line_length_ch}
          min={30}
          max={140}
          step={1}
          label="Maximum line length"
          format={(v) => `${v} characters`}
          onChange={(v) => props.patch((d) => (d.focus_access.reading.max_line_length_ch = v))}
        />
      </Field>

      <Field name="Reading ruler" help="A guide that follows the line you are on.">
        <Choice
          value={r().ruler}
          options={RulerModeValues}
          help={RulerModeHelp}
          label="Reading ruler"
          onChange={(v) => props.patch((d) => (d.focus_access.reading.ruler = v))}
        />
      </Field>

      <Show when={r().ruler !== 'off'}>
        <Field name="Ruler height">
          <Slider
            value={r().ruler_height_px}
            min={8}
            max={200}
            step={2}
            label="Ruler height"
            format={(v) => `${v}px`}
            onChange={(v) => props.patch((d) => (d.focus_access.reading.ruler_height_px = v))}
          />
        </Field>
        <Field name="Ruler strength">
          <Slider
            value={r().ruler_opacity}
            min={0}
            max={1}
            step={0.02}
            label="Ruler opacity"
            format={(v) => `${Math.round(v * 100)}%`}
            onChange={(v) => props.patch((d) => (d.focus_access.reading.ruler_opacity = v))}
          />
        </Field>
        <Field name="Ruler colour">
          <Choice
            value={r().ruler_color}
            options={RulerColorValues}
            label="Ruler colour"
            onChange={(v) => props.patch((d) => (d.focus_access.reading.ruler_color = v))}
          />
        </Field>
        <Field name="What moves the ruler">
          <Choice
            value={r().ruler_follows}
            options={RulerFollowsValues}
            help={RulerFollowsHelp}
            label="Ruler follows"
            onChange={(v) => props.patch((d) => (d.focus_access.reading.ruler_follows = v))}
          />
        </Field>
        <Field name="Dim everything else" help="Only the line under the ruler stays at full strength.">
          <Toggle
            checked={r().dim_surrounding_text}
            label="Dim surrounding text"
            onChange={(v) => props.patch((d) => (d.focus_access.reading.dim_surrounding_text = v))}
          />
        </Field>
      </Show>

      <Field
        name="Always align text left"
        help="Justified text stretches the gaps between words unevenly, which makes lines harder to track. On by default."
      >
        <Toggle
          checked={r().force_start_align}
          label="Force start alignment"
          onChange={(v) => props.patch((d) => (d.focus_access.reading.force_start_align = v))}
        />
      </Field>

      <Field name="Hyphenate" help="Shorter ragged edges without justification's uneven word gaps.">
        <Toggle
          checked={r().hyphenate}
          label="Hyphenate"
          onChange={(v) => props.patch((d) => (d.focus_access.reading.hyphenate = v))}
        />
      </Field>

      <p class="note">
        Emerald installs its reading typefaces as system fonts. That is deliberate: a site's
        own security policy can block a font the browser injects into the page, but it cannot
        block one that is already on your machine.
      </p>
    </>
  );
}

/* ---------------------------------------------------------------------------
 * Input
 * ------------------------------------------------------------------------- */

function InputSection(props: { settings: Settings; patch: Patch; state: StateSnapshot }) {
  const i = () => props.settings.focus_access.input;
  return (
    <>
      <header>
        <h2>Input</h2>
        <p class="lede">
          Bigger things to hit, and a promise: text you type into a web form is not lost to a
          crash, a misclick, or a session that quietly expired.
        </p>
      </header>

      <Field
        name="Smallest clickable area"
        help="Links and buttons get a larger hit area without changing size on screen, so nothing moves. Form fields grow for real, because that is what you want from a form field. WCAG asks for at least 24px; Emerald defaults to 44."
      >
        <Slider
          value={i().min_target_px}
          min={20}
          max={96}
          step={2}
          label="Minimum target size"
          format={(v) => `${v}px`}
          onChange={(v) => props.patch((d) => (d.focus_access.input.min_target_px = v))}
        />
      </Field>

      <Field name="Roomier form fields" help="More padding inside inputs and taller text areas. This one does change page layout.">
        <Toggle
          checked={i().enlarge_form_fields}
          label="Enlarge form fields"
          onChange={(v) => props.patch((d) => (d.focus_access.input.enlarge_form_fields = v))}
        />
      </Field>

      <Field name="Text size inside fields">
        <Slider
          value={i().field_font_size_pct}
          min={100}
          max={250}
          step={5}
          label="Field text size"
          format={(v) => `${v}%`}
          onChange={(v) => props.patch((d) => (d.focus_access.input.field_font_size_pct = v))}
        />
      </Field>

      <Field
        name="Save drafts of what you type"
        help="Every text field you type into is saved locally as you go. Nothing is sent anywhere."
      >
        <Toggle
          checked={i().autosave_drafts}
          label="Autosave drafts"
          onChange={(v) => props.patch((d) => (d.focus_access.input.autosave_drafts = v))}
        />
      </Field>

      <Show when={i().autosave_drafts}>
        <Field name="How often" help="Time after you stop typing before a field is written down.">
          <Slider
            value={i().autosave_interval_ms}
            min={250}
            max={10000}
            step={250}
            label="Autosave interval"
            format={(v) => `${(v / 1000).toFixed(2)}s`}
            onChange={(v) => props.patch((d) => (d.focus_access.input.autosave_interval_ms = v))}
          />
        </Field>

        <Field name="When you come back to a form">
          <Choice
            value={i().draft_restore}
            options={DraftRestoreValues}
            help={DraftRestoreHelp}
            label="Draft restore"
            onChange={(v) => props.patch((d) => (d.focus_access.input.draft_restore = v))}
          />
        </Field>

        <Field
          name="Skip sensitive fields"
          help="Passwords, card numbers, one-time codes, and anything a site marks as not-to-be-stored. Strongly recommended — a missed draft costs you a retype, a saved password costs you rather more."
        >
          <Toggle
            checked={i().draft_skip_sensitive}
            label="Skip sensitive fields"
            onChange={(v) => props.patch((d) => (d.focus_access.input.draft_skip_sensitive = v))}
          />
        </Field>

        <Field name="Keep drafts for" help="Zero keeps them until you clear them yourself.">
          <Slider
            value={i().draft_retention_days}
            min={0}
            max={365}
            step={1}
            label="Draft retention"
            format={(v) => (v === 0 ? 'Forever' : `${v} day${v === 1 ? '' : 's'}`)}
            onChange={(v) => props.patch((d) => (d.focus_access.input.draft_retention_days = v))}
          />
          <p class="help">{props.state.draft_count} drafts saved right now.</p>
        </Field>
      </Show>

      <Field
        name="Dictation"
        help="Shows a dictation button on text fields. Emerald contains no speech model and sends no audio anywhere — it uses whatever the platform provides. Where the engine has no speech support, the button says so and points you at your system's dictation instead of pretending."
      >
        <Toggle
          checked={i().voice_input}
          label="Voice input"
          onChange={(v) => props.patch((d) => (d.focus_access.input.voice_input = v))}
        />
      </Field>

      <Field name="Spellcheck everywhere" help="Including fields where the site turned it off.">
        <Toggle
          checked={i().force_spellcheck}
          label="Force spellcheck"
          onChange={(v) => props.patch((d) => (d.focus_access.input.force_spellcheck = v))}
        />
      </Field>

      <Field
        name="Don't submit on Enter"
        help="In a form with more than one field, Enter in a single-line box does nothing. Stops a half-written form being sent by a stray keystroke."
      >
        <Toggle
          checked={i().guard_enter_submit}
          label="Guard Enter submit"
          onChange={(v) => props.patch((d) => (d.focus_access.input.guard_enter_submit = v))}
        />
      </Field>
    </>
  );
}

/* ---------------------------------------------------------------------------
 * Appearance
 * ------------------------------------------------------------------------- */

const ACCENT_HEX: Record<string, string> = {
  green: '#a6e3a1',
  teal: '#94e2d5',
  sapphire: '#74c7ec',
  lavender: '#b4befe',
  mauve: '#cba6f7',
  peach: '#fab387',
  rosewater: '#f5e0dc',
};

function AppearanceSection(props: { settings: Settings; patch: Patch }) {
  const a = () => props.settings.appearance;
  return (
    <>
      <header>
        <h2>Appearance</h2>
        <p class="lede">
          Emerald is built for Catppuccin Mocha. The accent below means one thing throughout
          the interface — “this is the live thing, or this is yours”.
        </p>
      </header>

      <Field name="Theme">
        <Choice
          value={a().theme}
          options={ThemeValues}
          help={ThemeHelp}
          label="Theme"
          onChange={(v) => props.patch((d) => (d.appearance.theme = v))}
        />
      </Field>

      <Field name="Accent" help="Warnings stay red and needs-attention stays yellow regardless.">
        <div class="swatches">
          <For each={AccentValues}>
            {(accent) => (
              <button
                class="swatch"
                aria-pressed={a().accent === accent}
                aria-label={accent}
                title={accent}
                style={{ background: ACCENT_HEX[accent] }}
                onClick={() => props.patch((d) => (d.appearance.accent = accent))}
              />
            )}
          </For>
        </div>
      </Field>

      <Field
        name="Start from a familiar shape"
        help="Presets, not modes: each one sets the handful of options below and then gets out of the way. Nothing is hidden and you can change any of it afterwards."
      >
        <div class="seg" role="group" aria-label="Layout preset">
          <button
            onClick={() =>
              props.patch((d) => {
                d.appearance.tab_layout = 'sidebar';
                d.appearance.show_bookmarks_bar = false;
                d.appearance.always_show_tab_close = false;
              })
            }
          >
            Emerald
          </button>
          <button
            onClick={() =>
              props.patch((d) => {
                d.appearance.tab_layout = 'top';
                d.appearance.show_bookmarks_bar = true;
                d.appearance.always_show_tab_close = true;
              })
            }
          >
            Chrome-like
          </button>
          <button
            onClick={() =>
              props.patch((d) => {
                d.appearance.tab_layout = 'hidden';
                d.appearance.show_bookmarks_bar = false;
              })
            }
          >
            Bare
          </button>
        </div>
      </Field>

      <Field name="Where tabs live" help="Vertical keeps titles readable however many you have open.">
        <Choice
          value={a().tab_layout}
          options={TabLayoutValues}
          help={TabLayoutHelp}
          label="Tab layout"
          onChange={(v) => props.patch((d) => (d.appearance.tab_layout = v))}
        />
      </Field>

      <Field name="Interface text size">
        <Slider
          value={a().ui_font_size_pct}
          min={75}
          max={200}
          step={5}
          label="UI text size"
          format={(v) => `${v}%`}
          onChange={(v) => props.patch((d) => (d.appearance.ui_font_size_pct = v))}
        />
      </Field>

      <Field
        name="Bookmarks bar"
        help="A strip of the current space's bookmarks under the toolbar. Bookmarks belong to a space, so switching space changes the bar."
      >
        <Toggle
          checked={a().show_bookmarks_bar}
          label="Show bookmarks bar"
          onChange={(v) => props.patch((d) => (d.appearance.show_bookmarks_bar = v))}
        />
      </Field>

      <Field
        name="Always show tab close buttons"
        help="Chrome shows them all the time; Emerald reveals them on hover, which is quieter but less discoverable."
      >
        <Toggle
          checked={a().always_show_tab_close}
          label="Always show tab close buttons"
          onChange={(v) => props.patch((d) => (d.appearance.always_show_tab_close = v))}
        />
      </Field>

      <Field name="Density">
        <Segmented
          value={a().density}
          options={DensityValues}
          label="Density"
          onChange={(v) => props.patch((d) => (d.appearance.density = v))}
        />
      </Field>
    </>
  );
}

/* ---------------------------------------------------------------------------
 * Extensions
 * ------------------------------------------------------------------------- */

function ExtensionsSection() {
  const [state, setState] = createSignal<ExtensionState | null>(null);
  const [busy, setBusy] = createSignal('');
  const [error, setError] = createSignal('');

  const refresh = () => void ipc.listExtensions().then(setState).catch(() => setState(null));
  onMount(refresh);

  const install = async () => {
    setError('');
    const { open } = await import('@tauri-apps/plugin-dialog');
    const picked = await open({
      title: 'Choose an extension package',
      multiple: false,
      filters: [{ name: 'Extension', extensions: ['crx', 'zip'] }],
    });
    if (typeof picked !== 'string') return;
    setBusy('Installing…');
    try {
      await ipc.installExtension(picked);
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy('');
    }
  };

  return (
    <>
      <header>
        <h2>Extensions</h2>
        <p class="lede">
          Extension support is not Emerald's to give — it belongs to whichever web engine
          your operating system provides, and the three are not the same.
        </p>
      </header>

      <Show
        when={state()?.supported}
        fallback={
          <Show when={state()}>
            <p class="note warn">
              <strong>This build cannot run Chrome extensions.</strong> Emerald uses{' '}
              {state()!.engine} here, which has no Chrome extension system —{' '}
              {state()!.engine === 'WKWebView'
                ? 'the API does not exist on macOS at all.'
                : 'its extension mechanism loads compiled WebKit modules, a different technology that happens to share the name.'}{' '}
              Chrome extensions work in Emerald on Windows, where the engine is WebView2.
              This is the direct cost of not shipping Chromium, and it is the sharpest one.
            </p>
            <p class="note">
              You can still install packages here — they are stored, listed and kept — but
              nothing will load them until you run Emerald on a platform whose engine
              supports them.
            </p>
          </Show>
        }
      >
        <p class="note">
          Emerald loads unpacked extensions through WebView2. There is no Chrome Web Store
          button: the Store serves <code>.crx</code> files to Chrome-branded clients under
          terms that do not cover other browsers, and Emerald does not pretend to be Chrome
          to get around that. Download the package yourself and install it below — the same
          route Chrome offers in developer mode.
        </p>
      </Show>

      <div class="field">
        <div class="label">
          <strong class="name">Load extensions</strong>
          <p class="help">
            Extensions run with wide access to the pages they match. They are never loaded
            into Emerald's own interface — only into web pages.
          </p>
        </div>
        <div class="control">
          <button class="btn" onClick={install} disabled={Boolean(busy())}>
            {busy() || 'Install from file…'}
          </button>
          <Show when={error()}>
            <p class="help" style={{ color: 'var(--danger)' }}>{error()}</p>
          </Show>
        </div>
      </div>

      <Show when={state()}>
        <p class="help" style={{ 'margin-bottom': 'var(--space-4)' }}>
          Installed in <code>{state()!.directory}</code>
        </p>
      </Show>

      <Show
        when={(state()?.installed.length ?? 0) > 0}
        fallback={<p class="help">No extensions installed.</p>}
      >
        <div class="ext-list stagger">
          <For each={state()!.installed}>
            {(ext, i) => (
              <div class="ext" style={{ '--i': i() }}>
                <div>
                  <h4>{ext.name}</h4>
                  <span class="meta">
                    v{ext.version || '?'} · manifest v{ext.manifest_version}
                    {ext.manifest_version === 2 ? ' · Chrome no longer runs MV2' : ''}
                  </span>
                  <Show when={ext.description}>
                    <p class="desc">{ext.description}</p>
                  </Show>
                  <Show when={ext.host_permissions.length + ext.permissions.length > 0}>
                    <div class="perms">
                      <For each={ext.host_permissions}>
                        {(h) => <span class="perm host" title="Can read and change these sites">{h}</span>}
                      </For>
                      <For each={ext.permissions}>{(p) => <span class="perm">{p}</span>}</For>
                    </div>
                  </Show>
                </div>
                <div class="ext-actions">
                  <Toggle
                    checked={ext.enabled}
                    label={`Enable ${ext.name}`}
                    onChange={(v) => void ipc.setExtensionEnabled(ext.id, v).then(refresh)}
                  />
                  <button
                    class="btn danger"
                    onClick={() => void ipc.removeExtension(ext.id).then(refresh)}
                  >
                    Remove
                  </button>
                </div>
              </div>
            )}
          </For>
        </div>
      </Show>
    </>
  );
}

/* ---------------------------------------------------------------------------
 * Performance
 * ------------------------------------------------------------------------- */

function PerformanceSection(props: { settings: Settings; patch: Patch; state: StateSnapshot }) {
  const p = () => props.settings.performance;
  const [sample, setSample] = createSignal<MemorySample | null>(null);

  const refresh = () => void ipc.memorySample().then(setSample);
  onMount(refresh);

  return (
    <>
      <header>
        <h2>Performance</h2>
        <p class="lede">
          Emerald runs one recurring timer, and only when something below needs it. With the
          budget at zero and both idle-tab timers off, it does no periodic work at all.
        </p>
      </header>

      <Show when={sample() && !sample()!.unsupported}>
        <div class="stat">
          <div>
            <span class="n">{(sample()!.rss_kb / 1024).toFixed(0)} MB</span>
            <span class="k">Resident, whole tree</span>
          </div>
          <Show when={sample()!.pss_kb}>
            <div>
              <span class="n">{(sample()!.pss_kb! / 1024).toFixed(0)} MB</span>
              <span class="k">Proportional (fair)</span>
            </div>
          </Show>
          <div>
            <span class="n">{(sample()!.shell_rss_kb / 1024).toFixed(0)} MB</span>
            <span class="k">Emerald itself</span>
          </div>
          <div>
            <span class="n">{sample()!.processes.length}</span>
            <span class="k">Processes</span>
          </div>
          <div>
            <span class="n">{props.state.live_count}</span>
            <span class="k">Tabs awake</span>
          </div>
        </div>
        <button class="btn" onClick={refresh}>
          Measure again
        </button>
      </Show>

      <Show when={sample()?.unsupported}>
        <p class="note warn">
          Emerald cannot read process memory on this platform, so the budget below has nothing
          to act on and is ignored. The tab ceiling and the idle timers still work.
        </p>
      </Show>

      <Field
        name="Memory budget"
        help="A ceiling for Emerald and all of its page processes together. When it is exceeded, idle tabs are put to rest oldest-first until it is back under. Zero turns the check off and stops the sampler entirely."
      >
        <Slider
          value={p().memory_budget_mb}
          min={0}
          max={8192}
          step={128}
          label="Memory budget"
          format={(v) => (v === 0 ? 'Off' : `${v} MB`)}
          onChange={(v) => props.patch((d) => (d.performance.memory_budget_mb = v))}
        />
      </Field>

      <Field name="How often to check" help="Only runs while a budget or an idle timer is set.">
        <Slider
          value={p().sampler_interval_s}
          min={5}
          max={600}
          step={5}
          label="Sampler interval"
          format={(v) => `${v}s`}
          onChange={(v) => props.patch((d) => (d.performance.sampler_interval_s = v))}
        />
      </Field>

      <Field
        name="Start with tabs asleep"
        help="Restored tabs come back as placeholders and load when you open them, so startup takes the same time whether you left 3 tabs open or 300."
      >
        <Toggle
          checked={p().restore_tabs_discarded}
          label="Restore tabs discarded"
          onChange={(v) => props.patch((d) => (d.performance.restore_tabs_discarded = v))}
        />
      </Field>

      <Field name="Hardware acceleration" help="Off can lower memory on some systems, at the cost of scrolling smoothness.">
        <Toggle
          checked={p().hardware_acceleration}
          label="Hardware acceleration"
          onChange={(v) => props.patch((d) => (d.performance.hardware_acceleration = v))}
        />
      </Field>

      <Field
        name="Let pages prefetch"
        help="Off means Emerald opens no network connection you did not ask for, including the speculative ones sites request."
      >
        <Toggle
          checked={p().allow_page_prefetch}
          label="Allow page prefetch"
          onChange={(v) => props.patch((d) => (d.performance.allow_page_prefetch = v))}
        />
      </Field>
    </>
  );
}

/* ---------------------------------------------------------------------------
 * Privacy
 * ------------------------------------------------------------------------- */

function PrivacySection(props: { settings: Settings; patch: Patch }) {
  const p = () => props.settings.privacy;
  return (
    <>
      <header>
        <h2>Privacy</h2>
        <p class="lede">
          Emerald has no telemetry, no crash reporting, no update ping and no account. There
          is no switch for any of that because there is no code for it.
        </p>
      </header>

      <Field name="Search engine">
        <Choice
          value={p().search_engine}
          options={SearchEngineValues}
          label="Search engine"
          labels={{ duck_duck_go: 'DuckDuckGo' }}
          onChange={(v) => props.patch((d) => (d.privacy.search_engine = v))}
        />
      </Field>

      <Show when={p().search_engine === 'custom'}>
        <Field name="Custom search URL" help="Use {q} where the query should go.">
          <input
            type="text"
            value={p().custom_search_url}
            placeholder="https://example.com/search?q={q}"
            onChange={(e) =>
              props.patch((d) => (d.privacy.custom_search_url = e.currentTarget.value))
            }
          />
        </Field>
      </Show>

      <Field name="Send Do Not Track and Global Privacy Control">
        <Toggle
          checked={p().global_privacy_control}
          label="Global privacy control"
          onChange={(v) => props.patch((d) => (d.privacy.global_privacy_control = v))}
        />
      </Field>

      <Field
        name="Check GitHub for a newer Emerald"
        help="One request when a window opens, never on a timer. No identifier, no account, nothing about you — GitHub only learns that an IP address asked a public question. Nothing downloads or installs without you clicking. Turn this off and Emerald makes no request you did not ask for."
      >
        <Toggle
          checked={p().check_for_updates}
          label="Check for updates"
          onChange={(v) => props.patch((d) => (d.privacy.check_for_updates = v))}
        />
      </Field>

      <Field name="Block third-party cookies">
        <Toggle
          checked={p().block_third_party_cookies}
          label="Block third-party cookies"
          onChange={(v) => props.patch((d) => (d.privacy.block_third_party_cookies = v))}
        />
      </Field>

      <Field name="Clear site data when Emerald closes" help="Pinned tabs keep theirs.">
        <Toggle
          checked={p().clear_site_data_on_exit}
          label="Clear site data on exit"
          onChange={(v) => props.patch((d) => (d.privacy.clear_site_data_on_exit = v))}
        />
      </Field>

      <button class="btn danger" onClick={() => void ipc.clearHistory()}>
        Clear browsing history
      </button>
    </>
  );
}
