# ADR 0012 — The first framing slice: world-space members composed at the root

- **Status:** Accepted
- **Date:** 2026-06-29
- **Context doc:** implements the deferred principle of [ADR 0006](./0006-world-space-placement-engine-side.md) within the boundary rules of [ADR 0003](./0003-wasm-boundary-and-the-buffer-layout-keystone.md); reuses the junction detailing of [ADR 0009](./0009-parametric-junction-detailing.md); extends the single-recompute discipline of [ADR 0008](./0008-mvp-geometry-and-command-contract.md)

## Context

The space-first MVP ([ADR 0007](./0007-space-first-modeling-footprint-push-pull.md)/[ADR 0008](./0008-mvp-geometry-and-command-contract.md))
draws a footprint and push/pulls it into a mass. Framing was deferred — yet the engine already
*has* it: the `building` context's `FramingSolver` + `frame_walls`/`detect_junctions` produce
plates, an anchored OC stud grid, opening framing, and corner detailing (posts + lapped double top
plates, [ADR 0009](./0009-parametric-junction-detailing.md)), all tested. None of it was reachable
from the app: nothing derived walls from a mass, and members were only ever emitted in **wall-local**
coordinates ([ADR 0006](./0006-world-space-placement-engine-side.md) deferred the wall→world
transform off the MVP path). This ADR puts framing on the front door and un-defers ADR 0006.

## Decision

1. **Drawing a footprint frames its perimeter.** In the same recompute that writes the `footprint`
   and `volume` buffers, `bim-core`'s `Session` derives one bearing `Wall` per footprint edge (at the
   mass height, 16in OC), runs `detect_junctions` over the set, frames each wall, and writes the
   `MemberPlacement` buffer. Push/pull re-frames in lockstep — a taller mass yields taller studs —
   so one model feeds the plan view, the 3D mass, and the 3D framing with no divergence
   (ADR 0008 §5, extended to members).

2. **The wall→world transform is composed at the composition root, not the renderer.** This is the
   ADR 0006 decision, now realized: `bim-core` maps each wall-local member onto its wall's world
   baseline (`a + x·û + z·ẑ`). Corner posts are the one exception — the junction detailer already
   resolves them to world plan coordinates, so they pass through. The JS side performs **no**
   canonical geometry math; it tessellates world segments into display boxes only (ADR 0006 §2).

3. **No schema change.** The `MemberPlacement` buffer already declares a full 3D segment
   (`x0,y0,z0 → x1,y1,z1`, `width`, `roleId`); world placement *populates* those columns rather than
   adding any, so the `LAYOUT_HASH` is unchanged and the drift gate stays green. The wall-local note
   in `buffer-layouts.json` is the only thing that moves (a comment).

4. **The 3D view renders members as solid boxes inside a faint shell.** Each member becomes a
   `BoxGeometry` oriented along its world segment and colored by role; the mass shell fades to a hint
   once framed, while the top cap stays the pickable push/pull face.

## Consequences

- The headline capability — *parametric residential framing* — is finally visible end to end, and
  every downstream context (loads, sizing, takeoff, cut plans) now has real members to act on.
- World placement is single-homed at the root; a future reviewer who reaches for "just transform in
  the renderer" is pointed back at ADR 0006 §1 and here.
- Non-axis-aligned walls round `û` to the tick (integer SoA); exact for the common rectilinear case.
- **Through-wall depth stays nominal.** Members render with a square cross-section from the `width`
  column (ADR 0006 §2's sanctioned box tessellation); a distinct depth column and true 2x section are
  a clean follow-up when fidelity demands it.
- **Openings/interior walls remain out.** The slice frames the closed perimeter only; doors/windows
  and partition walls are the next increments (the solver already frames openings — they need a
  command surface).

## Alternatives considered

- **Make the `FramingSolver` emit world coordinates directly.** Rejected: it would rewrite the
  solver's wall-local contract and its whole test suite for no gain — composition is the root's job.
- **Add a world-placement column / face buffer now.** Rejected (YAGNI): the existing 3D segment
  columns already carry world coordinates; adding columns would move the `LAYOUT_HASH` for nothing.
- **Studs + plates only, defer corners.** Rejected: junction detailing is already built and tested
  (ADR 0009); shipping it is what makes the corners read as a real framed building.
