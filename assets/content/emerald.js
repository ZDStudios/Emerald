/*
 * Emerald page runtime.
 *
 * Injected into every page webview *before* the page's own scripts, so the
 * autoplay and layout-shift guards are installed before anything can move.
 * No build step, no dependencies, no framework: it has to be small and it has
 * to run synchronously at document-start.
 *
 * Everything here is driven by `globalThis.__EMERALD_CONFIG__`, which
 * src-tauri/src/inject.rs assembles from the Focus & Access settings. Calling
 * `__emerald.apply(cfg)` re-applies live, without a reload — that is what makes
 * dragging a spacing slider reflow the page you are reading underneath the
 * settings panel.
 *
 * All Emerald UI inside the page lives in a closed shadow root, so page CSS
 * cannot restyle it and page scripts cannot read it.
 */
(() => {
  'use strict';
  if (globalThis.__emerald) return;

  let cfg = globalThis.__EMERALD_CONFIG__ || {};
  const invoke = (cmd, args) => {
    try {
      return globalThis.__TAURI_INTERNALS__?.invoke(cmd, args) ?? Promise.resolve(null);
    } catch {
      return Promise.resolve(null);
    }
  };

  const NS = 'emerald';
  const onReady = (fn) =>
    document.readyState === 'loading'
      ? document.addEventListener('DOMContentLoaded', fn, { once: true })
      : fn();

  // ==========================================================================
  // Injected stylesheet
  // ==========================================================================

  let styleEl = null;

  function styleTag() {
    if (styleEl && styleEl.isConnected) return styleEl;
    styleEl = document.createElement('style');
    styleEl.id = `${NS}-a11y`;
    (document.head || document.documentElement).appendChild(styleEl);
    return styleEl;
  }

  /* Text containers only. Overriding `*` would break icon fonts, code
   * ligatures and layout-critical elements, which is exactly the failure mode
   * that makes people turn accessibility features off again. */
  const TEXT_SEL =
    'p, li, dd, dt, blockquote, figcaption, td, th, ' +
    'h1, h2, h3, h4, h5, h6, article, section > span, .content, main';

  function buildCss() {
    const r = cfg.reading || {};
    const i = cfg.input || {};
    const calm = cfg.calm || {};
    const out = [];

    // --- Reading: typography ------------------------------------------------
    // Applied to reader mode always; to the live page only on request, because
    // overriding a site's own type is a real trade and should be a choice.
    if (r.apply_to_all_pages) {
      const decls = [];
      if (r.font_stack) decls.push(`font-family: ${r.font_stack} !important`);
      if (r.letter_spacing_em > 0) decls.push(`letter-spacing: ${r.letter_spacing_em}em !important`);
      if (r.word_spacing_em > 0) decls.push(`word-spacing: ${r.word_spacing_em}em !important`);
      if (r.line_height) decls.push(`line-height: ${r.line_height} !important`);
      if (r.force_start_align) decls.push('text-align: start !important');
      if (r.hyphenate) decls.push('hyphens: auto !important');
      if (decls.length) out.push(`${TEXT_SEL} { ${decls.join('; ')} }`);

      if (r.paragraph_spacing_em > 0) {
        out.push(`p, li, blockquote { margin-block-end: ${r.paragraph_spacing_em}em !important; }`);
      }
      if (r.font_size_pct && r.font_size_pct !== 100) {
        // On :root so `rem`-based layouts scale with the text instead of
        // clipping it.
        out.push(`:root { font-size: ${r.font_size_pct}% !important; }`);
      }
      if (r.max_line_length_ch) {
        out.push(`p, li, blockquote, dd { max-width: ${r.max_line_length_ch}ch !important; }`);
      }
    }

    // --- Input: targets -----------------------------------------------------
    // Two different mechanisms on purpose.
    //
    // For links and buttons the hit area is grown with a centred ::after
    // overlay, which enlarges what you can hit without changing what you can
    // see — no reflow, so the page you learned the shape of stays that shape.
    //
    // Replaced elements (input/select/textarea) cannot carry pseudo-elements,
    // so those get a real `min-height`. Growing form fields is expected and
    // wanted, and `enlarge_form_fields` opts into the rest.
    if (i.min_target_px > 0) {
      const t = i.min_target_px;
      out.push(
        `a, button, summary, label, [role="button"], [role="link"], [role="checkbox"], [role="tab"] {` +
          ` position: relative; }`,
        `a::after, button::after, summary::after, [role="button"]::after, [role="link"]::after,` +
          ` [role="checkbox"]::after, [role="tab"]::after {` +
          ` content: ""; position: absolute; left: 50%; top: 50%;` +
          ` transform: translate(-50%, -50%); width: 100%; height: 100%;` +
          ` min-width: ${t}px; min-height: ${t}px; pointer-events: auto; }`,
        `input:not([type="hidden"]), select, textarea, button {` +
          ` min-height: ${t}px !important; }`,
        `input[type="checkbox"], input[type="radio"] {` +
          ` min-width: ${Math.min(t, 32)}px; min-height: ${Math.min(t, 32)}px; }`
      );
    }
    if (i.enlarge_form_fields) {
      out.push(
        `input:not([type="hidden"]):not([type="checkbox"]):not([type="radio"]), select, textarea {` +
          ` padding: 0.6em 0.75em !important; min-width: 12ch; }`,
        `textarea { min-height: ${Math.max(i.min_target_px * 3, 120)}px !important; }`
      );
    }
    if (i.field_font_size_pct && i.field_font_size_pct !== 100) {
      out.push(
        `input, select, textarea, [contenteditable="true"] {` +
          ` font-size: ${i.field_font_size_pct}% !important; }`
      );
    }

    // --- Calm --------------------------------------------------------------
    // `prefers-reduced-motion` is an OS-level signal that a webview cannot
    // spoof per-tab, so Emerald enforces the intent directly. Durations are
    // near-zero rather than zero so animation/transition *end* events still
    // fire — scripts that wait for them keep working.
    if (calm.reduce_motion) {
      out.push(
        `*, *::before, *::after {` +
          ` animation-duration: 0.001ms !important; animation-iteration-count: 1 !important;` +
          ` transition-duration: 0.001ms !important; scroll-behavior: auto !important; }`
      );
    }
    if (calm.no_layout_shift) {
      out.push(
        // Reserve the box before the pixels arrive.
        `img[width][height] { height: auto; }`,
        `img, video, iframe { max-width: 100%; }`,
        // Late-arriving fonts are a top cause of reflow mid-sentence.
        `@font-face { font-display: optional; }`
      );
    }

    // --- Emerald's own in-page furniture ------------------------------------
    out.push(
      `.${NS}-blocked-media { position: relative; display: inline-block; }`,
      // Nothing here animates, pulses or flashes. Ever.
      `#${NS}-ruler, #${NS}-shade-top, #${NS}-shade-bottom {` +
        ` position: fixed; left: 0; right: 0; pointer-events: none; z-index: 2147483640;` +
        ` will-change: transform; }`
    );

    return out.join('\n');
  }

  function applyCss() {
    styleTag().textContent = buildCss();
  }

  // ==========================================================================
  // Autoplay guard
  // ==========================================================================
  //
  // Installed at document-start by patching the prototype, so it is in place
  // before any page script can call play(). Pages routinely call play() from a
  // load handler; a policy applied later would already have lost.

  let lastGesture = 0;
  const GESTURE_WINDOW_MS = 1000;
  for (const evt of ['pointerdown', 'keydown', 'touchstart']) {
    addEventListener(evt, () => (lastGesture = Date.now()), { capture: true, passive: true });
  }

  function userActivated() {
    if (navigator.userActivation && typeof navigator.userActivation.isActive === 'boolean') {
      return navigator.userActivation.isActive;
    }
    return Date.now() - lastGesture < GESTURE_WINDOW_MS;
  }

  function installAutoplayGuard() {
    const proto = globalThis.HTMLMediaElement && HTMLMediaElement.prototype;
    if (!proto || proto.__emeraldPatched) return;
    const nativePlay = proto.play;
    proto.play = function emeraldPlay(...args) {
      const policy = (cfg.calm && cfg.calm.autoplay) || 'block_all';
      const blocked =
        (policy === 'block_all' && !userActivated()) ||
        (policy === 'block_audio' && !this.muted && !userActivated());
      if (!blocked) return nativePlay.apply(this, args);

      markBlocked(this, nativePlay);
      // Reject like a real autoplay-policy denial, so pages that already
      // handle NotAllowedError take their normal fallback path.
      return Promise.reject(
        new DOMException('Autoplay blocked by Emerald', 'NotAllowedError')
      );
    };
    proto.__emeraldPatched = true;
  }

  function markBlocked(media, nativePlay) {
    if (!(cfg.calm && cfg.calm.show_blocked_media_placeholder)) return;
    if (media.__emeraldBadge) return;
    media.__emeraldBadge = true;
    onReady(() => {
      if (!media.isConnected) return;
      const badge = document.createElement('button');
      badge.type = 'button';
      badge.textContent = '▶ Play';
      badge.setAttribute('aria-label', 'Play media that Emerald held back');
      // Inline styles: the page's CSS must not be able to hide this.
      badge.style.cssText =
        'position:absolute;inset-block-start:8px;inset-inline-start:8px;z-index:2147483641;' +
        'font:500 13px/1 ui-monospace,monospace;padding:8px 12px;border:1px solid #45475a;' +
        'border-radius:8px;background:#1e1e2e;color:#cdd6f4;cursor:pointer;min-height:44px;';
      badge.addEventListener('click', (e) => {
        e.stopPropagation();
        badge.remove();
        nativePlay.call(media);
      });
      const host = media.parentElement;
      if (!host) return;
      if (getComputedStyle(host).position === 'static') host.classList.add(`${NS}-blocked-media`);
      host.appendChild(badge);
    });
  }

  /* Strip `autoplay` before the parser acts on it, and stop preloading media
   * the user never asked for. */
  function tameMediaAttributes(root) {
    const policy = (cfg.calm && cfg.calm.autoplay) || 'block_all';
    if (policy === 'allow') return;
    for (const m of (root.querySelectorAll ? root.querySelectorAll('video, audio') : [])) {
      if (m.autoplay && !(policy === 'block_audio' && m.muted)) {
        m.autoplay = false;
        m.removeAttribute('autoplay');
        if (!m.hasAttribute('preload')) m.preload = 'none';
        if (!m.paused) {
          m.pause();
          markBlocked(m, HTMLMediaElement.prototype.play);
        }
      }
    }
  }

  // ==========================================================================
  // Layout-shift guard (scroll anchoring)
  // ==========================================================================
  //
  // The CSS above reserves space; this handles the rest. When something is
  // inserted or resized *above* what you are reading, the browser normally
  // pushes your text down the screen. Emerald picks the topmost visible block
  // as an anchor and, whenever the document's height changes, scrolls to keep
  // that anchor at the same offset from the top of the viewport.
  //
  // Chromium has `overflow-anchor` for this. WebKit does not, which is exactly
  // why it is implemented here.

  let anchorEl = null;
  let anchorTop = 0;
  let anchorRaf = 0;
  let heightObserver = null;

  function pickAnchor() {
    if (anchorRaf) return;
    anchorRaf = requestAnimationFrame(() => {
      anchorRaf = 0;
      const probeY = Math.min(120, innerHeight * 0.25);
      const candidates = document.elementsFromPoint(innerWidth / 2, probeY) || [];
      const el = candidates.find(
        (n) => n.nodeType === 1 && n !== document.body && n !== document.documentElement
      );
      if (el) {
        anchorEl = el;
        anchorTop = el.getBoundingClientRect().top;
      }
    });
  }

  function restoreAnchor() {
    if (!anchorEl || !anchorEl.isConnected) return;
    const now = anchorEl.getBoundingClientRect().top;
    const drift = now - anchorTop;
    // Ignore sub-pixel noise and implausible jumps (a genuine navigation).
    if (Math.abs(drift) > 1 && Math.abs(drift) < innerHeight * 4) {
      scrollBy(0, drift);
    }
  }

  function installAnchoring() {
    if (!(cfg.calm && cfg.calm.no_layout_shift)) {
      heightObserver?.disconnect();
      heightObserver = null;
      return;
    }
    if (heightObserver) return;
    addEventListener('scroll', pickAnchor, { passive: true });
    onReady(() => {
      pickAnchor();
      heightObserver = new ResizeObserver(() => restoreAnchor());
      heightObserver.observe(document.documentElement);
      if (document.body) heightObserver.observe(document.body);
    });
  }

  // ==========================================================================
  // Reading ruler
  // ==========================================================================

  let ruler = null;
  let shadeTop = null;
  let shadeBottom = null;
  let rulerY = 0;
  let rulerRaf = 0;

  function teardownRuler() {
    for (const n of [ruler, shadeTop, shadeBottom]) n?.remove();
    ruler = shadeTop = shadeBottom = null;
    removeEventListener('pointermove', onRulerPointer);
    removeEventListener('keydown', onRulerKey, true);
  }

  function installRuler() {
    const r = cfg.reading || {};
    const mode = r.ruler || 'off';
    teardownRuler();
    if (mode === 'off') return;

    onReady(() => {
      const host = document.body || document.documentElement;
      const h = r.ruler_height_px || 32;
      const color = r.ruler_css_color || '#a6e3a1';

      if (mode === 'line' || mode === 'band') {
        ruler = document.createElement('div');
        ruler.id = `${NS}-ruler`;
        ruler.style.cssText =
          `position:fixed;left:0;right:0;pointer-events:none;z-index:2147483640;` +
          (mode === 'line'
            ? `height:3px;background:${color};opacity:${Math.max(r.ruler_opacity, 0.5)};`
            : `height:${h}px;background:${color};opacity:${r.ruler_opacity};` +
              `border-block:1px solid ${color};`);
        host.appendChild(ruler);
      } else if (mode === 'spotlight') {
        // Two shades rather than one band: the lit line is simply the gap.
        const shade = (id) => {
          const el = document.createElement('div');
          el.id = id;
          el.style.cssText =
            `position:fixed;left:0;right:0;pointer-events:none;z-index:2147483640;` +
            `background:#11111b;opacity:${Math.min(0.85, r.ruler_opacity * 4 + 0.4)};`;
          host.appendChild(el);
          return el;
        };
        shadeTop = shade(`${NS}-shade-top`);
        shadeBottom = shade(`${NS}-shade-bottom`);
      }

      rulerY = innerHeight / 2;
      const follows = r.ruler_follows || 'pointer';
      if (follows === 'pointer' || follows === 'both') {
        addEventListener('pointermove', onRulerPointer, { passive: true });
      }
      if (follows === 'keyboard' || follows === 'both') {
        addEventListener('keydown', onRulerKey, true);
      }
      drawRuler();
    });
  }

  function onRulerPointer(e) {
    rulerY = e.clientY;
    drawRuler();
  }

  function onRulerKey(e) {
    // Only when nothing is being typed into.
    const t = e.target;
    if (t && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName))) return;
    const step = (cfg.reading?.ruler_height_px || 32) / 2;
    if (e.key === 'ArrowDown') rulerY = Math.min(innerHeight, rulerY + step);
    else if (e.key === 'ArrowUp') rulerY = Math.max(0, rulerY - step);
    else return;
    e.preventDefault();
    drawRuler();
  }

  function drawRuler() {
    if (rulerRaf) return;
    rulerRaf = requestAnimationFrame(() => {
      rulerRaf = 0;
      const h = cfg.reading?.ruler_height_px || 32;
      if (ruler) ruler.style.transform = `translateY(${Math.round(rulerY - h / 2)}px)`;
      if (shadeTop) {
        shadeTop.style.top = '0px';
        shadeTop.style.height = `${Math.max(0, rulerY - h / 2)}px`;
      }
      if (shadeBottom) {
        shadeBottom.style.top = `${rulerY + h / 2}px`;
        shadeBottom.style.height = `${Math.max(0, innerHeight - rulerY - h / 2)}px`;
      }
    });
  }

  // ==========================================================================
  // Form drafts
  // ==========================================================================
  //
  // The promise: nothing typed into a web form is ever lost. Every change to a
  // text field is debounced and written to Emerald's local draft store; a crash,
  // a misclick, an expired session, or a discarded tab all recover.
  //
  // Sensitive fields are excluded by default and the exclusion is deliberately
  // broad — a false positive costs a saved draft, a false negative writes a
  // password to disk.

  const SENSITIVE_TYPES = new Set(['password', 'hidden', 'file', 'image']);
  const SENSITIVE_AC = /(cc-|credit|card|cvc|cvv|one-time-code|current-password|new-password|otp)/i;
  const timers = new WeakMap();

  function isSensitive(el) {
    if (!(cfg.input && cfg.input.draft_skip_sensitive)) return false;
    const type = (el.type || '').toLowerCase();
    if (SENSITIVE_TYPES.has(type)) return true;
    const ac = el.getAttribute('autocomplete') || '';
    if (ac.toLowerCase() === 'off') return true;
    if (SENSITIVE_AC.test(ac)) return true;
    const name = `${el.name || ''} ${el.id || ''}`;
    if (SENSITIVE_AC.test(name)) return true;
    return false;
  }

  function isTextish(el) {
    if (!el || el.nodeType !== 1) return false;
    if (el.isContentEditable) return true;
    if (el.tagName === 'TEXTAREA') return true;
    if (el.tagName !== 'INPUT') return false;
    const type = (el.type || 'text').toLowerCase();
    return ['text', 'search', 'url', 'email', 'tel', 'number', ''].includes(type);
  }

  /** A key that survives reloads: prefer author-stable identifiers. */
  function fieldKey(el) {
    const bits = [
      el.name,
      el.id,
      el.getAttribute('aria-label'),
      el.getAttribute('placeholder'),
    ].filter(Boolean);
    if (bits.length) return `${el.tagName.toLowerCase()}:${bits[0]}`;
    // Positional fallback. Less stable across redesigns, still better than
    // losing the text.
    const peers = document.querySelectorAll(el.tagName);
    return `${el.tagName.toLowerCase()}#${Array.prototype.indexOf.call(peers, el)}`;
  }

  function fieldLabel(el) {
    const explicit = el.labels && el.labels[0] && el.labels[0].innerText;
    return (
      (explicit && explicit.trim()) ||
      el.getAttribute('aria-label') ||
      el.getAttribute('placeholder') ||
      el.name ||
      'Text field'
    ).slice(0, 80);
  }

  function fieldValue(el) {
    return el.isContentEditable ? el.innerText : el.value;
  }

  function onFieldInput(e) {
    const el = e.target;
    if (!isTextish(el) || isSensitive(el)) return;
    if (!(cfg.input && cfg.input.autosave_drafts)) return;

    clearTimeout(timers.get(el));
    timers.set(
      el,
      setTimeout(() => {
        invoke('page_draft', {
          field: fieldKey(el),
          label: fieldLabel(el),
          value: String(fieldValue(el) ?? ''),
        });
      }, cfg.input.autosave_interval_ms || 1500)
    );
  }

  /* A successful submit means the text made it somewhere. Drop the drafts so
   * the restore bar does not reappear on the next visit. */
  function onSubmit() {
    invoke('page_clear_drafts', {});
  }

  function onEnterKey(e) {
    if (!(cfg.input && cfg.input.guard_enter_submit)) return;
    if (e.key !== 'Enter' || e.shiftKey || e.isComposing) return;
    const el = e.target;
    if (!el || el.tagName !== 'INPUT' || !el.form) return;
    const fields = el.form.querySelectorAll('input:not([type="hidden"]), textarea, select');
    if (fields.length > 1) {
      e.preventDefault();
      toast('Enter held back — use the form’s own button to submit.');
    }
  }

  async function offerDrafts() {
    const mode = (cfg.input && cfg.input.draft_restore) || 'ask';
    if (mode === 'off' || !(cfg.input && cfg.input.autosave_drafts)) return;
    const drafts = await invoke('page_drafts', {});
    if (!Array.isArray(drafts) || drafts.length === 0) return;

    const fill = () => {
      let filled = 0;
      for (const d of drafts) {
        for (const el of document.querySelectorAll('input, textarea, [contenteditable="true"]')) {
          if (!isTextish(el) || fieldKey(el) !== d.key) continue;
          // Never clobber text the user has already retyped.
          if (String(fieldValue(el) ?? '').length > 0) continue;
          if (el.isContentEditable) el.innerText = d.value;
          else el.value = d.value;
          el.dispatchEvent(new Event('input', { bubbles: true }));
          filled++;
        }
      }
      return filled;
    };

    if (mode === 'auto') {
      const n = fill();
      if (n) toast(`Restored what you had written in ${n} field${n === 1 ? '' : 's'}.`);
      return;
    }
    restoreBar(drafts, fill);
  }

  // ==========================================================================
  // In-page furniture (closed shadow root)
  // ==========================================================================

  let uiRoot = null;

  function ui() {
    if (uiRoot && uiRoot.host.isConnected) return uiRoot;
    const host = document.createElement('div');
    host.id = `${NS}-ui`;
    host.style.cssText =
      'all: initial; position: fixed; inset-block-end: 0; inset-inline: 0; z-index: 2147483647;';
    (document.body || document.documentElement).appendChild(host);
    uiRoot = host.attachShadow({ mode: 'closed' });
    uiRoot.innerHTML = `<style>
      :host { all: initial; }
      .bar {
        display: flex; align-items: center; gap: 12px;
        margin: 0 auto 16px; max-width: 640px; padding: 12px 14px;
        font: 400 13px/1.5 'JetBrains Mono', ui-monospace, monospace;
        color: #cdd6f4; background: #181825;
        border: 1px solid #313244; border-radius: 10px;
        box-shadow: 0 8px 24px rgb(17 17 27 / 0.55);
      }
      .bar p { margin: 0; flex: 1; }
      .bar button {
        font: inherit; padding: 8px 12px; min-height: 44px;
        border-radius: 8px; border: 1px solid #45475a;
        background: #313244; color: #cdd6f4; cursor: pointer;
      }
      .bar button.primary { background: #a6e3a1; color: #11111b; border-color: #a6e3a1; }
      .bar button:focus-visible { outline: 2px solid #89b4fa; outline-offset: 2px; }
      .k { color: #a6e3a1; }
    </style><div id="slot"></div>`;
    return uiRoot;
  }

  function toast(message) {
    const root = ui();
    const wrap = document.createElement('div');
    wrap.className = 'bar';
    wrap.setAttribute('role', 'status');
    wrap.innerHTML = `<p></p><button type="button">Dismiss</button>`;
    wrap.querySelector('p').textContent = message;
    wrap.querySelector('button').addEventListener('click', () => wrap.remove());
    root.getElementById('slot').appendChild(wrap);
    // No auto-dismiss timer: things that vanish on their own are exactly the
    // kind of surprise Emerald is trying not to be.
  }

  function restoreBar(drafts, fill) {
    const root = ui();
    const wrap = document.createElement('div');
    wrap.className = 'bar';
    wrap.setAttribute('role', 'status');
    const n = drafts.length;
    wrap.innerHTML =
      `<p>You had written something here — <span class="k"></span>.</p>` +
      `<button type="button" class="primary">Restore</button>` +
      `<button type="button">Discard</button>`;
    wrap.querySelector('.k').textContent =
      `${n} field${n === 1 ? '' : 's'}, saved ${relTime(Math.max(...drafts.map((d) => d.updated_ms)))}`;
    const [restore, discard] = wrap.querySelectorAll('button');
    restore.addEventListener('click', () => {
      fill();
      wrap.remove();
    });
    discard.addEventListener('click', () => {
      invoke('page_clear_drafts', {});
      wrap.remove();
    });
    root.getElementById('slot').appendChild(wrap);
  }

  function relTime(ms) {
    const s = Math.max(0, (Date.now() - ms) / 1000);
    if (s < 90) return 'just now';
    if (s < 5400) return `${Math.round(s / 60)} min ago`;
    if (s < 172800) return `${Math.round(s / 3600)} h ago`;
    return `${Math.round(s / 86400)} days ago`;
  }

  // ==========================================================================
  // Dictation
  // ==========================================================================
  //
  // Honest note: WebKitGTK ships no SpeechRecognition implementation, so on
  // Linux this affordance reports that dictation is unavailable and points at
  // the OS instead of pretending. Emerald contains no speech model and sends
  // no audio anywhere itself. See docs/architecture.md §7.

  let micButton = null;
  let recognition = null;

  function speechAvailable() {
    return Boolean(globalThis.SpeechRecognition || globalThis.webkitSpeechRecognition);
  }

  function installDictation() {
    micButton?.remove();
    micButton = null;
    removeEventListener('focusin', onFocusForDictation, true);
    if (cfg.input && cfg.input.voice_input) {
      addEventListener('focusin', onFocusForDictation, true);
    }
  }

  function onFocusForDictation(e) {
    const el = e.target;
    micButton?.remove();
    if (!isTextish(el)) return;

    const rect = el.getBoundingClientRect();
    micButton = document.createElement('button');
    micButton.type = 'button';
    micButton.textContent = speechAvailable() ? '🎙 Dictate' : '🎙 System dictation';
    micButton.title = speechAvailable()
      ? 'Dictate into this field'
      : 'This engine has no built-in speech recognition — use your OS dictation shortcut';
    micButton.style.cssText =
      `position:fixed;top:${Math.max(4, rect.top - 40)}px;left:${rect.left}px;z-index:2147483646;` +
      `font:500 12px/1 'JetBrains Mono',ui-monospace,monospace;padding:8px 10px;min-height:36px;` +
      `border:1px solid #45475a;border-radius:8px;background:#181825;color:#cdd6f4;cursor:pointer;`;
    micButton.addEventListener('mousedown', (ev) => ev.preventDefault()); // keep focus
    micButton.addEventListener('click', () => startDictation(el));
    (document.body || document.documentElement).appendChild(micButton);
  }

  function startDictation(el) {
    const Impl = globalThis.SpeechRecognition || globalThis.webkitSpeechRecognition;
    if (!Impl) {
      toast(
        'This build’s web engine has no built-in speech recognition. Your operating ' +
          'system’s dictation still works in this field.'
      );
      return;
    }
    if (recognition) {
      recognition.stop();
      recognition = null;
      return;
    }
    recognition = new Impl();
    recognition.interimResults = true;
    recognition.continuous = true;
    const base = String(fieldValue(el) ?? '');
    recognition.onresult = (ev) => {
      let text = '';
      for (let i = ev.resultIndex; i < ev.results.length; i++) text += ev.results[i][0].transcript;
      const joined = base ? `${base} ${text}` : text;
      if (el.isContentEditable) el.innerText = joined;
      else el.value = joined;
      el.dispatchEvent(new Event('input', { bubbles: true }));
    };
    recognition.onend = () => {
      recognition = null;
      if (micButton) micButton.textContent = '🎙 Dictate';
    };
    recognition.start();
    if (micButton) micButton.textContent = '■ Stop';
  }

  // ==========================================================================
  // Reader mode
  // ==========================================================================
  //
  // A deliberately small extractor: score block containers by the amount of
  // text they hold that is not inside links, which separates prose from
  // navigation without a 100KB dependency. It handles articles and docs well
  // and gives up gracefully on apps.
  //
  // The article is rendered into an overlay rather than by replacing the
  // document, so leaving reader mode restores the real page exactly.

  let readerHost = null;

  function extractArticle() {
    const candidates = document.querySelectorAll('article, main, [role="main"], div, section');
    let best = null;
    let bestScore = 0;
    for (const el of candidates) {
      const text = el.innerText || '';
      if (text.length < 250) continue;
      const linkText = Array.from(el.querySelectorAll('a'))
        .reduce((n, a) => n + (a.innerText || '').length, 0);
      const density = 1 - Math.min(1, linkText / text.length);
      const paragraphs = el.querySelectorAll('p').length;
      // Prose wins on length × link-sparsity × paragraph count; nested
      // containers are penalised so we pick the tightest wrapper, not <body>.
      const depth = depthOf(el);
      const score = text.length * density * Math.log2(paragraphs + 2) - depth * 200;
      if (score > bestScore) {
        bestScore = score;
        best = el;
      }
    }
    return best;
  }

  function depthOf(el) {
    let d = 0;
    for (let n = el; n; n = n.parentElement) d++;
    return d;
  }

  function readerOn() {
    if (readerHost) return;
    const article = extractArticle();
    if (!article) {
      toast('Emerald could not find an article on this page.');
      return;
    }
    const r = cfg.reading || {};
    const host = document.createElement('div');
    host.id = `${NS}-reader`;
    host.style.cssText = 'all: initial; position: fixed; inset: 0; z-index: 2147483646;';
    const shadow = host.attachShadow({ mode: 'closed' });
    const title = (document.querySelector('h1') || {}).innerText || document.title;

    shadow.innerHTML = `<style>
      :host { all: initial; }
      .page {
        position: fixed; inset: 0; overflow-y: auto;
        background: #1e1e2e; color: #cdd6f4;
        font-size: ${r.font_size_pct || 100}%;
      }
      .col {
        max-width: ${r.max_line_length_ch || 68}ch;
        margin: 0 auto; padding: 72px 24px 160px;
        font-family: ${r.font_stack || 'system-ui, sans-serif'};
        line-height: ${r.line_height || 1.6};
        letter-spacing: ${r.letter_spacing_em || 0}em;
        word-spacing: ${r.word_spacing_em || 0}em;
        text-align: ${r.force_start_align ? 'start' : 'inherit'};
        hyphens: ${r.hyphenate ? 'auto' : 'manual'};
      }
      .col :is(p, li, blockquote) { margin-block-end: ${r.paragraph_spacing_em || 1}em; }
      .col h1 { font-size: 1.75em; line-height: 1.25; margin-block-end: 0.5em; }
      .col a { color: #89b4fa; }
      .col img, .col figure { max-width: 100%; height: auto; }
      .col pre { overflow-x: auto; padding: 12px; background: #181825; border-radius: 8px; }
      .exit {
        position: fixed; inset-block-start: 16px; inset-inline-end: 16px;
        font: 500 13px/1 'JetBrains Mono', ui-monospace, monospace;
        padding: 10px 14px; min-height: 44px;
        background: #313244; color: #cdd6f4;
        border: 1px solid #45475a; border-radius: 8px; cursor: pointer;
      }
      .exit:focus-visible { outline: 2px solid #89b4fa; outline-offset: 2px; }
    </style>
    <div class="page"><button class="exit" type="button">Close reader (Esc)</button>
      <article class="col"><h1></h1><div id="body"></div></article></div>`;

    shadow.querySelector('h1').textContent = title;
    // Cloned, so nothing in the reader can mutate the live page.
    shadow.getElementById('body').append(article.cloneNode(true));
    shadow.querySelector('.exit').addEventListener('click', readerOff);
    (document.body || document.documentElement).appendChild(host);
    readerHost = host;
    addEventListener('keydown', readerEsc, true);
  }

  function readerEsc(e) {
    if (e.key === 'Escape') readerOff();
  }

  function readerOff() {
    readerHost?.remove();
    readerHost = null;
    removeEventListener('keydown', readerEsc, true);
  }

  // ==========================================================================
  // Browser shortcuts
  // ==========================================================================
  //
  // In a multi-webview browser the chrome is a sibling of the page, not its
  // parent, so a keydown while you are reading a page is delivered to the page
  // and the chrome's own listener never runs. Without this, every shortcut —
  // including the command palette Emerald calls keyboard-first — would only
  // work when focus happened to be in the sidebar. Which is almost never.
  //
  // So each page forwards the specific chords Emerald owns, and nothing else.
  // The allowlist is deliberately short and explicit: a page's own shortcuts
  // must keep working, so anything not listed here is left entirely alone.

  const OWNED = new Set([
    'mod+k', 'mod+p', 'mod+t', 'mod+w', 'mod+l', 'mod+r',
    'mod+shift+r', 'mod+shift+a', 'mod+e', 'mod+\\', 'mod+,',
    'mod+1', 'mod+2', 'mod+3', 'mod+4', 'mod+5',
    'mod+6', 'mod+7', 'mod+8', 'mod+9',
  ]);

  const isMac = /mac/i.test(navigator.platform || '');

  function comboOf(e) {
    const mod = isMac ? e.metaKey : e.ctrlKey;
    if (!mod) return null;
    let combo = 'mod';
    if (e.shiftKey) combo += '+shift';
    if (e.altKey) combo += '+alt';
    const key = e.key.length === 1 ? e.key.toLowerCase() : e.key.toLowerCase();
    return `${combo}+${key}`;
  }

  function onShortcut(e) {
    const combo = comboOf(e);
    if (!combo || !OWNED.has(combo)) return;
    e.preventDefault();
    e.stopPropagation();
    invoke('page_shortcut', { combo });
  }

  // ==========================================================================
  // Reporting to the core
  // ==========================================================================

  let scrollTimer = 0;
  function reportScroll() {
    clearTimeout(scrollTimer);
    scrollTimer = setTimeout(() => invoke('page_scroll', { y: scrollY }), 400);
  }

  function reportFavicon() {
    const link =
      document.querySelector('link[rel~="icon"]') ||
      document.querySelector('link[rel="shortcut icon"]');
    const href = link ? link.href : new URL('/favicon.ico', location.origin).href;
    invoke('page_favicon', { url: href });
  }

  // ==========================================================================
  // Wiring
  // ==========================================================================

  function apply(next) {
    cfg = next || cfg;
    globalThis.__EMERALD_CONFIG__ = cfg;
    applyCss();
    installRuler();
    installAnchoring();
    installDictation();
    if (cfg.input && cfg.input.force_spellcheck) {
      for (const el of document.querySelectorAll('input, textarea, [contenteditable]')) {
        el.spellcheck = true;
      }
    }
  }

  installAutoplayGuard();
  applyCss();
  installAnchoring();
  // Registered at document-start, before the page can install its own capture
  // listener and swallow the chord.
  addEventListener('keydown', onShortcut, true);

  onReady(() => {
    apply(cfg);
    tameMediaAttributes(document);
    reportFavicon();
    offerDrafts();

    addEventListener('input', onFieldInput, true);
    addEventListener('submit', onSubmit, true);
    addEventListener('keydown', onEnterKey, true);
    addEventListener('scroll', reportScroll, { passive: true });

    // Catch media and fields inserted after load. One observer, subtree-wide,
    // doing near-nothing per record.
    new MutationObserver((records) => {
      for (const rec of records) {
        for (const node of rec.addedNodes) {
          if (node.nodeType === 1) tameMediaAttributes(node);
        }
      }
    }).observe(document.documentElement, { childList: true, subtree: true });
  });

  // `pagehide` rather than `unload`: it fires for back/forward-cache navigations
  // too, so the last few keystrokes are never the ones that get lost.
  addEventListener('pagehide', () => {
    invoke('page_scroll', { y: scrollY });
    for (const el of document.querySelectorAll('input, textarea, [contenteditable="true"]')) {
      if (!isTextish(el) || isSensitive(el)) continue;
      const value = String(fieldValue(el) ?? '');
      if (value) {
        invoke('page_draft', { field: fieldKey(el), label: fieldLabel(el), value });
      }
    }
  });

  globalThis.__emerald = {
    apply,
    reader: (on) => (on ? readerOn() : readerOff()),
    restoreScroll: (y) => onReady(() => scrollTo({ top: y, behavior: 'instant' })),
    version: 1,
  };
})();
