# ADR 0006 — World-space placement is produced by the engine, not the renderer

- **Status:** Accepted — **implementation landed** in [ADR 0012](./0012-framing-slice-world-space-members.md). (Originally deferred off the MVP critical path per [ADR 0007](./0007-space-first-modeling-footprint-push-pull.md); the wall→world composition described here is now realized at the `bim-core` composition root.)
- **Date:** 2026-06-28
- **Context doc:** [`docs/plans/repo-scaffold.md`](../plans/repo-scaffold.md) §5; extends the one-direction rule of [ADR 0003](./0003-wasm-boundary-and-the-buffer-layout-keystone.md)

## Context

The Phase 4 slice emits each framing member in **wall-local** coordinates: `x` along the wall
baseline, `z` up, and `y` (through-wall depth) literally `0`. The `MemberPlacement` buffer note
in [`schema/model/buffer-layouts.json`](../../schema/model/buffer-layouts.json) states plainly
that "composing the wall→world transform is a later pipeline stage." The single elevation
`<canvas>` never needed world placement, so the transform exists nowhere yet.

The initial drawing UX changes that. A **plan view** (top-down) and a **3D view** both require
each member positioned in **world** space — the wall's baseline position, its orientation, and
its through-wall depth composed in. Where that transform runs determines whether this work
touches the engine/schema at all, and whether canonical geometry math leaks onto the render side.

## Decision

1. **The Rust pipeline composes wall→world; the canonical SoA buffer ships world-space
   coordinates.** The world-placement columns are added through
   [`schema/model/buffer-layouts.json`](../../schema/model/buffer-layouts.json) + `bun run codegen`
   (extending `MemberPlacement` so `y` carries real through-wall depth, or adding a world-placement
   buffer), so the Rust offsets, the TS offsets, and the `LAYOUT_HASH` move together under the
   existing drift gate. Both the plan and the 3D view read the **same** zero-copy mirror; the JS
   render side performs **no** canonical geometry math.

2. **The JS side may tessellate world-space segments into display meshes — as presentation
   only.** The 3D view extrudes simple boxes from a member's world-space segment + draw width,
   exactly as the 2D view strokes a line from the same columns. This display tessellation carries
   **no authority**: it is a pixel-level rendering of canonical geometry the engine owns, not a
   second source of truth. This preserves the one-direction rule of
   [ADR 0003](./0003-wasm-boundary-and-the-buffer-layout-keystone.md) — the engine is the sole
   writer of canonical geometry; render is a read-only mirror.

3. **Engine-emitted solids are deferred (YAGNI).** Generating true solids from the
   `geometry-kernel` BREP extrusion kernel and shipping mesh/triangle buffers to the 3D view is a
   defensible later optimization, not part of this slice. It is recorded here as the known next
   step *if and when* box tessellation stops being faithful enough.

## Consequences

- The **first deliverable on the critical path is a schema/codegen/Rust change** — world
  coordinates in the buffer — which blocks both the plan and the 3D panes. The UI work cannot
  start with pixels.
- One snapshot feeds both panes: a single recompute writes the buffer once, and the plan and 3D
  views render the same mirror. There is no per-pane geometry divergence to keep in sync.
- The render/domain boundary is restated for 3D the same way it held for 2D: JS tessellates for
  display, the engine owns geometry. A future reviewer who reaches for "just compute the
  transform in the renderer" is pointed back here.

## Alternatives considered

- **Compose the wall→world transform in JS at render time.** Rejected: it puts canonical
  geometry math on the render side, contradicting [ADR 0003](./0003-wasm-boundary-and-the-buffer-layout-keystone.md),
  and would be reclaimed into the engine later — paying twice.
- **Hybrid: prototype the transform in JS now, migrate to the engine later.** Rejected: the
  temporary version tends to calcify, and the schema/codegen change is small enough to do once,
  correctly, up front.
- **Ship engine-generated BREP meshes immediately.** Deferred, not rejected: box tessellation
  from world-space segments is sufficient for the first 3D view, and the mesh-buffer contract is
  more weight than the slice needs (YAGNI). Revisit when fidelity demands it.
