# Let-timer Web — Motion Plan (calm and minimal)

> Status: **draft v1 for owner review**. Companion documents: `docs/web_design_plan.md` (flows, tokens), `docs/ui-wireframes.md` (v2 screens and components), `AGENTS.md` (conventions).
> Reference prototypes (not production code): `motion-styles-demo.html` (five styles compared) and `letimer-motion-preview.html` (the hybrid applied to Today, Focus, Media).
> IDs: moments `A01…`, decisions `DM1…`, phases `M0…M5`.

---

## 1. Goal and scope

Give the web app a **calm, minimal, purposeful** sense of motion. Motion is allowed to do exactly three jobs:

1. **Confirm** that something changed (a task started, finished, was added).
2. **Preserve spatial continuity** (a task moved *there*, a panel came from *that* side).
3. **Add a little atmosphere** where the user is meant to dwell (Focus view, a running timer).

Everything else stays still. Out of scope: sound, haptics, Lottie/illustration animation, page-load splash, parallax, and any animation of the timer digits themselves.

## 2. Current state (from the screenshot review)

Observed in the 11 desktop/mobile screenshots (dark theme, one wallpaper; hover and motion not visible):

| Observation | Why it matters for motion |
|---|---|
| Today list sits directly on a full-screen wallpaper; panels are translucent | Animating elements over `backdrop-filter` regions is expensive on mid-range phones |
| Focus view: opening the music player moves the clock from about y=330 to y=205 | A layout shift. Animation cannot hide it; it must be fixed first (prerequisite 0.1) |
| Workspace popover is see-through (page title shows through) | Needs an opaque surface before it can be animated convincingly |
| Floating timer pill at the bottom covers content in Settings | Needs bottom padding; the pill is the main animated element |
| Running and pending rows both show an empty ring | State changes cannot be animated meaningfully if the states look the same |
| Mobile: second task row shows no actions (hover-only?) | Touch users cannot trigger the animations we are adding |
| App already has palettes (Indigo, Forest) and "Harmonize with Wallpaper" | Theme/palette change needs a transition rule that does not repaint every hover state |
| Header stays visible in Focus (v2 wireframes wanted it to fade) | Part of the idle-fade work in M4 |

The component names below (`TimerBar`, `FocusView`, `TodayView`, `TaskDrawer`) come from earlier reviews; agents must confirm the real file names before editing.

## 3. Principles

1. **Motion explains, it never decorates.** If removing an animation loses no information, prefer removing it.
2. **Fast for input, slow only for ambience.** Direct responses to a tap or click finish within 150ms of perceived feedback; only state changes and scenery may take 300ms or more.
3. **Only `transform` and `opacity`** (plus `background-color`/`color` for state tint). No animation of `width`, `height`, `top/left`, `blur` radius, or box-shadow. Allowed exceptions: the delete-row collapse (A21) and the completed disclosure (A33).
4. **No bounce, overshoot, shake, or flashing.** Pulses are at most two, then still.
5. **At most one ambient loop visible at a time**, small in amplitude, and it stops by itself.
6. **Interruptible.** Every animation can be cancelled or reversed; no control is ever disabled "while animating".
7. **Digits never animate.** The timer text changes once per second, `tabular-nums`, no rolling or flipping.
8. **Reduced motion is a first-class design**, not a fallback: opacity fades of 100ms, nothing travels, nothing loops.

## 4. The hybrid model

| Layer | Name | Job | Typical moments | Timing |
|---|---|---|---|---|
| **L1** | Feedback (Gentle lift) | Respond to input; UI appears or disappears | Overlays, buttons, new rows, toasts, tab fade | 120–240ms, decelerating, travel at most 8px (drawer 28px) |
| **L2** | Presence (Breathing) | Show time passing and ambience | Running dot, time's-up, wallpaper change, theme change | 360–460ms for state changes; loops have a period of at least 3s |
| **L3** | Continuity (Paper) | Keep spatial meaning | Task moves between groups, date change | 280–340ms, FLIP, emphasized easing |

**Layer decision rule** (agents must follow it, in this order):

