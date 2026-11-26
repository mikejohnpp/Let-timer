# Motion Fixes — Agent Handoff

> Audience: implementing agents. Baseline: branch `dev` @ `1f9524b` (builds, 251 Vitest tests pass).
> Read first: `AGENTS.md`, `docs/motion_plan.md` (spec A01–A32, tokens, accessibility), this file.
> Decision DM6 stays as recorded: the **`motion` library is the animation library of this app**. Do not remove it and do not work on bundle size (the app runs locally; owner decision).

## 0. Why this handoff exists

A review of the implementation found that three animation engines are layered on the same elements (Motion `layout`/`initial`/`exit`, the custom `useFlip` hook, and CSS keyframe classes), which explains the series of "fix flicker / prevent duplicate animation" commits, and that several rules of `docs/motion_plan.md` are not met:

| # | Finding (evidence) | Task |
|---|---|---|
| F1 | `MotionProvider`: mode `full` maps to `reducedMotion="user"`, so Motion still obeys the OS "reduce motion" setting while the CSS side does not; ease is hard-coded | T1 |
| F2 | 15 raw `duration: 0.x` and 16 `ease: [...]` in TSX instead of tokens; `npm run lint` is only `tsc --noEmit` | T1 |
| F3 | `App.tsx`: outer `<div key={currentView} className="view-enter">` remounts the subtree, so the `AnimatePresence` exit never plays; enter animates twice (CSS 180ms plus Motion 150ms) | T2 |
| F4 | `TodayView` rows use Motion `layout` **and** `useFlip` **and** the CSS class `task-row-add`; no `layoutId`/`LayoutGroup`, so a row moving between groups fades out and fades in instead of travelling (spec A17/A18) | T3 |
| F5 | `FloatingPlayerIsland` combines CSS classes `island-card`/`island-pill` with Motion `initial/animate/exit` on the same elements | T4 |
| F6 | `animate-ping` (infinite) on the "Now Running" dot (`TodayView`) and in `Badge`; breaks the calm budget and WCAG 2.2.2 | T5 |
| F7 | `motion.test.ts`: 136 `toContain` checks on source text and 0 renders; no Playwright/axe although the plan requires them | T6 |
| F8 | Dead or duplicated code after the switch (`useFlip`, `Presence`, CSS keyframes) | T7 |

## 1. Rules for everyone

1. **One engine per element.** An element may be animated by exactly one of: Motion props (`initial/animate/exit/layout/layoutId`), a CSS animation/transition class, or the WAAPI `animate()` wrapper. Never combine them on the same element or on the same CSS property of one element.
2. **Ownership policy** (also goes into `docs/motion_plan.md` §8 and `AGENTS.md` in T7):

| Concern | Engine |
|---|---|
| Mount/unmount of views, list rows, toasts, island, workspace/media lists, shared thumb | Motion (`AnimatePresence`, `layout`, `layoutId`) |
| Radix overlays (drawer, modal, popover, menu, tooltip) | CSS keyframes on `data-state` |
| State tints, hover, press, progress bar, date slide, tick draw, running dot, ring pulses, equalizer | CSS transitions/keyframes |
| Imperative one-offs (wallpaper crossfade) | `animate()` wrapper |

3. **No raw timings or easings in TSX** (`duration: 0.2`, `ease: [..]`, `cubic-bezier(` outside `styles/motion.css` and `motion/tokens.ts`). Use the helper from T1.
4. Only `transform`, `opacity` and colour are animated. Allowed exceptions: row delete (A21, height collapse) and the Completed disclosure (height, new A33 in T7).
5. Reduced and Off modes must keep working: Reduced = fades of at most 100ms, no travel, no loops; Off = zero durations (tests).
6. Do **not** change the visual design, copy or layout. Do not add dependencies except the dev dependencies named in T6.
7. Every task ends with `npm run lint && npm test && npm run build` green and a manual check of `/dev/motion` and the affected screens in **Full**, **Reduced**, **Off** (and Full with the OS set to reduce, see T1).
8. One task = one commit (or PR) with a message `fix(motion): …` listing the finding ids.

Execution order: **T1 → T2 → T3 → T4 → T5 → T6 → T7** (T1 is required by T2–T5).

---

## T1 — Motion foundation: provider mapping, token helper, lint (F1, F2)

**Files**: `web/src/motion/tokens.ts`, `motion/MotionProvider.tsx`, new `motion/useMotionTransition.ts`, `motion/index.ts`, every file listed below, `web/package.json`, new `web/scripts/check-motion-tokens.mjs`.

