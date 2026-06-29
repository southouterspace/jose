# Coverage gaps

Where Jose has **no product-design standard yet**. This list keeps missing guidance *visible* so the
skill never silently invents taste to fill a hole. The honest move when you hit a gap: handle it in
scope if you can, and **say so** in the review/PR; don't claim a guarantee the product doesn't make.

A gap graduates to a real standard only with evidence and human acceptance (`AGENTS.md` governance) —
then move it into a reference/rule/exemplar and delete it here.

## States and resilience

- **No error / rejected-command state.** A failed engine load, a rejected `DrawFootprint`/`PushPull`,
  or a `LAYOUT_HASH` mismatch (`assertLayout`) has no user-facing treatment. No decision yet on what
  the user sees, whether input is preserved, or how they recover. (`resilience.md`.)
- **No permission/empty-billing/auth states.** There is no account or persistence surface in the MVP
  (`apps/api` is orthogonal). Nothing to standardize yet.
- **No undo / redo.** Nothing reverses a draw or a push/pull. This raises the bar for any destructive
  action (`rule/destructive-names-action`) — don't ship one assuming undo exists.
- **No degenerate-geometry feedback.** Self-intersecting, zero-area, or out-of-bounds footprints
  aren't flagged client-side; the engine is the authority but its rejection isn't surfaced.

## Responsive and input

- **No compact/stacked breakpoint.** The `1fr 1fr` viewport grid crushes on narrow windows. No
  decision on whether panes stack, tab, or one becomes primary.
- **Touch ergonomics unverified.** Pointer gestures work, but target sizes, the ring-close tolerance,
  and pinch/zoom on touch haven't been validated.

## Plan view

- **No pan/zoom** — fixed window on the world; off-view geometry is unreachable.
- **No snapping or dimension guides** — picks land at the raw cursor tick; the plan intends snapping,
  it isn't built.
- **No post-close footprint editing** — no vertex drag/insert/delete.

## 3D view

- **No selection or hover affordance** — the grabbable top cap isn't signaled until you try it.
- **No numeric height input** — gesture-only.

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
