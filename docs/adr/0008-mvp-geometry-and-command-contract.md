# ADR 0008 — The MVP geometry & command contract (space-first slice)

- **Status:** Accepted
- **Date:** 2026-06-28
- **Context doc:** [`docs/plans/drawing-ux-mvp.md`](../plans/drawing-ux-mvp.md); implements the space-first flow of [ADR 0007](./0007-space-first-modeling-footprint-push-pull.md) within the boundary rules of [ADR 0003](./0003-wasm-boundary-and-the-buffer-layout-keystone.md) / [ADR 0006](./0006-world-space-placement-engine-side.md)

## Context

[ADR 0007](./0007-space-first-modeling-footprint-push-pull.md) settled *what* the MVP does (draw a
footprint in plan → push/pull into a mass in 3D, framing deferred). This ADR settles the **engine
contract** that realises it: what crosses the wasm boundary, what the two panes read, and how a
push/pull gesture stays honest about the one-direction rule. A throwaway JS prototype validated the
interaction loop and surfaced the one real constraint: a push/pull gesture must reference a face the
**engine** owns, not a face the renderer guesses at (the prototype detected the top cap by a surface-
normal heuristic; the kernel already names faces — `BASE_FACE = 0`, `TOP_FACE = 1`).

## Decision

1. **Two thin SoA buffers, declared in `schema/model/buffer-layouts.json` (codegen as usual).**
   - A **`footprint`** buffer: world-XY vertex columns in ticks, grouped by a `spaceId` (which ring
     a vertex belongs to). This is the plan view's entire data source.
   - A **`volume`** (mass) buffer: `volumeId`, `height` (ticks), and the base-plane reference — the
     scalars that, with the footprint, define the extruded solid.
   Both move under the existing `LAYOUT_HASH` drift gate; no hand-edited offsets.

2. **JS tessellates the 3D mass for display; the engine owns the geometry.** The 3D view builds the
   box mesh from `footprint + height` (presentation tessellation, exactly the
   [ADR 0006](./0006-world-space-placement-engine-side.md)-sanctioned move — the 2D analogue is
   stroking a line). Canonical geometry remains the engine's footprint + volume; the renderer holds
   no second source of truth.

3. **Push/pull references a canonical face.** Picking the top cap in 3D resolves to the kernel's
   `TOP_FACE`; the command carries `PushPull { volumeId, faceIndex, distance }`, and
   `GeometryKernel::apply_push_pull` validates it (rejecting any non-top face in this slice). The
   renderer never invents a face — it names one the engine defined.

4. **Command surface is `DrawFootprint` + `PushPull`.** `DrawFootprint { vertices }` carries a
   *closed* ring; `PushPull` carries the face + signed distance. The **in-progress footprint**
   (a polyline mid-draw, before it closes) is **client-only UI state** — it is not canonical and
   crosses no boundary until it closes into a ring. The engine only ever sees closed footprints.
   `DrawWall` and the wall-first path remain in the engine for the later framing layer, off the MVP
   front door.

5. **One recompute feeds both panes.** A command produces one buffer rewrite; plan and 3D both
   re-read the same mirror. No per-pane geometry divergence.

## Consequences

- The MVP buffer surface is small and maps 1:1 onto what the kernel already produces (base + top
  faces; no side faces needed yet). Plan = footprint columns; 3D = footprint + height tessellated.
- Push/pull is correct-by-construction against the boundary: the gesture's authority is a
  `volumeId + faceIndex` the engine validates, not a renderer guess.
- **Side faces stay absent** (the kernel's `extrude` emits only base + top). The day a wall face must
  be *pushed* (carving an L/T footprint), this contract grows an explicit **face buffer** and the
  kernel grows side-face generation — the same BREP-modeler phase ADR 0007 already deferred.
- Framing remains deferred; member world-placement ([ADR 0006](./0006-world-space-placement-engine-side.md))
  stays off the MVP path.

## Alternatives considered

- **Emit an explicit face buffer now** (engine-authored faces with stable ids; no JS tessellation).
  Rejected for the MVP: it forces side-face generation immediately for no MVP benefit, and inflates
  the buffer. Adopt it when any-face push/pull lands.
- **Let the renderer keep its own footprint/volume model** (tessellate and pick purely client-side).
  Rejected: it reintroduces a second source of truth and a normal-heuristic for picking, both of
  which the boundary rules (ADR 0003/0006) exist to prevent.
- **Make the in-progress footprint canonical** (stream each vertex to the engine). Rejected as
  needless chatter across the boundary; an open polyline is transient UI, not domain state.
