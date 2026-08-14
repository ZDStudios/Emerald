/**
 * Projects settings onto the document.
 *
 * Everything the chrome's CSS needs to know about the user's choices arrives
 * through `:root` data attributes and a handful of custom properties. No
 * component reads settings to decide a colour or a duration — they read tokens,
 * and this function sets the tokens. That keeps the theming surface in one
 * place and means a new component is themed correctly by default.
 */

import type { Settings } from './settings.gen';

export function applyTheme(settings: Settings) {
  const root = document.documentElement;
  const { appearance, focus_access } = settings;

  const theme =
    appearance.theme === 'system'
      ? window.matchMedia('(prefers-color-scheme: light)').matches
        ? 'latte'
        : 'mocha'
      : appearance.theme;

  root.dataset.theme = theme;
  root.dataset.accent = appearance.accent;
  root.dataset.density = appearance.density;
  root.dataset.sensory =
    focus_access.predictability.sensory_profile === 'high_contrast'
      ? 'high-contrast'
      : focus_access.predictability.sensory_profile;

  root.style.setProperty('--ui-scale', String(appearance.ui_font_size_pct / 100));

  // Motion. `transitions: instant` overrides the speed multiplier entirely —
  // choosing "instant" should mean instant, not "fast".
  const instant = focus_access.predictability.transitions === 'instant';
  const scale = instant ? 0 : focus_access.attention.animation_speed;
  root.style.setProperty('--motion-scale', String(scale));
  root.dataset.motion = scale === 0 ? 'off' : 'on';

  root.style.setProperty('--focus-dim', String(focus_access.attention.focus_dim_opacity));

  // Reading settings drive the live preview in the settings panel and the
  // chrome's own prose (help text, empty states), so what you set is what you
  // immediately read.
  const r = focus_access.reading;
  const family =
    r.font === 'open_dyslexic'
      ? 'var(--font-dyslexic)'
      : r.font === 'mono'
        ? 'var(--font-ui)'
        : r.font === 'system'
          ? 'system-ui, sans-serif'
          : 'var(--font-read)';
  root.style.setProperty('--read-family', family);
  root.style.setProperty('--read-leading', String(r.line_height));
  root.style.setProperty('--read-letter', `${r.letter_spacing_em}em`);
  root.style.setProperty('--read-word', `${r.word_spacing_em}em`);
  root.style.setProperty('--read-measure', `${r.max_line_length_ch}ch`);
  root.style.setProperty('--para-space', `${r.paragraph_spacing_em}em`);
}

/** Platform-appropriate modifier label for shortcut hints. */
export const MOD = /mac/i.test(navigator.platform ?? '') ? '⌘' : 'Ctrl';

/** True when the event carries the platform's primary modifier. */
export function hasMod(e: KeyboardEvent) {
  return /mac/i.test(navigator.platform ?? '') ? e.metaKey : e.ctrlKey;
}