**Do**

1. `tokens.ts`: add `mode: 'system' | 'reduced' | 'full' | 'off'` to `MotionTokens` (read `data-motion`, default `system`). Keep `isReduced`/`isOff`.
2. `MotionProvider.tsx`:
   - Expose tokens through a React context and add `useMotionTokens()` (subscribes to the existing `let-timer-motion-changed` event and the `prefers-reduced-motion` media query, as the provider already does).
   - Map the mode correctly:

```ts
const reducedMotion =
  tokens.mode === 'off' || tokens.mode === 'reduced' ? 'always'
  : tokens.mode === 'full' ? 'never'
  : 'user';                                   // system: follow the OS
```
   - Take the default `transition` from tokens, not literals: duration `tokens.durBase / 1000`, ease parsed from `tokens.easeOut`.
3. New `useMotionTransition(kind, opts?)` (reads `useMotionTokens()` so components re-render on mode change):

```ts
type Kind = 'instant' | 'fast' | 'base' | 'slow' | 'ambient';
export function useMotionTransition(kind: Kind, opts?: { exit?: boolean; ease?: 'out' | 'inout' | 'emph' }) {
  const t = useMotionTokens();
  const ms = { instant: t.durInstant, fast: t.durFast, base: t.durBase, slow: t.durSlow, ambient: t.durAmbient }[kind];
  const eased = { out: t.easeOut, inout: t.easeInout, emph: t.easeEmph }[opts?.ease ?? 'out'];
  return {
    duration: t.mode === 'off' ? 0 : (opts?.exit ? ms * t.exitRatio : ms) / 1000,
    ease: parseBezier(eased),                  // [x1, y1, x2, y2]; add parseBezier + unit test
  };
}
```
4. Replace **every** raw transition (list from the review; re-grep before editing):

| File | Lines (approx.) | Kind |
|---|---|---|
| `App.tsx` | 150 | tab transition (T2) |
| `components/ui/Toast.tsx` | 40 | `base` |
| `components/ui/SegmentedControl.tsx` | 53 | `base` |
| `components/Header.tsx` | 289 | `base` |
| `components/FloatingPlayerIsland.tsx` | 189, 472 | `fast` or `base` (T4) |
| `views/MediaView.tsx` | 400, 449, 670, 744, 877 | `base` |
| `views/TodayView.tsx` | 376, 813 | `base` (T3) |
| `views/WorkspacesView.tsx` | 159, 232 | `base` |

   Exits use `{ exit: true }`.
5. Lint: create `scripts/check-motion-tokens.mjs`, run from `npm run lint` (`"lint": "tsc --noEmit && node scripts/check-motion-tokens.mjs"`). It scans `src/**/*.{ts,tsx,css}` excluding `src/test/**`, `src/motion/tokens.ts`, `src/styles/motion.css`, and fails on: `duration:\s*0?\.\d`, `ease:\s*\[`, `cubic-bezier\(`, `animate-(ping|pulse|bounce)`, `transition-duration:\s*\d`. A line may be allowed with the comment `// motion-allow: <reason>` (reason required). Confirm that `.github/workflows/ci.yml` runs `npm run lint`; add it if missing.

**Tests (Vitest)**: `useMotionTransition` returns token-derived values, exit ratio, `0` in Off; `parseBezier` round-trip; provider: with `data-motion="full"` and `matchMedia('(prefers-reduced-motion: reduce)')` mocked true, a child calling `useReducedMotion()` from `motion/react` gets `false`; `reduced` gets `true`; `system` follows the media query; the lint script exits non-zero on a fixture containing `duration: 0.2`.
**Done when**: no raw timing remains outside the two token files; setting Motion = Full while the OS requests reduced motion makes Motion and CSS animations both run.

---

## T2 — Tab transition: make the exit play and enter once (F3)

**Files**: `web/src/App.tsx`, `web/src/styles/motion.css`, new `components/ViewHost.tsx`.

**Do**

1. Extract the view switch into `ViewHost` (keeps `App.tsx` small and testable). Remove the outer `<div key={currentView} className="view-enter h-full">`. Target structure:

```tsx
<main className="relative z-10 flex-1 …">
  <AnimatePresence mode="wait" initial={false}>
    <motion.div
      key={currentView}
      data-view={currentView}
      initial={{ opacity: 0, y: t.travelS }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0 }}
      transition={enter}
    >
      {renderView(currentView)}
    </motion.div>
  </AnimatePresence>
</main>
```
   with `enter = useMotionTransition('base')` and an `exit` transition of `useMotionTransition('base', { exit: true })` (spec A24: outgoing shorter than incoming). Use `y: travelS` from tokens, no literal `4`.
2. Delete the CSS class `view-enter`, its keyframes and its reduced/off overrides from `motion.css` (grep first; keep the `.wallpaper-layers { view-transition-name: none }` rule).
3. Make sure the wallpaper layers are **outside** `ViewHost` so they never remount or fade.
4. If any view relied on being remounted for scroll reset, call `window.scrollTo(0, 0)` in the `onExitComplete` callback.

**Tests**: Vitest: `ViewHost` renders the current view inside a single `motion.div` with `data-view`, and no ancestor carries `key={currentView}` (render-based, not source-string). Playwright (T6): sample `[data-view]` opacity every 30ms across a tab change; expected: old view fades below 1 and is removed before the new one is visible, and the new one fades in exactly once (monotonic increase).
**Done when**: switching tabs shows a single fade-out then a single fade-in with no double flash; wallpaper never flickers; Reduced shows opacity only.

---

## T3 — Today rows: one engine and real continuity (F4)

**Files**: `web/src/views/TodayView.tsx`, `web/src/components/ui/StatusGlyph.tsx`, `web/src/styles/motion.css`, `web/src/motion/index.ts`.

**Do**

1. **Remove `useFlip`** from `TodayView` (`registerFlip`, the `ref={registerFlip(...)}` on rows, and `listContainerRef` if it only served FLIP). Remove the CSS class `task-row-add` from rows (and `isNewlyAdded` class logic).
2. **Rows**: `motion.div` with `layout="position"` (not plain `layout`, to avoid scale distortion of text), no `scale` in `initial`/`exit`:

```tsx
<motion.div
  key={t.id}
  layout="position"
  layoutId={`task-${t.id}`}
  initial={{ opacity: 0, y: tokens.travelS }}
  animate={{ opacity: 1, y: 0 }}
  exit={removingIds.has(t.id) ? { opacity: 0, height: 0 } : undefined}   // A21 only for real deletes
  transition={moveTransition}                                              // useMotionTransition('slow', { ease: 'emph' })
  data-task-id={t.id}
/>
```
3. **Continuity (A17/A18)**: wrap all four groups (Now running, Overdue, Tasks, Completed) in one `<LayoutGroup id="today-tasks">` so `layoutId` is shared across the groups' separate `AnimatePresence` containers. Rows that **change group** must not run the delete exit: keep a `removingIds` set that contains a task id **only** while a real delete is in progress; moves, completes and restarts leave it empty (so `exit` is `undefined`).
4. **Add highlight**: replace the root-level `.task-row-add` with a child overlay `<span aria-hidden className="row-highlight" />` (absolute, inset 0, one-shot CSS opacity keyframe, 600ms from tokens) rendered only for the newly added id. It animates a different element and property than the row, so it cannot conflict.
5. **Complete (A17)**: the tick draws first (`StatusGlyph animateTick`, CSS, 200ms), then the row moves. If the mutation resolves faster than the tick, delay applying the group change until the tick's `animationend` (or 200ms, whichever is first).
6. **Completed disclosure** (`height: 0 to 'auto'`) may stay but gets `useMotionTransition('base')` and must not wrap rows that carry `layoutId` in a way that clips the shared transition (`overflow-hidden` only on the disclosure body).
7. **Fallback (decision rule)**: if shared layout across the group containers cannot be made clean (visible duplicates, jumps, clipped rows) after a focused attempt, fall back to a single engine at layer L1 (fade/rise, no travel), keep everything else in this task, and record the downgrade in `docs/motion_plan.md` (A17/A18 note). Under no circumstances keep `useFlip` or CSS row animations as a second engine.

**Tests**: Vitest: a task row root has no `task-row-*` class and exposes `data-task-id`; deleting a task adds it to `removingIds` and a move does not (unit-test the small reducer/hook that owns `removingIds`). Playwright (T6): complete a task and sample `[data-task-id]` for 600ms: at no sample are there more than two nodes with that id, after 600ms exactly one, located in the Completed group; start/pause moves between Now running and Tasks end with one node; rapid double-clicks never strand or duplicate a row.
**Done when**: completing or starting a task visibly **travels** to its new group (or, if the fallback was used, fades cleanly) with no flicker; delete collapses; add rises and highlights once.

