# Design language

## 0. The thing this is trying not to be

There is a recognisable look that a generated interface has: a purple-to-blue gradient, a frosted translucent card floating on a blurred background, corners rounded to 16px everywhere, an icon set assembled from whatever was nearest, and a soft drop shadow on things that are not floating. It reads as unconsidered because it *is* unconsidered — each decision was made once, in isolation, by reaching for the nearest default.

Emerald's defence against that is not a rule saying "no gradients". It is that **every visual decision here has a reason attached, and the reason is about this browser specifically.** Where a choice is arbitrary, this document says so rather than inventing a justification.

Concretely, and non-negotiably:

- **No glassmorphism.** No `backdrop-filter`, no translucent panels, anywhere. Depth is carried by surface *value* — five greys — because a blurred backdrop makes text sit on an unpredictable ground, and unpredictable text contrast is the exact opposite of what this browser is for.
- **No gradients in the UI.** The app mark uses three flat colours. If a surface needs to feel different from the one behind it, it gets a different value and a hairline.
- **Shadow does exactly one job:** marking a surface that genuinely floats above the page — the command palette, a dialog. Never for hover, never for emphasis, never on a static card.
- **Nothing flashes, blinks, pulses or breathes.** Ever, in any theme, at any animation speed, for any reason. §4.

---

## 1. Palette

Catppuccin Mocha, used through three layers. Components never reference a palette value directly; they reference a semantic token, and the semantic tokens reference the palette. Re-theming happens in one place, and "which green is the active tab" has exactly one answer.

```
palette      →  semantic       →  component
--ctp-green     --accent          box-shadow: inset 2px 0 0 var(--accent)
```

### Surfaces — depth by value, five steps

| Token | Mocha | Used for |
| --- | --- | --- |
| `--sunken` | `crust` `#11111b` | the content well behind pages |
| `--surface` | `base` `#1e1e2e` | the app ground, active tab |
| `--surface-raised` | `mantle` `#181825` | sidebar, toolbar, palette |
| `--surface-hover` | `surface0` `#313244` | hover |
| `--surface-active` | `surface1` `#45475a` | pressed, selected segment |

Note that in Mocha the *raised* surfaces are darker than the base. That is the correct direction for a dark theme — receding means darker — and Latte inverts it, which is why the theme override redefines these four rather than only swapping the palette.

### The accent means one thing

**The accent means "this is the live thing, or this is yours."** The active tab. The current space. The bookmarked state. A focused toggle's track.

If an accented surface is not the thing you are working on, that is a bug. This is the single rule that stops a Catppuccin palette turning into a theme dump — the temptation with a palette this pretty is to use mauve for one thing and peach for another because both look nice, and the result is a UI where colour carries no information at all.

The accent is user-selectable across seven Catppuccin hues. Everything else about its meaning is fixed.

### Status hues never move

`--danger` is red, `--warn` is yellow, `--info` is blue — in every theme, under every accent, in every sensory profile. If red drifted with the accent, the one thing colour is genuinely load-bearing for would stop being learnable. Someone who has learned that red means destructive should not have to re-learn it because they preferred the teal accent.

### Focus is blue, and deliberately not the accent

Focused and active are different states that frequently co-occur — the active tab can also be keyboard-focused. If both were accent-coloured they would be indistinguishable exactly when it matters. So `--focus` is blue, `2px`, offset `2px`, applied by one `:focus-visible` rule for the entire application. There is no component that styles its own focus ring.

### Sensory profiles change loudness, not meaning

| Profile | What changes |
| --- | --- |
| `standard` | Mocha as designed |
| `muted` | accents mixed 45% toward the text colour; surface separation flattened; float shadow becomes a hairline |
| `high_contrast` | text/background pushed past WCAG AAA; focus ring to 3px |

`muted` exists because for some people saturated colour is itself the problem, and the usual advice — "use the light theme" — does not address it. Crucially it preserves hue relationships: a warning in `muted` is still recognisably the warning colour, just quieter.

---

## 2. Typography

### JetBrains Mono for the chrome

A monospace UI font is an unusual choice. The reasons:

1. **Most chrome text is identifiers.** Tab titles, URLs, settings values, keyboard hints. A fixed advance width makes a vertical column of them scannable — your eye can compare position between rows, which a proportional face makes impossible.
2. **URLs are the security-relevant text in a browser**, and a monospace face makes homoglyph and lookalike-domain tricks measurably harder to hide.
3. **It gives Emerald a face.** A browser chrome in the system sans is a browser chrome with no point of view. JetBrains Mono is warm for a mono — humanist curves, a tall x-height, and it was drawn for reading code for hours, which is the right adjacent problem.

Weights: 400 and 500 only, plus 700 for the rare heavy label. Emphasis in this UI comes from colour and position, not from a third weight.

### Not mono for reading

Body prose — settings help text, empty states, reader mode — uses **Atkinson Hyperlegible**, designed by the Braille Institute with deliberately differentiated letterforms. It is Emerald's reading default because it helps a wide range of readers without looking like an accessibility mode. **OpenDyslexic** and a plain system stack are the alternatives, and the mono face is offered too because some readers do track better on a grid.

On OpenDyslexic specifically: the research is genuinely mixed and several studies find no reading-speed benefit. Emerald offers it anyway, prominently, because plenty of people report that it helps them and that is sufficient reason to make it one click away. Pretending the evidence is settled in either direction would be the dishonest move.

### Scale

A 1.2 minor third off a 13px base, rounded to whole pixels, multiplied by `--ui-scale` from settings.

| Token | px @100% | Used for |
| --- | --- | --- |
| `--text-xs` | 10 | section labels, kind tags, `kbd` |
| `--text-sm` | 11 | help text, secondary lines |
| `--text-md` | 13 | **base** — tabs, buttons, inputs |
| `--text-lg` | 16 | palette input, panel lede |
| `--text-xl` | 19 | stat figures |
| `--text-2xl` | 23 | panel headings |

Three leading values (`1.25` / `1.5` / `1.7`). Tracking opens slightly at the smallest sizes, where mono needs air.

---

## 3. Space and shape

**4px base unit**, tokens `--space-1` through `--space-8`. There are no arbitrary pixel values in the component CSS; if you find one that is not a 1px hairline, it is a bug.

**Radii step by 2px** — `4 / 6 / 10` — matching the 2px corner rule on the icon grid, so a button corner and an icon corner belong to the same drawing. The only pill shape in the interface is the space chip, which is a pill because it is the one control that is genuinely a toggle between named states.

**Density** (`compact` / `comfortable` / `roomy`) drives `--row-height` and `--pad-inline` only. It changes rhythm, never type size — those are separate settings because they solve separate problems, and conflating them is why "compact mode" in most apps also makes text unreadable.

---

## 4. Motion

**Two properties are ever animated: `opacity` and `transform`.** Never height, never colour, never anything that could read as a flash.

**One easing curve** for the whole application, `cubic-bezier(0.2, 0, 0, 1)`. Multiple curves in one interface read as multiple authors.

**Three durations**, all multiplied by `--motion-scale`: 90ms, 140ms, 220ms.

`--motion-scale` comes from the animation-speed setting. **At zero, every transition becomes 0ms and the interface simply snaps.** Nothing is lost, because no information in Emerald is conveyed by movement alone — anything a transition indicates is also indicated by a durable state change. Choosing `transitions: instant` forces zero regardless of the speed multiplier, because "instant" should mean instant rather than "fast".

### The no-flashing rule is enforced, not requested

A design rule that depends on everyone remembering it will be broken by the fifth contributor. So `tokens.css` cancels every infinitely-repeating animation globally:

```css
*, *::before, *::after { animation-iteration-count: 1 !important; }
```

Whatever declares a pulse, it runs once and stops. The loading spinner is the single permitted exception and opts in explicitly by class — it rotates at a constant rate and does not change opacity, and at zero motion it stops dead and becomes a static glyph. A spinner that cannot be turned off is a spinner some people cannot use.

`prefers-reduced-motion: reduce` sets `--motion-scale: 0` without waiting for the in-app setting.

---

## 5. Icons