1. The element **changes container or position because of a state change** (task from Tasks to Completed): **L3**.
2. The motion is **not triggered by a click/tap**, or it describes something that continues over time: **L2**.
3. Everything else: **L1**.
4. Anything the user can trigger many times per minute (button press, hover, row hover): only the fast end of L1 (at most 150ms, usually colour only).

## 5. Motion tokens

Single source of truth is CSS custom properties in `web/src/styles/motion.css`; TypeScript reads them (never duplicates numbers).

```css
:root {
  --dur-instant: 80ms;
  --dur-fast: 120ms;
  --dur-base: 200ms;
  --dur-slow: 340ms;
  --dur-ambient: 460ms;
  --dur-scenic: 1200ms;          /* wallpaper crossfade only */

  --ease-out:   cubic-bezier(0.2, 0, 0, 1);      /* default, decelerating */
  --ease-inout: cubic-bezier(0.45, 0, 0.25, 1);  /* ambient, soft */
  --ease-emph:  cubic-bezier(0.3, 0, 0, 1);      /* continuity moves */

  --travel-s: 8px;               /* rise for new/entering items */
  --travel-m: 28px;              /* drawer, sheets */
  --exit-ratio: 0.7;             /* exits are 70% of the entry duration */
}

/* Reduced: everything becomes a short fade, nothing travels. */
:root[data-motion="reduced"] {
  --dur-fast: 100ms; --dur-base: 100ms; --dur-slow: 100ms;
  --dur-ambient: 100ms; --dur-scenic: 100ms;
  --travel-s: 0px; --travel-m: 0px;
}
@media (prefers-reduced-motion: reduce) {
  :root:not([data-motion="full"]) {   /* system default and "reduced" both land here */
    --dur-fast: 100ms; --dur-base: 100ms; --dur-slow: 100ms;
    --dur-ambient: 100ms; --dur-scenic: 100ms;
    --travel-s: 0px; --travel-m: 0px;
  }
}
/* Test-only: set by ?motion=off in non-production builds */
:root[data-motion="off"] { --dur-instant: 0ms; --dur-fast: 0ms; --dur-base: 0ms;
  --dur-slow: 0ms; --dur-ambient: 0ms; --dur-scenic: 0ms; --travel-s: 0px; --travel-m: 0px; }
```

`data-motion` values: `system` (default; follows the OS), `reduced`, `full`, and test-only `off`. The reduced block appears twice (explicit attribute and media query); a unit test asserts both blocks are identical.

## 6. Moment-by-moment specification

Legend: **P** = properties animated, **Red.** = behaviour under reduced motion.