---

## T4 — Floating island: single engine (F5)

**Files**: `web/src/components/FloatingPlayerIsland.tsx`, `web/src/styles/motion.css`.

**Do**: the expanded card (`key="expanded-island-card"`) and collapsed pill (`key="collapsed-island-pill"`) keep their Motion `initial/animate/exit` (transform origin bottom, `useMotionTransition('fast')`, exits with `{ exit: true }`). Remove the CSS classes `island-card` and `island-pill` from the className strings, and delete `@keyframes island-expand/island-collapse`, `.island-card`, `.island-pill` and their reduced/off overrides from `motion.css`. Keep `AnimatePresence mode="wait"` and make sure the clock position in Focus does not move when the island expands (plan prerequisite 0.1; verify with the CLS check in T6).

**Tests**: Playwright: layout shift during expand/collapse in Focus < 0.01; no element carries both a Motion-driven transform and a CSS animation (check `getAnimations()` on the island root: none).
**Done when**: expand/collapse animates once, smoothly, in all three modes.

---

## T5 — No infinite loops (F6)

**Files**: `views/TodayView.tsx` (the "Now Running" dot), `components/ui/Badge.tsx`, `styles/motion.css`.

**Do**

1. "Now Running" dot: replace `animate-ping` with the finite `.dot-running` (breathe, 4s, 7 iterations) used by the timer pill; no loop after about 28s.
2. `Badge.tsx` pulse variant: remove the `animate-ping` ring. If an "attention" cue is needed, use the finite `.ring-pulse` (2 iterations) or no motion. Update the variant's usages.
3. Keep `animate-spin` only on the `Syncing` icon in `ConnectionBadge` (A30). The `Live` badge stays static.
4. Add a global guard to `motion.css`, in the three blocks that already exist for reduced, system-reduced and off:

```css
:root[data-motion="reduced"] [class*="animate-"],
:root[data-motion="off"] [class*="animate-"] { animation: none !important; }
@media (prefers-reduced-motion: reduce) {
  :root:not([data-motion="full"]) [class*="animate-"] { animation: none !important; }
}
```
   (This also freezes the date-slide class under Reduced, which is the intended behaviour.)
5. The lint from T1 keeps `animate-ping|pulse|bounce` out of the codebase.

**Tests**: Playwright: with a running task on Today, wait 1s and assert that `document.getAnimations()` contains no animation with `iterations === Infinity` other than `.animate-spin` (Syncing), `.skeleton-shimmer` (while loading) and the equalizer bars (while audio plays); with Reduced, no running CSS animations at all.
**Done when**: nothing on Today pulses forever; the Now Running dot settles after about 28s.

---

## T6 — Real tests (F7)

Split into two commits.

### T6a — Vitest behaviour tests

1. In `web/src/test/motion.test.ts`, **delete every test that reads a `.tsx` file and asserts `toContain(...)` on its source text** (for example the `useFlip`, `slide-date-*`, `task-check-tick` source checks). Keep tests that parse `motion.css` and assert structure: token completeness, both reduced blocks identical, Off zeroes durations (reuse `cssTokens.ts` helpers).
2. Add render-based tests with Testing Library: `MotionProvider` mapping (T1), `useMotionTransition` (T1), `ViewHost` (T2), the `removingIds` logic (T3), `Presence` replaced or removed (T7), `StatusGlyph` adds the tick class only when `animateTick` is true, `Toast` stack renders newest at the correct position and removes after timeout (fake timers).
3. Add a small test helper `renderWithMotion(ui, { mode, osReduced })` that sets `data-motion`, mocks `matchMedia`, wraps in `MotionProvider`.

### T6b — Playwright and axe

1. Dev dependencies: `@playwright/test` pinned to **the same version as nixpkgs' `playwright-driver`** (see `docs/playwright_mcp_nix.md`; check with `nix eval --raw nixpkgs#playwright-driver.version`), and `@axe-core/playwright`. Scripts: `"e2e": "playwright test"`. Browsers come from Nix (`PLAYWRIGHT_BROWSERS_PATH`); never download.
2. `web/e2e/` with a global setup that starts `let-timer-daemon` on a throw-away DB and socket (`LET_TIMER_DB`, `LET_TIMER_SOCKET`, HTTP on a random local port), creates a token and seeds demo data (tasks in every status, one without estimate, a media list).
3. Specs (each runs in default mode; those marked R also run with `test.use({ reducedMotion: 'reduce' })`):

