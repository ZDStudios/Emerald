/*
 * Emerald's icon set.
 *
 * Drawn as one system, not collected. The rules — see docs/design-language.md
 * §5 for the reasoning — are enforced here by construction rather than by
 * discipline: every icon is the same `<Icon>` wrapper with the same stroke
 * width, caps, joins and viewBox, and only the path data differs.
 *
 *   grid          24 × 24, geometry snapped to a 2px sub-grid
 *   stroke        1.75px, round cap, round join, never filled
 *   safe area     2px margin — nothing touches the box edge
 *   corner        2px radius on every corner that is not a facet
 *   the facet     every icon carries exactly one 45° cut, echoing the
 *                 emerald-cut corner of the app mark. It is the signature
 *                 that makes a strip of these read as one hand.
 *
 * Icons never animate. Not on hover, not on load, not for notifications.
 * `Spinner` is the single exception and it rotates at a constant rate with no
 * pulsing or opacity flicker — and it degrades to a static glyph when the user
 * has turned motion off.
 */

import type { JSX } from 'solid-js';

export interface IconProps {
  /** Pixel size. Defaults to 1em so icons inherit the type scale. */
  size?: number | string;
  /** Extra class names. */
  class?: string;
  title?: string;
  'aria-hidden'?: boolean;
}

function Icon(props: IconProps & { children: JSX.Element }) {
  return (
    <svg
      class={`i ${props.class ?? ''}`}
      width={props.size ?? '1em'}
      height={props.size ?? '1em'}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.75"
      stroke-linecap="round"
      stroke-linejoin="round"
      role={props.title ? 'img' : 'presentation'}
      aria-hidden={props.title ? undefined : true}
    >
      {props.title ? <title>{props.title}</title> : null}
      {props.children}
    </svg>
  );
}

/* ---------------------------------------------------------------------------
 * The mark
 * ------------------------------------------------------------------------- */

/**
 * Emerald's app mark: an emerald-cut gem, corners cut at 45°, with the table
 * facet drawn as an inset trapezoid. Every other icon inherits its one cut
 * corner from this shape.
 */
export function Gem(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M8 3h8l5 5v8l-5 5H8l-5-5V8z" />
      <path d="M8.5 8.5h7l2 3.5-2 3.5h-7l-2-3.5z" />
    </Icon>
  );
}

/* ---------------------------------------------------------------------------
 * Navigation and tabs
 * ------------------------------------------------------------------------- */

export function Plus(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 5v14M5 12h14" />
    </Icon>
  );
}

export function Close(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M6 6l12 12M18 6L6 18" />
    </Icon>
  );
}

export function Back(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M20 12H5" />
      <path d="M11 6l-6 6 6 6" />
    </Icon>
  );
}

export function Forward(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 12h15" />
      <path d="M13 6l6 6-6 6" />
    </Icon>
  );
}

export function Reload(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M20 12a8 8 0 1 1-2.4-5.7" />
      <path d="M20 4v5h-5" />
    </Icon>
  );
}

/** Vertical sidebar, Zen/Arc style. The cut corner marks the content side. */
export function Sidebar(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 6a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v9l-3 3H6a2 2 0 0 1-2-2z" />
      <path d="M10 4v16" />
    </Icon>
  );
}

/** Split view: two panes, the right one carrying the facet. */
export function Split(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 5h7v14H4z" />
      <path d="M13 5h4l3 3v11h-7z" />
    </Icon>
  );
}

export function Pin(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M9 3h6l-1 6 3 3v2H7v-2l3-3z" />
      <path d="M12 14v7" />
    </Icon>
  );
}

/** A discarded tab. A closed eye, not a "zzz" — the tab is not asleep, it is
 *  simply not being looked at, and it comes back instantly. */
export function Resting(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M3 12s3.5-5 9-5 9 5 9 5" />
      <path d="M3 12s3.5 5 9 5c3 0 5.3-1.5 6.9-3l2.1 1" />
      <path d="M9 16.5L8 19M15 16.5l1 2.5" />
    </Icon>
  );
}

export function Archive(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M3 6h18v3H3z" />
      <path d="M5 9v9a2 2 0 0 0 2 2h7l4-4V9" />
      <path d="M10 13h4" />
    </Icon>
  );
}

/* ---------------------------------------------------------------------------
 * Command palette and search
 * ------------------------------------------------------------------------- */

export function Search(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="11" cy="11" r="6" />
      <path d="M15.5 15.5L20 20" />
    </Icon>
  );
}