| ID | Moment | Layer | Animation | P | Duration and easing | Red. | Notes |
|---|---|---|---|---|---|---|---|
| A01 | Button press | L1 | Scale 0.98 on `:active` | transform | `--dur-instant` | none | Hover is colour only (`--dur-fast`) |
| A02 | Hover / focus states | L1 | Background or colour change | background-color | `--dur-fast` | same | Focus ring appears instantly |
| A03 | Segmented control / toggle | L1 | Selected thumb slides | transform | `--dur-base`, `--ease-out` | instant | |
| A04 | Popover / menu open | L1 | Scale 0.97 to 1 plus fade, origin at the trigger | transform, opacity | `--dur-fast`; close `--dur-instant` | fade only | Surface must be opaque |
| A05 | Drawer open | L1 | Scrim fades; panel fades and slides `--travel-m` from the right edge | transform, opacity | `--dur-base`, `--ease-out`; close ×`--exit-ratio` | fade only | Short slide, not full width |
| A06 | Modal (StartPicker, ConfirmDialog) | L1 | Fade plus rise `--travel-s` | transform, opacity | `--dur-base` | fade only | Bottom sheet on mobile: slide `--travel-m` |
| A07 | Toast | L1 | Fade plus rise; exit by fade | transform, opacity | `--dur-base` | fade only | Auto-exit after 4s; paused on hover/focus |
| A08 | Floating timer pill enters | L1 | Fade plus rise `--travel-s` | transform, opacity | `--dur-base` (240ms) | fade only | Fixed width so name changes do not resize it |
| A09 | Pill exits | L1 | Fade plus small drop | transform, opacity | ×`--exit-ratio` | fade only | |
| A10 | Play/pause icon, task name swap | L1 | Crossfade | opacity | `--dur-fast` | same | No icon morph |
| A11 | Timer digits | none | Value changes once per second | – | – | – | Colour tint on pause/overtime only (A13) |
| A12 | Running dot | L2 | Gentle breathing: scale 0.75–1, opacity 0.55–1 | transform, opacity | 4s period, `--ease-inout`, **7 iterations (about 28s), then still** | static | Stops on pause, overtime, done |
| A13 | State tint (running, paused, overtime) | L2 | Background/border/text colour transition | color props | 400ms | same | Overtime uses orange tokens, never red |
| A14 | **Time's up** | L2 | Tint to orange (A13) plus two ring pulses (scale 1 to 1.16, opacity 0.7 to 0), then still | transform, opacity | each pulse about 1.1s, second delayed 1.5s | tint only | Plus toast (A07) and one `role="alert"` announcement |
| A15 | Daily progress bar | L1 | Fill grows from the left | transform (`scaleX`) | 300ms, `--ease-out` | instant | Never animate `width` |
| A16 | Row state glyph | L1 | Ring to filled dot (running), dashed ring (paused) | opacity, stroke | `--dur-fast` | same | States must look different first (prereq 0.5) |
| A17 | **Complete task** | L1 then L3 | Tick draws (stroke), then row travels to Completed while others close the gap | stroke-dashoffset, transform | tick `--dur-base`; move `--dur-slow` with `--ease-emph` | tick appears; row jumps with a 100ms opacity highlight | Signature moment; implemented with Motion `LayoutGroup` + `layoutId` |
| A18 | **Start / pause a task (group change)** | L3 | Row travels between Now running and Tasks | transform | `--dur-slow`, `--ease-emph` | instant | Implemented with Motion `LayoutGroup` + `layoutId` |
| A19 | Add task | L1 | Row fades and rises; background highlight fades out | opacity, transform, background-color | `--dur-base`; highlight 600ms | fade only | |
| A20 | Quick-add parsing preview | L1 | Preview chip crossfades | opacity | `--dur-fast` | same | |
| A21 | Delete task | L1 | Row fades; height collapses so siblings follow | opacity, height | `--dur-base` | fade then remove | Allowed height animation exception (Motion exit) |
| A22 | Date pager | L3 | Content slides 12px in the direction of travel with a crossfade | transform, opacity | `--dur-base` | fade only | |
| A23 | Workspace switch | L1 | List crossfade, no stagger | opacity | 160ms | same | |
| A24 | Tab/view change | L1 | ViewHost `mode="wait"` crossfade; wallpaper never changes | opacity | enter `--dur-base`, exit ×`--exit-ratio` | same | Single engine ViewHost; wallpaper isolated |
| A25 | Focus: idle fade | L1 | Header, controls and hints fade out after 3s without input; any mouse, key, touch or focus brings them back | opacity | `--dur-fast` (150ms) | same | **Hidden via opacity only**: stays focusable; never hides while a menu, drawer or sheet is open |
| A26 | Focus: music sheet | L1 | Bottom sheet slides up | transform, opacity | 280ms, `--ease-emph`; close ×`--exit-ratio` | fade only | The clock must not move |
| A27 | Wallpaper change | L2 | Two stacked layers crossfade | opacity | `--dur-scenic` | 100ms fade | Image decoded before the fade |
| A28 | Theme / palette change | L2 | Colours transition | color props | 250ms | instant | Transition class attached only during the switch |
| A29 | Equalizer bars | L2 | Three bars oscillate | transform | 900ms loop | static bars | **Only while audio plays** |
| A30 | Connection badge | – | `Live` is static; `Syncing` spinner rotates | transform | 1s linear | static icon | No pulsing Live dot |
| A31 | Skeleton | L2 | Slow shimmer | opacity | 1.6s | static | Shown only when loading exceeds 300ms |
| A32 | Focus wake-up / ken-burns | – | Not implemented | – | – | – | Optional later (DM5) |
| A33 | Completed disclosure | L1 | Height expands/collapses | height, opacity | `--dur-base` | instant | Allowed height animation exception |

