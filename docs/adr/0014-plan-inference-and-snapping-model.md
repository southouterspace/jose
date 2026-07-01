# ADR 0014 — Plan inference/snapping is resolved screen-space in a view-side module

- **Status:** Proposed
- **Date:** 2026-07-01
- **Context doc:** [`docs/plans/inference-engine.md`](../plans/inference-engine.md); the prioritized item is
  [`docs/analysis/sketchup-tools-analysis.md`](../analysis/sketchup-tools-analysis.md) §1.1 / §3 (P1 #5).
  Mirrors the selection model of [ADR 0013](./0013-selection-model.md); rides the plan camera of P0 #2 and
  the command contract of [ADR 0008](./0008-mvp-geometry-and-command-contract.md).

## Context

SketchUp's crown jewel is its **inference engine** — continuous, colored, modeless snapping. Jose had a
partial base living inside `tool-runner` (`inferAlignment`: from-point row/column alignment) plus a 1in grid
snap and a `Shift` axis-lock, all resolved in **world ticks** inside `ToolRunner.draft`. Finishing the
snapping (endpoint / midpoint / on-edge point snaps now; on-axis / parallel / perpendicular / locks next)
raises one question that must be answered once, before the snap taxonomy grows: **where does snapping get
resolved, and in what coordinate space** — because every future snap kind and cue inherits that answer.

Two forces frame it:

- **A snap must *feel* the same at every zoom.** A world-tick tolerance balloons when zoomed in and shrinks
  to sub-pixel when zoomed out. Selection already solved this by resolving in **screen space** through the
  `PlanCamera` (ADR 0013). Snapping is the same problem and wants the same answer.
- **`tool-runner` is pixel-free by boundary rule.** It emits `Command` intents in ticks and never sees
  pixels or a camera. A screen-space resolver cannot live there without breaking that boundary — and the
  colored markers + badges are pure presentation, which also don't belong in the runner.

## Decision

1. **Snapping is resolved in a pure, screen-space, view-side module — `plan-snap.ts`** (apps/web),
   alongside `plan-selection.ts` and `plan-camera.ts`: no React, no DOM, unit-tested directly.
   `resolveSnap(camera, ring, pending, screenPoint)` projects the committed footprint and the in-progress
   picks through the camera and returns the snapped **world** point plus the snap **kind** (for the cue).

2. **It supersedes `tool-runner`'s `inferAlignment` as the home of snapping.** The runner keeps its role —
   the commit grammar (picks → `Command`, ticks only) — and its grid snap / axis-lock / ring-close in
   `draft()`. The richer geometric inference moves to the view module; the from-point alignment folds in as
   a later phase. Snapping now has **one** home, at constant screen tolerance.

3. **The view commits the snapped point through the existing `pick({ exact: true })` path.** That is the
   same channel typed-length entry already uses to say "take this point as-is." So the engine still receives
   only exact ticks, `tool-runner` stays pixel-free, and the client holds no second model (ADR 0008).

4. **Priority is fixed and total, so the snapped point never jitters:** an explicit **lock** (`Shift` /
   arrow, next phase) > **point snaps** (endpoint → midpoint → on-edge) > **linear inference** (next phase)
   > the **grid** fallback. A point pins both axes exactly; a linear inference only constrains one, so points
   win. Tolerances are screen px, `point ≥ edge` (a corner beats the edge through it), matching `hitTest`.

5. **Cues are transient presentation, and never color-only.** A colored marker (endpoint square, midpoint
   diamond, on-edge ✕) plus a text **badge** ("Endpoint"/"Midpoint"/"On Edge") render over — never into —
   the canonical footprint. The **shape and badge** carry the meaning without hue, since there is no design-
   token layer yet (`coverage-gaps.md`).

## Consequences

- **The inference engine has one seam to grow on.** On-axis, parallel/perpendicular, intersection, and the
  arrow/Shift locks all become new cases in `resolveSnap` + new cue kinds — no re-litigating where snapping
  lives or what space it runs in.
- **Constant felt tolerance, for free.** Because resolution is screen-space over the `PlanCamera`, a snap
  behaves identically at every zoom — the same property selection and the camera already rely on.
- **`tool-runner` stays a clean commit grammar.** Moving inference out keeps the boundary crate pixel-free;
  the view owns pixels, cameras, and cues, as it already does for selection.
- **No schema, boundary, or top-level change.** Snapping touches only `apps/web`; like selection, it is a
  front-end structure (hence an ADR) that adds no directory and no dependency-direction change.

## Alternatives considered

- **Keep inference in `tool-runner`, fed a camera-derived tick tolerance.** Rejected: it either leaks pixels
  into the pixel-free boundary crate or passes a per-frame tolerance *and* the committed ring into
  `draft()`, bloating the runner — and the cues (pure presentation) still can't live there. The view-side
  module is the same shape selection already validated.
- **Resolve snapping in world space.** Rejected for the same reason ADR 0013 rejected world-space hit-testing:
  a fixed world tolerance can't feel constant across zoom levels.
- **Skip the badge; rely on marker color alone.** Rejected: color-only cues fail the interface guidelines and
  there is no token layer to lean on; the badge also names inferences (parallel/perpendicular) that have no
  distinct point marker.
