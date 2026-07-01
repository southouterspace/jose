# Coverage gaps

Where Jose has **no product-design standard yet**. This list keeps missing guidance *visible* so the
skill never silently invents taste to fill a hole. The honest move when you hit a gap: handle it in
scope if you can, and **say so** in the review/PR; don't claim a guarantee the product doesn't make.

A gap graduates to a real standard only with evidence and human acceptance (`AGENTS.md` governance) —
then move it into a reference/rule/exemplar and delete it here.

## States and resilience

- **Partial rejected-command state.** A rejected `DrawFootprint`/`PushPull` now surfaces: the engine
  returns a `RejectReason` code, the worker relays it as a `rejected` message, and the store raises a
  dismissible, auto-expiring **toast** (`role="alert"`, copy in `rejection.ts`). Unparseable value-box
  entries route through the same path (`store.flagRejection`). **Still uncovered:** a failed engine
  load and a `LAYOUT_HASH` mismatch (`assertLayout` throws with no user-facing treatment); the toast
  is also the only rejection surface — there's no inline field-level validation. (`resilience.md`.)
- **No permission/empty-billing/auth states.** There is no account or persistence surface in the MVP
  (`apps/api` is orthogonal). Nothing to standardize yet.
- **Undo / redo exists (space history).** The `Session` keeps a bounded space-state history;
  `DrawFootprint` and `PushPull` are undoable via the toolbar buttons and Cmd/Ctrl+Z / Shift+Z (Ctrl+Y).
  **Still uncovered:** the legacy `DrawWall` path doesn't participate, there's no history affordance
  beyond button enablement, and no multi-space model to reverse across. Destructive actions still can't
  *assume* undo covers them (`rule/destructive-names-action`).
- **Degenerate-geometry feedback exists at draw time.** Footprints with too few vertices, zero area
  (collinear), or a self-crossing boundary (`Path2D::is_simple`) are rejected **before** they become
  canonical, with a specific toast per reason. **Still uncovered:** out-of-bounds/overflow rings and
  any *warning* (vs hard reject) treatment; there's no client-side preview of the rejection while
  drawing (it fires on commit).

## Responsive and input

- **No compact/stacked breakpoint.** The `1fr 1fr` viewport grid crushes on narrow windows. No
  decision on whether panes stack, tab, or one becomes primary.
- **Touch ergonomics unverified.** Pointer gestures work, but target sizes, the ring-close tolerance,
  and pinch/zoom on touch haven't been validated.

## Plan view

- **Pan/zoom + Zoom-Extents exist** (P0 #2) — scroll zooms to the cursor, middle-drag pans, Fit /
  Shift+Z frames the drawing, all through a stateful `PlanCamera`. **Still uncovered:** touch
  pinch-zoom, and the plan has no explicit zoom-level readout.
- **Snapping is mostly there** (P1 #5, ADR 0014) — the cursor snaps to **endpoint / midpoint / on-edge**,
  infers **on-axis**, and takes **Shift / arrow-key axis locks**, all with colored cues + badges
  (`plan-snap.ts`), over a 1in grid + from-point alignment fallback. **Still uncovered:** parallel /
  perpendicular to *arbitrary* (non-axis) edges and intersection snaps — deferred as low-value in an
  orthogonal framing tool.
- **Selection exists** (P0 #3) — the select tool picks a vertex/edge/footprint with hover + Esc-clear
  (`plan-selection.ts`, ADR 0013). **Still uncovered:** multi-select, window/crossing box-select, and
  the editing it unlocks — no vertex drag/insert/delete yet.

## 3D view

- **No selection or hover affordance** — the grabbable top cap isn't signaled until you try it, and
  plan selection (P0 #3) does not extend to the mass/faces in 3D yet.
- **Typed height entry lands only after a mass exists** — the value box (`ValueBox`, `value: "height"`)
  sets an exact height, but Push/Pull is gated on an existing mass, so the *first* extrude is still
  gesture-only. Invalid typed input now raises a toast (`store.flagRejection`) instead of being
  silently dropped.

## Design system

- **No design tokens / component library.** Color, spacing, and type are ad-hoc CSS in `app.css`
  (a dark palette, `system-ui`). There is no Geist-equivalent and no token layer. Until there is,
  reuse the existing palette and flag when you need more — don't invent a parallel scale.
- **No theming** beyond the single dark scheme.

## Deterministic checks

- **No Jose-specific lint rules.** Only the Ultracite/Biome preset runs (it already gates the
  `[biome]`-tagged accessibility/anti-pattern rules in `web-interface-guidelines.md`).
  Mechanically-checkable *product* rules (e.g. `display-feet-not-ticks`, a canonical-noun check) and
  the `[gap]`-tagged web rules (`outline: none` without a focus replacement, `transition: all`,
  literal `...`, `user-scalable=no`) are reliable custom-check candidates but **are not built** —
  deferred until there's real surface to check (ADR 0011). `rules.md` and `web-interface-guidelines.md`
  carry them as judgment for now.

## Process

- **No evidence-intake loop running.** The `tooling/scripts/evals/` structure exists as scaffolding
  (fixtures + a rules checklist), but there is no weekly Slack/Figma/PR intake job and no eval runner
  wired into CI. The eval harness is a documented starting point, not an automated gate.
- **The agent reviewers run in-session, not in CI.** `/web-interface-guidelines` and
  `/react-best-practices` (via `/ci-local`) run locally under the session's Claude auth — no
  `ANTHROPIC_API_KEY`, no Actions job (ADR 0011). The tradeoff: **there is no automatic PR gate** for
  them; coverage depends on the habit of running `/ci-local` before pushing web changes. Not a
  substitute for the offline evals.