Example timelines:

```
Complete task   0ms tick draws ───> 200ms row travels + siblings close gap ───> 540ms counters crossfade
                (parallel: progress bar scaleX 300ms)

Time's up       0ms tint to orange (400ms) ───> ring pulse 1 (0–1.1s) ───> ring pulse 2 (1.5–2.6s) ───> still
                (parallel: toast in, one aria-live alert, dot stops breathing)
```

## 7. Calm budget (guard rails)

- Maximum durations: L1 240ms (sheets 280ms), L3 340ms, L2 state changes 460ms, wallpaper 1200ms.
- No easing with overshoot. No `steps()` except none. No infinite loops except A29/A31 (both conditional) and the one finite A12.
- Pulses: at most 2 per event. Flashing: none.
- At most **one ambient loop on screen** (A12); A29 applies only inside the player.
- Exits are shorter than entries (×`--exit-ratio`).
- New animation = new row in section 6, otherwise it does not ship.

## 8. Architecture

```
web/src/
  styles/motion.css          tokens + shared keyframes (section 5)
  motion/
    tokens.ts                MotionTokens interface, readMotionTokens(), invalidateCache(), parseBezier()
    MotionProvider.tsx       MotionProvider context + useMotionTokens()
    useMotionMode.ts         system | reduced | full | off, persists to localStorage 'let_timer_motion'
    useMotionTransition.ts   useMotionTransition('instant'|'fast'|'base'|'slow'|'ambient') helper
    animate.ts               Web Animations API wrapper for imperative one-offs
  components/ViewHost.tsx    ViewHost tab transition container (AnimatePresence mode="wait")
  scripts/check-motion-tokens.mjs  Strict token linter (enforces no raw durations/easings in TSX)
  test/                      behavioral component tests
  e2e/                       Playwright E2E suite
```

### Single Engine Ownership Policy

1. **One engine per element.** An element may be animated by exactly one of: Motion props (`initial/animate/exit/layout/layoutId`), a CSS animation/transition class, or the WAAPI `animate()` wrapper. Never combine them on the same element or on the same CSS property of one element.
2. **Ownership table**:

| Concern | Engine |
|---|---|
| Mount/unmount of views, list rows, toasts, island, workspace/media lists, shared thumb | Motion (`AnimatePresence`, `layout`, `layoutId`) |
| Radix overlays (drawer, modal, popover, menu, tooltip) | CSS keyframes on `data-state` |
| State tints, hover, press, progress bar, date slide, tick draw, running dot, ring pulses, equalizer | CSS transitions/keyframes |
| Imperative one-offs (wallpaper crossfade) | `animate()` wrapper |

3. **No raw timings or easings in TSX** (`duration: 0.x`, `ease: [...]`, `cubic-bezier(` outside `styles/motion.css` and `motion/tokens.ts`). Use `useMotionTransition()`. Enforced by `npm run lint`.
4. **Mode** is `data-motion` on `<html>`, set by an inline script in `index.html` before first paint, mapped by `MotionProvider` (`reduced/off` -> `always`, `full` -> `never`, `system` -> `user`), and controlled by `useMotionMode`.
5. Exit animations never delay focus return or screen-reader announcements (mark exiting nodes `inert`).

## 9. Accessibility

| Requirement | How it is met |
|---|---|
| WCAG 2.3.3 / user preference | `prefers-reduced-motion` honoured by default; Settings "Motion: System / Reduced / Full" overrides |
| WCAG 2.2.2 (auto-moving over 5s) | A12 stops after about 28s and is static in reduced; A29 only while audio plays; A31 only during long loads |
| WCAG 2.3.1 (flashing) | No flashing; at most two soft pulses per event |
| Screen readers | Timer live region announces on **minute change or state change** (not every second); overtime announced once with `role="alert"` |
| Keyboard | Idle-hidden controls remain in tab order; `:focus-within` and any key press restores them |
| Focus management | Radix dialogs trap and return focus; exits do not delay it |
| Colour | Overtime uses `--timer-over` and `--over-text` (never red); state is also conveyed by icon and text, not colour alone |