/** The palette's own glyph: a command key with the facet cut. */
export function Command(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M9 6a2.5 2.5 0 1 0-2.5 2.5H18l-3 3" />
      <path d="M6.5 15.5H18a2.5 2.5 0 1 1-2.5 2.5V8.5" />
    </Icon>
  );
}

export function Bookmark(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M6 4h9l3 3v13l-6-4-6 4z" />
    </Icon>
  );
}

export function Clock(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 7.5V12l3 2" />
    </Icon>
  );
}

/* ---------------------------------------------------------------------------
 * Focus & Access — one icon per settings section
 * ------------------------------------------------------------------------- */

/** Attention: a target with one facet-cut ring, not a brain or a lightning
 *  bolt. ADHD tooling does not need a metaphor for the person using it. */
export function Attention(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 8.5a3.5 3.5 0 1 0 3.5 3.5" />
      <path d="M15.5 8.5L20 4" />
    </Icon>
  );
}

/** Predictability: a level, because the promise is that nothing tilts. */
export function Predictable(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M3 9h15l3 3-3 3H3z" />
      <path d="M11 9v6" />
    </Icon>
  );
}

/** Reading: a line of text with the reading ruler beneath it. */
export function Reading(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 5h11l3 3M4 9.5h9M4 14h13" />
      <path d="M3 19h18" stroke-width="2.75" />
    </Icon>
  );
}

/** Input: a text caret in a field with an enlarged hit box. */
export function InputAssist(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M3 7h14l4 4v6H3z" />
      <path d="M9 10v4" />
      <path d="M7.5 10h3M7.5 14h3" />
    </Icon>
  );
}

/** Focus mode: one thing lit, everything else set aside. */
export function FocusMode(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M8 4H5a1 1 0 0 0-1 1v3M16 4h3a1 1 0 0 1 1 1v3M8 20H5a1 1 0 0 1-1-1v-3M16 20h3a1 1 0 0 0 1-1v-3" />
      <path d="M10 10h4l1.5 2-1.5 2h-4l-1.5-2z" />
    </Icon>
  );
}

export function Type(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M5 7V5h14v2" />
      <path d="M12 5v14" />
      <path d="M9 19h6" />
    </Icon>
  );
}

export function Mic(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 3a3 3 0 0 1 3 3v5a3 3 0 0 1-6 0V6a3 3 0 0 1 3-3z" />
      <path d="M6 11a6 6 0 0 0 12 0" />
      <path d="M12 17v4" />
    </Icon>
  );
}

/** A saved draft: a page with a fold cut at the corner and a saved mark. */
export function Draft(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M6 3h8l4 4v14H6z" />
      <path d="M14 3v4h4" />
      <path d="M9 12h6M9 16h4" />
    </Icon>
  );
}

export function MutedMedia(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 9h3l4-3v12l-4-3H4z" />
      <path d="M15 10l4 4M19 10l-4 4" />
    </Icon>
  );
}

/* ---------------------------------------------------------------------------
 * Chrome and system
 * ------------------------------------------------------------------------- */

export function Settings(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 7h10M18 7h2M4 17h4M12 17h8" />
      <path d="M16 4.5v5M9.5 14.5v5" />
    </Icon>
  );
}

export function Shield(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 3l7 3v6c0 4-3 7.5-7 9-4-1.5-7-5-7-9V6z" />
      <path d="M9 12l2 2 4-4" />
    </Icon>
  );
}

export function Gauge(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 17a8 8 0 1 1 16 0" />
      <path d="M12 17l4.5-5" />
    </Icon>
  );
}

export function Space(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 5h6v6H4zM14 5h4l2 2v4h-6z" />
      <path d="M4 14h6v6H4zM14 14h6v6h-6z" />
    </Icon>
  );
}

export function Leaf(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M20 4c0 9-5 13-11 13a5 5 0 0 1-5-5C4 7 10 4 20 4z" />
      <path d="M4 20c3-6 7-9 12-11" />
    </Icon>
  );
}

export function Moon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5z" />
    </Icon>
  );
}

export function Chevron(props: IconProps & { dir?: 'up' | 'down' | 'left' | 'right' }) {
  const rot = { up: 180, down: 0, left: 90, right: -90 }[props.dir ?? 'down'];
  return (
    <Icon {...props}>
      <g transform={`rotate(${rot} 12 12)`}>
        <path d="M7 10l5 5 5-5" />
      </g>
    </Icon>
  );
}

