# Surface: 3D view

The perspective, orbitable viewport showing the **mass**. Interactive for **push/pull** (drag the
top cap to set height); otherwise a read-only mirror of the canonical volume. Shows massing solids,
not framing, in the MVP. Canonical language owner:
[`apps/web/CONTEXT.md`](../../../apps/web/CONTEXT.md) "3D view", "Mass", "Push/pull". Code:
`apps/web/src/three-view.tsx`, `apps/web/src/mass-tessellation.ts`.

_Use "3D view" and "mass" — avoid "model view", "scene", "block", "box". The gesture is "push/pull"
— avoid "extrude" (the kernel verb), "drag", "pull-up"._

## Imperative, by decision

This is an **imperative Three.js renderer** mounted into a React container via ref+effect; React
owns no per-frame state ([ADR 0005](../../../docs/adr/0005-frontend-application-stack-react-vite.md)).
A mount-once effect builds the scene (camera, lights, grid, OrbitControls, the render loop); a
rebuild effect disposes and recreates the mass mesh when the canonical mirrors change. **Don't move
per-frame or scene state into React** — it belongs in the imperative `SceneHandle`. Dispose
geometries/materials on rebuild and unmount (`disposeMass`); a leak here is a real defect.

## The mass mesh is a presentation tessellation

The mesh is built from `footprint + height` (`rebuildMass`) — it **holds no geometry of its own**
(ADR 0006/0008). It is translucent walls + a **separate, named `top-cap` mesh** + wireframe edges.
The top cap is its own mesh so push/pull picking can identify it directly. If you restyle the mass,
keep the top cap visually and structurally identifiable as the interactive face.

## Push/pull

- Active only with the Push/Pull tool **and** an existing mass. A pointer-down that raycasts the top
  cap starts a drag; vertical pointer movement maps (via `pushPullDistance`) to a signed tick delta
  dispatched as `PushPull { volumeId, TOP_FACE, distance }`.
- **A live distance readout follows the cursor during the drag** (`three__readout`, fed by
  `hud.ts`'s `pushPullReadout`): the resulting mass height plus the signed distance (`▲`/`▼`), in
  feet/inches — so the cap isn't dragged blind. This is the first brick of the 3D HUD layer
  ([ADR 0012](../../../docs/adr/0012-tool-chrome-framework.md)); it is pointer-inert and hidden when
  not dragging. It is a *readout*, not a typed **input** (see coverage gaps).
- **Reference the engine's named face, never a guessed normal.** `TOP_FACE` is the kernel's named
  face index (`crates/geometry-kernel/.../brep.rs`); the code confirms the world normal is vertical
  as a sanity check, but the source of truth is the named face (ADR 0008 §3). Don't pick by
  normal-heuristic alone.
- **Freeze orbit during the drag** (`controls.enabled = false`) and restore it on release. A new drag
  interaction must do the same so gestures don't fight.
- Distance is **signed** — dragging down is negative. A non-positive resulting height renders no mass
  (`rebuildMass` guards `height > 0`) and the engine now **rejects** the push with a reason surfaced
  in the toast (`non_positive_height`), rather than silently doing nothing.

## Camera framing — preserve the user's context

`frameView` re-frames the camera on the mass centroid, but **only when the footprint geometry
changed** (a fresh draw), tracked by `footprintSig`. A **height-only push/pull must not re-frame** —
the user may be mid-orbit, and yanking the camera is disorienting. This is a settled product
decision; preserve it. The view opens at an angled orbit (`camera.position.set(24, 22, 28)`) so the
mass reads as a solid.

## Coverage gaps (don't claim these work)

- **No selection / no hover affordance** — there's no indication the top cap is grabbable until you
  try, and no other pickable element.
- **Typed height entry.** The value box (`ValueBox`, docked when the active tool's `value` grammar is
  `height`, ADR 0012 §4) accepts an absolute height in feet/inches and commits it as a signed push/pull
  delta. Push/Pull is enabled as soon as a **closed footprint** exists (`footprintVertices >= 3`), so
  the **first extrude** works on a flat face by *either* dragging the top cap *or* typing a height — it's
  not gesture-only. (It used to be gated on `hasMass`, which was a bug: you need push/pull to make the
  first mass, so it can't require one.) Invalid input is **not silent** — an unparseable height raises a
  toast telling the user how to phrase it (`store.flagRejection`).
- **No any-face push/pull** — top-cap vertical only by decision (ADR 0007 §3); don't imply otherwise.
- **No framing/studs** — massing solids only in the MVP.

See `coverage-gaps.md` before designing into any of these.
