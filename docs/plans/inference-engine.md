# Plan — Inference engine v1.5 (finish the snapping) (P1 #5)

SketchUp's crown jewel is its **inference engine**: continuous, colored, *modeless* snapping that lets
you draw precisely without dialogs ([`docs/analysis/sketchup-tools-analysis.md`](../analysis/sketchup-tools-analysis.md)
§1.1, §3 #5). Jose already has a surprisingly good base — from-point axis alignment (`inferAlignment`),
`Shift` axis-lock, ring-close snap, and a 1in grid snap — but it stops well short. This plan finishes
it: **point snaps** (endpoint / midpoint / on-edge), **linear inference** (on-axis / parallel /
perpendicular), **locks** (generalized `Shift` + arrow keys), and **colored point cues + a snap
badge**. Always on, always visual, never a settings dialog.

## What we're building (and not)

**In:**
- **Point snaps** to the committed footprint (and to in-progress picks): **endpoint**, **midpoint** of
  an edge, and **on-edge** (nearest point on an edge). Each pins an exact world point.
- **Linear inference**: **on-axis** (world X/Y through the anchor) surfaced *without* holding `Shift`;
  **parallel** and **perpendicular** to an existing edge; and the existing **from-point** row/column
  alignment, folded in.
- **Locks**: `Shift` generalized to lock *whichever inference is currently shown* (axis, parallel, or
  perpendicular), and **arrow keys** to lock an axis (→ = world-X, ↑ = world-Y; press again / `Esc`
  releases) — SketchUp's "turn a suggestion into a constraint" gesture.
- **Cues**: a colored point marker at the snapped point (shape + color per kind), the inference line(s)
  in play, and a single **snap badge** naming the winning inference ("Endpoint", "Midpoint", "On Edge",
  "On Axis", "Parallel", "Perpendicular") — the snap/inference badge deferred from the measurement HUD
  (§2b.1).
- **Constant felt tolerance** at any zoom (screen-space), matching selection (P0 #3) and the camera
  (P0 #2).

**Out (deferred, by decision):**
- **3D snapping** — plan only, as selection was (P0 #3). The mass/faces come later.
- **Full edge–edge geometric intersection** — only the practical intersection of *two active inference
  lines* (e.g. on-axis-X meets another vertex's row) is in; arbitrary edge crossings are not.
- **Tangent / center / from-face** — no arcs or circles in a framing tool (analysis §4, YAGNI).
- **Snap on/off settings UI** — inference is modeless; there is no dialog (the whole point).

## Decisions this plan rests on

- **Snapping is resolved in a pure, screen-space, view-side module** — `plan-snap.ts` (apps/web),
  mirroring [`plan-selection.ts`](../../apps/web/src/plan-selection.ts) and `plan-camera.ts`: no React,
  no DOM, unit-tested directly. This is where constant felt tolerance lives (it has the camera), and
  where colored cues + badges (pure presentation) belong. It **supersedes `tool-runner`'s
  `inferAlignment`**, consolidating all snapping in one place. Recorded as
  [ADR 0014](../adr/0014-plan-inference-and-snapping-model.md), the way selection's was
  ([ADR 0013](../adr/0013-selection-model.md)).
- **The engine stays the source of truth; snapping is presentation.** The module resolves the raw
  cursor to a snapped world point, and the view commits it through the **existing `pick({ exact: true })`
  path** — the same channel typed-length entry already uses. So `tool-runner` stays pixel-free (its
  boundary rule) and owns only the commit grammar; the client holds no second model
  ([ADR 0008](../adr/0008-mvp-geometry-and-command-contract.md)).
- **Cues are transient client state**, never rendered as canonical geometry (the one-direction rule).
  Meaning is carried by the **badge text + marker shape**, not color alone — there is no design-token
  layer yet (`coverage-gaps.md`), and color-only cues would fail the interface guidelines.
- **Priority is fixed and total**, so the snapped point never jitters between candidates: **an explicit
  lock** (`Shift`/arrow) > **point snaps** > **linear inferences** > **grid fallback**. A point snap
  pins both axes exactly; a linear inference only constrains one, so points win. Two active linear
  inferences *combine* into their intersection point (the practical intersection snap).

## The snap taxonomy (resolved in priority order)

| Kind | Pins | Cue marker | Line | Badge |
|---|---|---|---|---|
| **Lock** (Shift / arrow) | one axis/direction | — | solid axis (red=X, green=Y) or dashed parallel/perp | "On Axis (locked)" etc. |
| **Endpoint** | exact point | green square | — | "Endpoint" |
| **Midpoint** | exact point | cyan diamond | — | "Midpoint" |
| **Intersection** (two inference lines) | exact point | ✕ | both lines | "Intersection" |
| **On-edge** | exact point on an edge | red ✕ on the edge | — | "On Edge" |
| **On-axis** (from anchor) | X or Y through anchor | — | solid red (X) / green (Y) | "On Axis" |
| **Parallel** (to an edge) | direction | — | dashed magenta | "Parallel" |
| **Perpendicular** (to an edge) | direction | — | dashed magenta | "Perpendicular" |
| **From-point** (share a vertex's row/col) | X or Y | — | dashed (existing `plan__guide`) | "On Axis from point" |
| **Grid** (fallback, 1in) | rounded point | — | — | (none — the quiet default) |

Colors reuse the existing ad-hoc palette (X=red, Y=green mirror SketchUp; the red on-edge ✕ and the red
X-axis line are different mark types, as in SketchUp). Every color is redundant with the badge + shape.

## Reachable states (what the module must cover)

- **No committed footprint, first pick** — grid fallback only (nothing to snap to yet).
- **No footprint, ≥1 pending pick** — on-axis + from-point + endpoint(pending) + parallel/perp to the
  in-progress edges.
- **Committed footprint present** — plus endpoint / midpoint / on-edge of the real ring.
- **Lock active** — the locked line shows prominently; competing inferences are suppressed.
- **Rectangle tool** — the two corners snap to endpoint/midpoint/on-edge/on-axis too (parallel/perp are
  moot for an axis-aligned box); both plan draw tools already share the pick/draft path (`isPlanDraw`).

## Phases (each keeps `main` green)

1. ✅ **Pure `plan-snap.ts` module.** The `Snap` union (kind + snapped world point) and
   `resolveSnap(camera, ring, pending, cursor)` with the point-snap priority. Reuses screen-space
   `projectToSegment` (extracted from `plan-selection.ts`). Unit-tested with no React.
2. ✅ **Point snaps + cues.** Endpoint / midpoint / on-edge to the committed ring + pending picks;
   colored markers (shape+color) + the badge; commit via `pick({ exact: true })`. Works for both plan
   draw tools (footprint + rectangle). `inferAlignment`'s from-point guides still render alongside for
   now; they fold into the module in phase 3.
3. ✅ **Linear inference + locks.** On-axis inference (no `Shift` needed), the generalized `Shift`-lock
   (dominant axis), and arrow-key axis lock (`→`/`↑`), with red-X / green-Y axis lines + the badge
   ("On Axis", "On Axis — locked") via `resolveDraw`. **Parallel / perpendicular to arbitrary edges is
   deferred** (YAGNI: an orthogonal framing tool's edges are axis-aligned, so their parallels already
   coincide with on-axis). `inferAlignment`'s from-point guides are **kept as the free-cursor fallback**
   rather than retired — folding them in fully is deferred with parallel/perp.
4. **Polish + sync (as needed).** Hysteresis if the point proves jittery, a badge copy pass (`copy.md`),
   and — if ever justified — parallel/perpendicular with arbitrary-angle guide clipping.

## Smaller engineering calls (mine to make; recorded so they're not re-litigated)

- **Tolerances (screen px, constant at zoom):** point-snap ~10px (≥ the on-edge ~6px so a corner beats
  its edges, as `hitTest` already does), linear-inference band ~6px.
- **Grid stays the fallback, not a competitor.** Inference snaps override the 1in grid; the grid only
  applies when nothing else fires, so free-drawing still lands on clean inches.
- **Perf is a non-issue.** Residential rings are tiny (< ~30 vertices); projecting + testing all snaps
  per pointer-move is negligible. No spatial index needed.
- **Accessibility:** the badge + marker shape carry meaning without color; no reliance on hue alone.
- **Complexity budget:** `plan-view.tsx` is near the cognitive-complexity gate — the cue rendering and
  the snap wiring go through extracted helpers/components (as the HUD and rectangle work did).
- **One badge at a time** — the winning snap only; never a stack of competing labels.

## Acceptance criteria

- Hovering near a committed vertex shows a green endpoint marker + "Endpoint"; a click lands *exactly*
  on it. Midpoint → cyan + "Midpoint"; on-edge → red ✕ + "On Edge".
- Drawing near-horizontal shows the red X-axis line + "On Axis"; the click lands exactly on-axis.
  Pressing → locks X until released; the badge shows the lock.
- An edge drawn parallel/perpendicular to an existing one shows the cue + "Parallel"/"Perpendicular".
- Tolerances feel identical zoomed in and out.
- Cues never render on committed geometry as if canonical, and every cue's meaning survives without
  color (badge + shape).