## 10. Performance

- Budget: 60fps on a mid-range Android with 4x CPU throttle in Playwright traces; no long task over 50ms during any section 6 animation; **CLS = 0** across all interactions.
- `will-change` only while an animation runs (set before, removed on finish).
- **Backdrop blur:** do not animate elements that sit over large `backdrop-filter` regions; prefer pre-blurred wallpaper variants and opaque overlay surfaces. If a panel needs blur, test it with the throttle above.
- Wallpaper: two stacked layers; call `img.decode()` before the crossfade; the hidden layer is `visibility: hidden` and unpainted.
- Bundle: no new runtime dependency in M0–M4 beyond Radix primitives (if not already present); target change in bundle size at most +3 KB gzip.
- Animate with compositor-friendly properties only (section 3). Row collapse (A21) is the one measured exception.

## 11. Prerequisites (fix before any animation work)

| # | Work | Why |
|---|---|---|
| 0.1 | Focus: make the music sheet absolutely positioned (or a bottom sheet) so the clock never moves | Layout shift (screenshot 05) |
| 0.2 | Popover, menu and drawer surfaces become opaque (or use a stronger overlay) | See-through workspace popover (screenshot 03) |
| 0.3 | Bottom padding = pill height + safe area on every view | Pill covers content (screenshot 09) |
| 0.4 | Row actions always visible on touch devices | Mobile row had no actions (screenshot 10) |
| 0.5 | Distinct glyphs: pending = empty ring, running = ring plus filled dot, paused = dashed ring, done = success ring plus tick | Needed for A16–A18 |
| 0.6 | Media panel: constrain the select/Attach row so it cannot overflow the card | Overflow (screenshot 07) |

## 12. Phases

Every item is complete only when `npm run lint && npm test && npm run build` pass, the item's tests are added, and the manual check in **Done when** is performed in `full`, `reduced` and `off` modes.

### M0 — Foundations

**Goal**: tokens, mode switch, prerequisites, a gallery. No visible motion change yet except what prerequisites fix.

| # | Do | Files |
|---|---|---|
| 0.1–0.6 | Prerequisites from section 11 | `FocusView`, popover/drawer components, layout wrappers, `TodayView`, status glyph, `MediaView` |
| 0.7 | Create `motion.css` (section 5) and import it once in the app entry; create `motion/tokens.ts`, `useMotionMode.ts`, `animate.ts` | `web/src/styles`, `web/src/motion` |
| 0.8 | Inline script in `index.html` sets `data-motion` from `localStorage` (or `system`) before paint; `?motion=off` honoured only when `import.meta.env.MODE !== 'production'` | `index.html` |
| 0.9 | Settings: "Motion" segmented control (System, Reduced, Full) with a one-line description; persists to `let_timer_motion` | `SettingsView` |
| 0.10 | CI grep rule: no raw ms/easing outside the two token files | `package.json` scripts, CI |
| 0.11 | `/dev/motion` gallery: port the preview prototype, driven by real tokens and real `animate()`; used for manual review and Playwright | `views/DevMotion.tsx` |

**Tests**: `motion.test.ts` (both reduced blocks identical; every token present; `off` zeroes durations); `useMotionMode` persists and reacts to `matchMedia` change; layout test that opening the player leaves the clock position unchanged.
**Done when**: switching the Motion setting changes `data-motion` live; CLS in Focus is 0; the gallery loads.

### M1 — Overlays and micro-interactions (L1)

| # | Do |
|---|---|
| 1.1 | Drawer (A05), Modal and bottom sheet (A06), Popover/menu (A04) using Radix `data-state` keyframes (appendix C); exit ×`--exit-ratio`; scrim fade |
| 1.2 | Toast (A07) with pause on hover/focus; ErrorChip fade |
| 1.3 | Buttons (A01), hover/focus (A02), segmented thumb (A03) |
| 1.4 | Lint rule: new overlay components must use the shared keyframes (no inline durations) |

