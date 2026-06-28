# ADR 0007 — Space-first modeling: footprint + push/pull as the canonical flow

- **Status:** Accepted
- **Date:** 2026-06-28
- **Context doc:** [`docs/plans/repo-scaffold.md`](../plans/repo-scaffold.md) §5; builds on the `geometry-kernel` extrude/push-pull verbs; reframes the MVP scope of [ADR 0006](./0006-world-space-placement-engine-side.md)

## Context

The Phase 4 slice wired a **wall-first** flow: a `DrawWall` command promotes a `Wall` from a
baseline `Segment` (`bim-core/session.rs`) and the `FramingSolver` emits members into the SoA
buffer. The initial drawing UX targets a different, **SketchUp-style** modeling loop: draw a 2D
**space footprint** in plan, **push/pull** it into a 3D mass, and *derive* framing from that space.

The `geometry-kernel` already provides the verbs for this: `extrude(profile: Path2D, …) -> Volume`
and `apply_push_pull(volume, op)`. But two facts shape what is reachable:

1. **The building pipeline starts at `Wall`, not at a space.** There is no `Space`/`Room`
   first-class concept; walls promote from a `FaceRef`, floors from a horizontal face.
2. **The kernel is a prism model.** `extrude` materializes only the **base and top** faces
   (`faces: vec![base, top]`) — the vertical side faces do not exist as geometry — and
   `apply_push_pull` accepts **only the top cap** and changes **only height**. Arbitrary-face
   push/pull, face subdivision, and topology changes are not represented.

## Decision

1. **Canonical input is space-first.** The thing the user draws is a **closed footprint**
   (`Path2D`), extruded into a `Volume` via push/pull. This **supersedes `DrawWall` as the MVP's
   front door**; walls and framing become a *derived* layer generated from the volume, not the
   primary input. Command surface ≈ `DrawFootprint(closed Path2D)` + `PushPull(height)`.

2. **Both panes are input surfaces** (this revises the earlier view-composition decision recorded
   in [`apps/web/CONTEXT.md`](../../apps/web/CONTEXT.md), which had the 3D view read-only). **Plan**
   is a 2D CAD surface for drawing/editing the footprint — pure geometry, no framing shown. **3D**
   is interactive for **push/pull**: grab the top cap and drag to set height (`apply_push_pull` on
   `TOP_FACE`). The 3D view therefore needs **face picking** (a raycast). Both panes read the same
   canonical `Volume`; the one-direction rule still holds — gestures → command → engine → buffer →
   both panes re-render.

3. **MVP push/pull is vertical top-cap extrude only.** Footprint → extrude → drag the top cap.
   **General / any-face push/pull** (carving an L- or T-shaped footprint by pushing a wall face,
   recesses, openings, and the topology changes they imply) is **explicitly deferred**: it requires
   the `geometry-kernel` to grow from a prism/extrusion kernel into a **general BREP solid modeler**
   — a separate, large effort that will get its own ADR. "Mirror SketchUp" is the destination,
   reached in stages; this slice is the first stage.

4. **Framing is deferred; the outward-framing rule is recorded now.** The first 3D slice renders
   the **massing volume** (boxes), not studs. When framing lands, **the drawn footprint is the
   interior face** of the framing: the wall assembly offsets **outward** along the face normal, so
   interior clear dimensions are preserved (draw 8×8 → interior stays 8×8; the exterior footprint
   grows by the assembly thickness on each side). This makes member world-placement
   ([ADR 0006](./0006-world-space-placement-engine-side.md)) a post-MVP step.

## Consequences

- The MVP critical path is **footprint geometry in plan → kernel extrude/push-pull → massing
  volume in 3D**. No `FramingSolver`, no member world-transform, and no wall buffer on the MVP path
  (the `WallPlacement`-for-plan idea explored during design is dropped).
- The engine exposes (a) **footprint/edge geometry** for the plan view and (b) the **volume's
  faces** for the 3D massing view — likely new SoA buffer(s) through `schema/model/`; their exact
  shape is the next thing to grill.
- The **3D view gains an input role** (face picking / raycasting), a new responsibility for the
  render-mirror / tool-runner layer beyond read-only rendering.
- `DrawWall` and the wall-first pipeline are **not deleted** — they remain for the later derived-
  framing layer — but they are off the MVP front door.

## Alternatives considered

- **Keep wall-first drawing for the MVP.** Rejected: it is not the modeling paradigm intended;
  drawing the *space* (and deriving framing) is the point.
- **General any-face push/pull in the MVP.** Deferred: it needs a general BREP solid modeler, out
  of scope for a first slice. Captured as the explicit next kernel phase.
- **Keep the 3D view read-only, with push/pull as a numeric height field.** Rejected: that is a
  height input with a live preview, not "push/pull of the 3D space"; it loses the SketchUp feel
  that is the heart of the MVP.