The set is in one file, `src/icons/index.tsx`, drawn as one system. Consistency is **enforced by construction** rather than by discipline: every icon is the same `<Icon>` wrapper with the same viewBox, stroke width, cap and join, and only the path data differs. It is not possible to add an off-system icon without editing the wrapper.

| Rule | Value |
| --- | --- |
| Grid | 24×24, geometry on a 2px sub-grid |
| Stroke | 1.75px, round cap, round join, **never filled** |
| Safe area | 2px — nothing touches the box edge |
| Corner | 2px radius, except facets |
| **The facet** | every icon carries exactly one 45° cut |

**The facet is the signature.** It comes from the app mark — an emerald-cut gem, whose defining feature is its cut corners — and it recurs in every icon: the sidebar's content edge, the split view's right pane, the archive box's lid, the reader's page corner. It is what makes a strip of these read as one hand rather than a collection. When adding an icon, find where its one cut goes.

Icons scale with type (`1em` default) so they sit on the text baseline at every UI scale.

**Icons never animate** — not on hover, not on load, not for notification.

### Choices that were made rather than defaulted

- **Attention** is a target, not a brain or a lightning bolt. ADHD tooling does not need a metaphor for the person using it.
- **Predictability** is a spirit level, because the promise is that nothing tilts.
- **A resting tab** is a closed eye, not a "z z z". The tab is not asleep and does not need waking — it is simply not being looked at, and it returns instantly. The metaphor should not imply a delay that is not there.
- **Reading** is a line of text with the ruler beneath it — the feature, not a book.

### Cursors

Emerald uses the platform cursor set. Custom cursors are a hit-area and accessibility risk for no benefit, and they are the fastest way to make an app feel wrong on someone's OS. The chrome sets `cursor: default` over furniture (so it does not present a text caret over things that are not text) and leaves everything else to the platform.

### Loading

The spinner is the only moving thing, and it is a last resort. Preferred, in order: (1) show the thing, (2) show its shape, (3) say what is happening in words, (4) spin. Tab loading uses the spinner in place of the favicon — same 16px box, no layout change when it resolves.

### Empty states

Four illustrations, drawn on the same 24-unit grid scaled to 96 with the stroke ratio preserved, so they are visibly the same hand as the icons rather than stock art dropped in.

**One idea each, no scenery, no mascot.** Nothing open is a single empty facet — the shape of the app, waiting. Empty archive is an open, level box: nothing has been lost. No drafts is a blank page with nothing on the lines.

The copy does the real work. "Nothing open" tells you the two keystrokes that fix it. The resting-tab state explains what happened to your memory, what will come back, and links to the setting that controls it — because a browser that silently discards your tab owes you an explanation more than it owes you an illustration.

---

## 6. Writing

The interface text is part of the design language, and for this product it may be the most important part.

- **Say what happened and what will happen.** "Emerald released its memory because it had not been used recently. Everything about it comes back when you open it."
- **Name the trade-off where there is one.** The reading-font setting says which sites will show empty boxes and why it is off by default. A setting that hides its cost gets turned on, then blamed.
- **Write for the person the setting is for**, not for the engineer who added it. Compare "Enable draft autosave" with "Save drafts of what you type — every text field you type into is saved locally as you go. Nothing is sent anywhere."
- **No exclamation marks, no encouragement, no personality.** This browser is furniture for people who need less noise.
- **State is written out, not just coloured.** Every toggle prints "On" or "Off" beside it. Colour is confirmation, never the only signal.

Help text comes from the Rust doc comments in `settings.rs` via codegen, which has a useful forcing effect: explaining a setting badly in the code produces a visibly bad settings panel.

---

## 7. Adding something

1. Does it use `--`-prefixed semantic tokens only? No raw hex, no raw px.
2. If it is coloured with the accent, is it the live thing or the user's thing?
3. Does it animate anything other than opacity or transform? Does it survive `--motion-scale: 0`?
4. If it is an icon, is it in `icons/index.tsx`, on the grid, with one facet?
5. Does its state read without colour?
6. Is the copy honest about the trade-off?
7. Is any hit target smaller than the user's `min_target_px`?