**Tests**: unit tests assert the keyframe names applied for `open` and `closed`; Playwright: with `reducedMotion: 'reduce'` no element has a non-zero `transform` during open; focus returns to the trigger after close; Esc closes.
**Done when**: all overlays open and close without flicker at 4x CPU throttle; reduced mode shows 100ms fades only.

### M2 — Timer pill and lifecycle feedback (L1 and L2)

| # | Do |
|---|---|
| 2.1 | Pill enter/exit (A08/A09) with fixed width; icon and name crossfade (A10) |
| 2.2 | State tint transitions for running, paused, overtime (A13) |
| 2.3 | Running dot breathing, 7 iterations, cancelled on pause/overtime/done (A12) |
| 2.4 | Time's-up sequence (A14): tint, two ring pulses, toast, single `role="alert"`; digits unchanged (A11) |
| 2.5 | Daily progress bar via `scaleX` (A15) |
| 2.6 | Live region updates on minute or state change only |

**Tests**: component tests for each state class; fake timers confirm the dot animation has finite iterations and stops on state change; the overtime alert fires exactly once per run; the ring runs two pulses and none in reduced.
**Done when**: starting, pausing, resuming and overtime look as specified in the gallery and the real app; screen-reader transcript shows no per-second announcements.

### M3 — Task flow (L3)

| # | Do |
|---|---|
| 3.1 | `useFlip` hook (appendix B) with rects measured relative to the list container and ignored when scroll position changed between measures |
| 3.2 | Rows keyed by task id so DOM nodes persist across group changes (Now running, Tasks, Completed) |
| 3.3 | Start/pause group change (A18) and complete (A17): tick draw then FLIP |
| 3.4 | Add (A19), delete (A21), date pager (A22), workspace crossfade (A23) |
| 3.5 | Feature flag `motion.flip` (default on); any exception inside FLIP falls back to instant placement |
| 3.6 | Glyph transitions (A16) |

**Tests**: hook unit test with mocked `getBoundingClientRect` (known deltas produce the expected `animate` calls; none when reduced or flag off); Playwright: complete a task and assert the row ends in Completed with no duplicate nodes; rapid double-click does not break ordering; counters update.
**Done when**: completing five tasks quickly never leaves a row stranded or duplicated; reduced mode places rows instantly with a 100ms highlight.

### M4 — Focus and ambience (L1 and L2)

| # | Do |
|---|---|
| 4.1 | Idle fade (A25): header, controls, hints fade via opacity after 3s; restore on mouse, key, touch, focus; never while a menu/drawer/sheet is open |
| 4.2 | Music sheet (A26) as an absolutely positioned bottom sheet |
| 4.3 | Wallpaper crossfade (A27) with two layers and `decode()` |
| 4.4 | Theme/palette transition (A28): add a short-lived `theme-transition` class on `<html>`; re-check contrast of harmonized colours after transition |
| 4.5 | Equalizer (A29) only while playing; `Live` badge static, `Syncing` spinner (A30); skeleton shimmer after a 300ms delay (A31) |

**Tests**: Playwright: idle hide after 3s, restore on key press, hidden controls remain focusable (Tab reaches them); clock position unchanged when the sheet opens; wallpaper change causes no CLS; equalizer animation only while `audio` is playing.
**Done when**: a keyboard-only user can use Focus fully; CPU-throttled trace stays at 60fps during wallpaper crossfade.

### M5 — View transitions and polish

| # | Do |
|---|---|
| 5.1 | Tab crossfade (A24) via CSS; optional `document.startViewTransition` enhancement behind feature detection, with the wallpaper isolated from the transition |
| 5.2 | Performance pass: trace each section 6 moment at 4x CPU throttle; fix anything below 60fps; remove stray `will-change` |
| 5.3 | Visual QA matrix: 360, 768, 1280 px, light and dark, `full` and `reduced`; record short videos or screenshots in `web/.design/motion/` |
| 5.4 | Documentation: update `AGENTS.md` (motion rules), `docs/ui-wireframes.md` (link), add the section 6 table as the review checklist for future PRs |

