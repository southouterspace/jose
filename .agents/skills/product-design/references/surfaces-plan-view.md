# Surface: plan view

The top-down (world XY), orthographic, 2D CAD viewport — the surface for **drawing and editing the
footprint**. Pure geometry; no framing or studs are shown here. Canonical language owner:
[`apps/web/CONTEXT.md`](../../../apps/web/CONTEXT.md) "Plan view" and "Footprint". Code:
`apps/web/src/plan-view.tsx`.

_Use "plan view" — avoid "2D view", "top view", "floorplan". The thing drawn is a "footprint" —
avoid "outline", "polygon", "sketch", "perimeter"._

## The draw interaction

- While the footprint tool is active, a pointer-down adds a ring vertex (`store.pick`). A click near
  the first vertex closes the ring and fires a `DrawFootprint` command. Other tools' picks are
  ignored here.
- **The mid-draw polyline is transient client state**, not geometry: `store.pendingPicks` renders as
  a dashed polyline (`plan__pending`) with vertex dots. Only the **closed ring** crosses into the
  engine ([ADR 0008](../../../docs/adr/0008-mvp-geometry-and-command-contract.md)). Never send picks
  command-by-command, and never render raw picks as if canonical.
- After the command returns, the committed footprint renders **from the `FootprintMirror`** (the
  engine's canonical ring), as a solid polygon (`plan__footprint`) — never from the raw clicks. This
  is the one-direction rule made visible: dashed = mine-in-progress, solid = the engine's truth.

## Snapping / inference (P1 #5, ADR 0014)

- While drawing (footprint or rectangle), the cursor **snaps** to existing geometry: a footprint/pending
  **endpoint** (green square), an edge **midpoint** (cyan diamond), or the nearest point **on an edge**
  (red ✕), each named by a **snap badge** ("Endpoint" / "Midpoint" / "On Edge"). The shape + badge carry
  the meaning — never color alone (no token layer yet, `coverage-gaps.md`).
- The cursor also infers **on-axis** — running the segment within a few degrees of a world axis pins it
  onto that axis (red **X** line / green **Y** line + "On Axis"). **Locks** turn inference into a
  constraint: **Shift** locks the dominant axis; the **arrow keys** (`→` = X, `↑` = Y; `←`/`↓` release)
  lock an axis explicitly, and the badge reads "On Axis — locked". Locks + on-axis are **footprint-only**
  (a rectangle is already axis-aligned); the arrow lock releases when the draw ends.
- Snapping is resolved in a pure, **screen-space** module (`plan-snap.ts`), so a snap feels the same at
  every zoom (like selection). Priority: **lock → point snap → on-axis → grid**; a resolved snap commits
  **exactly** via `pick({ exact: true })`, bypassing the runner's own grid/axis handling.
- **Deferred:** parallel / perpendicular to *arbitrary* edges (low value in an orthogonal framing tool —
  axis-aligned edges' parallels already coincide with on-axis) and intersection snaps. The existing
  from-point row/column alignment guides still render as the free-cursor fallback.

## The rectangle tool (P2 #8)

- The **rectangle tool** (shortcut `R`) draws a footprint from **two opposite corners**: click the
  first corner, move (a dashed axis-aligned box rubber-bands from it, `plan__rubber-rect`), click the
  opposite corner. It emits the *same* closed `DrawFootprint` ring the polyline does — the fast path
  for the rectangular common case. Winding doesn't matter (the engine validates on unsigned area).
- Its value box is a **`W,D` size** grammar (`parseSize`): after the first corner, type `24', 16'`
  (comma / `x` / `×` separated) to place the opposite corner exactly, grown toward the cursor's
  quadrant (`rectangleCorner`). The running width×depth readout doubles as its live size.
- Both plan draw tools share the same pick/draft plumbing (`isPlanDraw` in `plan-view.tsx`); only the
  preview shape and the value grammar differ. A rectangle is already axis-aligned, so Shift-axis-lock
  is a footprint-only gesture.

## Coordinates and units

- World is in **ticks**; the plan maps them into a fixed `640×640` viewBox through a **`PlanCamera`**
  (`plan-camera.ts`) held in view state — a `scale` (px per tick) plus an `offsetX`/`offsetY`. World Y
  is up, screen Y is down. Every rendered coordinate goes through `toScreenX`/`toScreenY`; a readout
  that needs the world point under a pointer must invert through `toWorldX`/`toWorldY` (or the
  `worldOf` helper), not assume screen pixels.
- The view is **navigable** (P0 #2): scroll zooms toward the cursor, middle-drag pans, and the **Fit**
  button / **Shift+Z** run Zoom-Extents (framing the footprint + in-progress picks). The camera clamps
  its zoom so the view can't invert or vanish.
- The grid is **1ft spacing** (`GRID_TICKS = 384`) across a `±7680`-tick half-extent, redrawn per
  camera. Anything the user displays should be feet/inches, not ticks (`copy.md`).

## The measurement HUD (P1 #6/#7)

- The live draw shows a **length + angle** readout trailing the cursor (`plan__dim`, via
  `hud.ts`'s `segmentReadout`) — e.g. `12' 0"  45°`. The angle is the segment's plan bearing, degrees
  CCW from world +X (east 0, north 90), matching what the value box's polar entry types back.
- The committed footprint carries **persistent per-edge length labels** (`plan__edgelen`, centered on
  each edge midpoint) and a **running width×depth** readout pinned to the top-left corner
  (`plan__extents`) — the overall size updates live from the in-progress picks while drawing.
- The value box accepts an optional **`< angle` clause** for polar entry: `10' 6" < 45` places the next
  vertex at that length *and* absolute bearing (`parsePolarLength` / `pointAtAngle`). A bare length still
  runs along the cursor direction, and Shift+Enter still axis-locks it. All feet/inches, never ticks.

## The select interaction (P0 #3)

- The **select tool** (shortcut `S`) picks a piece of the committed footprint under the cursor — a
  **vertex**, an **edge**, or the whole **footprint** — resolved by a pure screen-space `hitTest`
  (`plan-selection.ts`) in priority order vertex → edge → face. Hover previews what a click would pick;
  a primary click commits it; a click on empty space (or `Esc`) clears.
- **Selection is presentation state**, held in the store and keyed by ring index — never engine
  geometry ([ADR 0013](../../../docs/adr/0013-selection-model.md)). It is cleared on every recompute,
  so it can't outlive the ring it names. This is the precondition for footprint editing (still to come).
- Cues render **over** the canonical footprint: the committed selection warm and solid, the hover cue
  quieter and dashed. The status bar names what's picked ("Selected an edge — Esc to clear"). Use the
  canonical nouns — *vertex*, *edge*, *footprint*, *selection* (`apps/web/CONTEXT.md`) — not
  point/node/handle or side/segment.

## What to get right here

- **Closing the ring must be discoverable.** The status bar tells the user "click the first to
  close"; keep that copy and the close tolerance honest with each other.
- **A ring is ≥ 3 vertices.** Below that it stays transient. Don't promote a 1- or 2-pick polyline to
  a footprint.
- Snapping picks to a sensible increment is intended (the plan calls for it) but not all implemented
  — see coverage gaps.

## Coverage gaps (don't claim these work)

- **No angle *inference guide*.** The live readout now shows the segment's angle and typed polar
  entry (`< angle`) exists (P1 #6/#7), but the inference engine still doesn't *snap* to angles
  (e.g. parallel/perpendicular/45°) — only to existing vertices' rows/columns. And committed
  dimensions are read-only labels: there's no click-a-label-to-edit yet.
- **No footprint editing after close.** Selection (P0 #3) exists, but selecting a vertex/edge doesn't
  yet let you move it — no vertex drag, insert, or delete on a committed footprint.
- **Degenerate geometry is rejected on commit, not previewed.** A zero-area or self-intersecting ring
  is refused with a toast when it closes (P0 #4); there's still no client-side warning *while* drawing.

Pan/zoom + Zoom-Extents (P0 #2) and plan selection (P0 #3), which used to be gaps here, are now built.
See `coverage-gaps.md` and `resilience.md` before designing into the rest.