| Spec | Assertion |
|---|---|
| `tabs` (T2) | opacity sampling shows one fade-out then one fade-in; wallpaper node unchanged (same element, no opacity change) |
| `task-move` (T3) | complete/start/pause: node count over time, final location, no duplicates; R: row moves instantly with no transform |
| `overlays` | drawer/popover/modal open and close; focus returns to the trigger; R: no non-`none` transform during open |
| `mode-override` (T1) | OS reduced plus Motion = Full: the drawer shows an in-flight `translateX`; OS reduced plus System: it does not |
| `loops` (T5) | no infinite animations except the allowed ones |
| `focus` | opening the player/island leaves the clock position unchanged (CLS < 0.01); controls fade after 3s and are still reachable with Tab; any key restores them |
| `a11y` | axe: 0 serious/critical on every view, with an overlay open and closed |

4. Add `npm run e2e` to CI if the runner has Nix browsers; otherwise document how to run it locally in `docs/deploy.md` or `web/README`. Playwright traces and videos on failure go to `web/.design/e2e/` (git-ignored).

**Done when**: `npm test` has no source-text assertions; `npm run e2e` passes locally in Full and Reduced.

---

## T7 — Cleanup and documentation (F8)

1. After T3/T4: delete `motion/useFlip.ts` and its export; delete `motion/Presence.tsx` if no longer imported (`grep -rn "Presence" src` excluding `AnimatePresence`); delete `motion/animate.ts` only if nothing but `WallpaperLayers` uses it, otherwise keep it and document that it is for imperative one-offs. Remove the corresponding tests.
2. Delete unused CSS from `motion.css`: `task-row-add`, `task-row-delete` (not referenced in TSX), `island-*`, `view-enter`, and any reduced/off override that referenced them.
3. Update `docs/motion_plan.md`:
   - §8: replace the architecture with the ownership table from section 1 of this file and the rule "one engine per element".
   - DM6: keep, and add "own `useFlip` removed in favour of Motion layout".
   - A17/A18: note the implementation (`LayoutGroup` + `layoutId`) or the L1 fallback if T3 used it.
   - A24: enter `--dur-base`, exit `--dur-base × --exit-ratio` (the 90/180ms figures are replaced).
   - Add **A33 Completed disclosure** (height, allowed exception) and list the two height exceptions in section 3.
4. Add to `AGENTS.md` a "Motion rules" block: the ownership table, "no raw timings in TSX (`npm run lint` enforces)", "no infinite loops", "run `/dev/motion` and `npm run e2e` before merging motion changes".
5. Update `/dev/motion` (`DevMotion.tsx`) so each demo uses the same code path as the app (tokens helper, Motion for lists), and add a scenario "Complete a task" and "Start a task" that exercise T3.

**Done when**: `grep -rn "useFlip\|task-row-\|island-card\|island-pill\|view-enter" web/src` returns nothing; documentation matches the code.

---

## 2. Acceptance checklist (whole handoff)

- [ ] `npm run lint && npm test && npm run build` pass; `npm run e2e` passes locally.
- [ ] Motion = Full overrides an OS "reduce motion" setting for both Motion and CSS; Reduced and Off behave as specified.
- [ ] No raw durations/easings in TSX; the lint script is part of `npm run lint` (and CI).
- [ ] Tab change: one fade-out, one fade-in, no flash, wallpaper unaffected.
- [ ] Task start/complete/pause: rows travel between groups (or the documented L1 fallback), never duplicate or strand; delete collapses; add rises with one highlight.
- [ ] Floating island expands/collapses once; the Focus clock never moves (CLS < 0.01).
- [ ] No infinite loops except Syncing spinner, skeleton shimmer while loading and equalizer while playing.
- [ ] Tests assert behaviour, not source text; axe reports no serious/critical issue.
- [ ] Dead code and docs updated (T7).

## 3. Explicitly out of scope

Bundle size or code-splitting of `motion` (local use); any new animation not listed in `docs/motion_plan.md`; visual redesign; changes to API or backend.

## 4. Open points for the owner

| # | Point | Default used in this handoff |
|---|---|---|
| H1 | If shared layout across groups cannot be made clean, accept the L1 fallback for task moves | Yes, with a note in the plan |
| H2 | Tab transition timing | Enter `--dur-base` (200ms), exit 140ms (70%), replacing the 90/180ms in the plan |
| H3 | Run Playwright in CI | Local only until Nix browsers are available in CI |