**Done when**: the acceptance checklist (section 14) passes in CI and the owner signs off the QA matrix.

## 13. Testing strategy

- **Unit**: token completeness, reduced-block equality, hook behaviour (`useFlip`, `useMotionMode`), "exactly once" semantics for alerts.
- **Playwright**: run every spec in `full` and with `test.use({ reducedMotion: 'reduce' })`; visual snapshots use `?motion=off`; separate perf specs use a CPU-throttled CDP session and read Layout Shift entries (appendix D).
- **Manual**: the `/dev/motion` gallery is the reference; reviewers compare the app against it.
- **Axe**: run on every view with the overlay open and closed; confirm no focus-order regressions.

## 14. Acceptance checklist (every motion PR)

- [x] The change corresponds to a row in section 6 (or adds one with layer, properties, duration, reduced behaviour).
- [x] Only `transform`, `opacity` and colour properties are animated (A21 excepted).
- [x] All durations and easings come from tokens; CI grep passes.
- [x] `data-motion="reduced"`: no travel, no loops, fades at most 100ms.
- [x] `data-motion="off"`: durations zero, tests stable.
- [x] No controls are disabled during animations; animations can be interrupted.
- [x] No new infinite loop; ambient loops stop by themselves.
- [x] Screen-reader behaviour checked: no per-second announcements.
- [x] Keyboard-only run-through passes; focus never trapped or delayed by exits.
- [x] CLS = 0 on the affected screens; 60fps under 4x CPU throttle.
- [x] `npm run lint && npm test && npm run build` pass.

## 15. Risks

| Risk | Mitigation |
|---|---|
| Agents drift (different timings per component) | Tokens only, CI grep, section 6 as the single spec, gallery as reference |
| FLIP bugs with filters, reorders, scroll | Rects relative to container, scroll guard, feature flag, graceful fallback |
| Backdrop blur hurts frame rate on phones | Pre-blurred wallpapers, opaque overlays, throttled traces in M5 |
| "Calm" erodes as animations accumulate | Calm budget (section 7) and the rule that every new animation needs a table row |
| Idle fade confuses screen-reader or keyboard users | Opacity only, stays focusable, restores on focus-within and key press |
| Palette and harmonized colours change at runtime and break contrast after a transition | Contrast clamp on generated colours (separate task); re-test after transitions |
| Mobile safe areas (home indicator) overlap the pill and sheets | Respect `env(safe-area-inset-bottom)`; test on a 390x844 viewport |

## 16. Decisions (defaults chosen; owner may change)

| # | Decision | Default | Alternative |
|---|---|---|---|
| DM1 | Overall style | Hybrid: L1 base, L2 for presence, L3 for continuity | A single pure style |
| DM2 | Drawer | Short slide (28px) plus fade | Full-width slide, or fade only |
| DM3 | Breathing dot | 28s then still | Never, or always while running |
| DM4 | `Live` badge | Static | Slow pulse |
| DM5 | Ken Burns / parallax in Focus | Not implemented; optional later behind a setting, default off | Ship now |
| DM6 | List & overall animation library | Adopted Motion library (`motion` v14) with `MotionConfig`, `AnimatePresence`, and layout animations (own `useFlip` removed in favour of Motion layout) | Manual CSS keyframes / custom `useFlip` hook (caused flickering / duplicate animations) |
| DM7 | Tab transitions | CSS crossfade; View Transitions as an enhancement | None |
| DM8 | Reduced motion | 100ms fades; in-app Motion setting (System/Reduced/Full) | Follow the OS only |
| DM9 | Sound on time's up | None in this plan | Optional chime (needs a user gesture to unlock audio) |
| DM10 | Stagger on lists | None | 20ms stagger, max six rows |

---

## Appendix A — shared keyframes (excerpt)

