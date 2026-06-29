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

## Coordinates and units

- World is in **ticks**; the view maps `PX_PER_TICK = 0.05` over a fixed `640×640` viewBox, with the
  world origin offset so the drawing area sits in view (`ORIGIN_TICKS`). World Y is up; screen Y is
  down (`sy`/`wy` invert it). Keep these transforms paired — a new readout must convert back through
  `wx`/`wy`, not assume screen pixels.
- The grid is **1ft spacing** (`GRID_TICKS = 384`) across a `±7680`-tick half-extent. Anything the
  user displays should be feet/inches, not ticks (`copy.md`).

## What to get right here

- **Closing the ring must be discoverable.** The status bar tells the user "click the first to
  close"; keep that copy and the close tolerance honest with each other.
- **A ring is ≥ 3 vertices.** Below that it stays transient. Don't promote a 1- or 2-pick polyline to
  a footprint.
- Snapping picks to a sensible increment is intended (the plan calls for it) but not all implemented
  — see coverage gaps.

## Coverage gaps (don't claim these work)

- **No pan/zoom.** The viewport is a fixed window on the world; a footprint drawn outside the
  `±7680`-tick span renders off-view with no way to reach it.
- **No snapping / no dimensions readout.** Picks land at the raw cursor tick; there's no grid snap,
  no length/angle guide, and no editable dimension. The plan intends snapping; it isn't there yet.
- **No footprint editing after close.** You can draw a new ring, but there's no vertex drag, insert,
  or delete on a committed footprint.
- **No degenerate-geometry feedback.** A self-intersecting or zero-area ring isn't flagged client-side.

See `coverage-gaps.md` and `resilience.md` before designing into any of these.