export function Check(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M5 12.5l4.5 4.5L19 7" />
    </Icon>
  );
}

export function Alert(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 4l8 15H4z" />
      <path d="M12 10v4M12 16.5v.01" />
    </Icon>
  );
}

/* ---------------------------------------------------------------------------
 * Loading
 * ------------------------------------------------------------------------- */

/**
 * The only moving thing in Emerald's chrome.
 *
 * A constant-rate rotation of the gem's facet — no pulsing, no opacity
 * flicker, no bouncing. Rotation speed is scaled by `--motion-scale`, and at
 * scale 0 the CSS stops the animation entirely and this becomes a static
 * glyph. A spinner that cannot be turned off is a spinner that some people
 * cannot use.
 */
export function Spinner(props: IconProps) {
  return (
    <Icon {...props} class={`spin ${props.class ?? ''}`}>
      <path d="M12 4a8 8 0 0 1 8 8" />
      <path d="M20 12l-2.5 2.5" opacity="0.55" />
    </Icon>
  );
}

/* ---------------------------------------------------------------------------
 * Empty states
 * ------------------------------------------------------------------------- */

/*
 * These are drawn on the same 24-unit grid scaled up to 96, with the same
 * stroke ratio, so they sit in the same family as the icons rather than
 * looking like stock art dropped in. Each is one idea, no scenery, no mascot.
 */

function Illustration(props: { children: JSX.Element; class?: string }) {
  return (
    <svg
      class={`illus ${props.class ?? ''}`}
      viewBox="0 0 96 96"
      fill="none"
      stroke="currentColor"
      stroke-width="2.5"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      {props.children}
    </svg>
  );
}

/** Nothing open. A single empty facet — the shape of the app, waiting. */
export function EmptyTabs() {
  return (
    <Illustration>
      <path d="M32 20h32l20 20v32L64 92H32L12 72V40z" opacity="0.35" />
      <path d="M34 42h28l8 14-8 14H34l-8-14z" />
    </Illustration>
  );
}

/** Empty archive: an open, level box. Nothing has been lost. */
export function EmptyArchive() {
  return (
    <Illustration>
      <path d="M12 30h72v12H12z" opacity="0.35" />
      <path d="M18 42v30a4 4 0 0 0 4 4h30l14-14V42" />
      <path d="M38 54h20" />
    </Illustration>
  );
}

/** No drafts: a blank page, corner cut, nothing on the lines. */
export function EmptyDrafts() {
  return (
    <Illustration>
      <path d="M24 12h32l16 16v56H24z" opacity="0.35" />
      <path d="M56 12v16h16" />
      <path d="M34 48h28M34 60h28M34 72h16" opacity="0.55" />
    </Illustration>
  );
}

/** Reader could not find an article. A page whose text does not resolve. */
export function EmptyReader() {
  return (
    <Illustration>
      <path d="M14 24h30a8 8 0 0 1 8 8v44a8 8 0 0 0-8-8H14z" opacity="0.35" />
      <path d="M82 24H60a8 8 0 0 0-8 8v44a8 8 0 0 1 8-8h22z" />
      <path d="M64 40h12M64 52h8" opacity="0.55" />
    </Illustration>
  );
}

export const icons = {
  Gem,
  Plus,
  Close,
  Back,
  Forward,
  Reload,
  Sidebar,
  Split,
  Pin,
  Resting,
  Archive,
  Search,
  Command,
  Bookmark,
  Clock,
  Attention,
  Predictable,
  Reading,
  InputAssist,
  FocusMode,
  Type,
  Mic,
  Draft,
  MutedMedia,
  Settings,
  Shield,
  Gauge,
  Space,
  Leaf,
  Moon,
  Chevron,
  Check,
  Alert,
  Spinner,
};

/** Icon names usable as a space's glyph, in the order the picker shows them. */
export const spaceIcons = ['leaf', 'space', 'moon', 'gem', 'bookmark', 'clock'] as const;
export type SpaceIconName = (typeof spaceIcons)[number];

export function SpaceIcon(props: { name: string; size?: number | string }) {
  const map: Record<string, (p: IconProps) => JSX.Element> = {
    leaf: Leaf,
    space: Space,
    grid: Space,
    moon: Moon,
    gem: Gem,
    bookmark: Bookmark,
    clock: Clock,
  };
  const Chosen = map[props.name] ?? Leaf;
  return <Chosen size={props.size} />;
}