```css
@keyframes rise-in  { from { opacity: 0; transform: translateY(var(--travel-s)); } to { opacity: 1; transform: none; } }
@keyframes rise-out { from { opacity: 1; transform: none; } to { opacity: 0; transform: translateY(calc(var(--travel-s) * .6)); } }
@keyframes slide-in-right  { from { opacity: 0; transform: translateX(var(--travel-m)); } to { opacity: 1; transform: none; } }
@keyframes slide-out-right { from { opacity: 1; transform: none; } to { opacity: 0; transform: translateX(var(--travel-m)); } }
@keyframes pop-in  { from { opacity: 0; transform: scale(.97); } to { opacity: 1; transform: none; } }
@keyframes ring-pulse { from { opacity: .7; transform: scale(1); } to { opacity: 0; transform: scale(1.16); } }
@keyframes breathe { 0%, 100% { transform: scale(1); opacity: 1; } 50% { transform: scale(.75); opacity: .55; } }

.dot-running { animation: breathe 4s var(--ease-inout) 7; }              /* 7 iterations, then still */
.ring-pulse  { animation: ring-pulse 1.1s var(--ease-out) 2; animation-delay: 0s; }
:root[data-motion="reduced"] .dot-running, :root[data-motion="reduced"] .ring-pulse { animation: none; }
```

## Appendix B — `useFlip` sketch (Historical — superseded by Motion `LayoutGroup` + `layoutId`)

```ts
import { useLayoutEffect, useRef } from 'react';

export function useFlip<T extends HTMLElement>(deps: unknown[], enabled: boolean, container: React.RefObject<HTMLElement>) {
  const nodes = useRef(new Map<string, T>());
  const prev = useRef(new Map<string, DOMRect>());
  const prevScroll = useRef(0);
  const register = (key: string) => (el: T | null) => {
    if (el) nodes.current.set(key, el); else nodes.current.delete(key);
  };
  useLayoutEffect(() => {
    const origin = container.current?.getBoundingClientRect();
    const scroll = container.current?.scrollTop ?? 0;
    const next = new Map<string, DOMRect>();
    nodes.current.forEach((el, k) => next.set(k, el.getBoundingClientRect()));
    if (enabled && origin && scroll === prevScroll.current) {
      next.forEach((b, k) => {
        const a = prev.current.get(k);
        const el = nodes.current.get(k);
        if (!a || !el) return;
        const dx = a.left - b.left, dy = a.top - b.top;
        if (dx || dy) el.animate(
          [{ transform: `translate(${dx}px, ${dy}px)` }, { transform: 'none' }],
          { duration: 280, easing: 'cubic-bezier(0.3, 0, 0, 1)' }, // read from tokens in production
        );
      });
    }
    prev.current = next;
    prevScroll.current = scroll;
  }, deps); // eslint-disable-line react-hooks/exhaustive-deps
  return register;
}
```

Production version reads durations/easing from `readMotionTokens()` and returns instantly when `data-motion` resolves to reduced.

## Appendix C — Radix overlay animation

```css
.drawer[data-state="open"]   { animation: slide-in-right  var(--dur-base) var(--ease-out); }
.drawer[data-state="closed"] { animation: slide-out-right calc(var(--dur-base) * var(--exit-ratio)) var(--ease-out) forwards; }
.scrim[data-state="open"]    { animation: fade-in  var(--dur-base); }
.scrim[data-state="closed"]  { animation: fade-out calc(var(--dur-base) * var(--exit-ratio)) forwards; }
.popover[data-state="open"]  { animation: pop-in var(--dur-fast) var(--ease-out); transform-origin: var(--radix-popover-content-transform-origin); }
```

## Appendix D — Playwright snippets

```ts
// reduced-motion variant of a spec
test.use({ reducedMotion: 'reduce' });

// layout shift check
export async function layoutShift(page: Page, ms = 1000) {
  return page.evaluate((t) => new Promise<number>((resolve) => {
    let total = 0;
    new PerformanceObserver((list) => {
      for (const e of list.getEntries() as any[]) if (!e.hadRecentInput) total += e.value;
    }).observe({ type: 'layout-shift', buffered: true });
    setTimeout(() => resolve(total), t);
  }), ms);
}

// CPU throttle for performance specs
const client = await page.context().newCDPSession(page);
await client.send('Emulation.setCPUThrottlingRate', { rate: 4 });
```
