# ADR 0009 — Parametric junction detailing (corners & lapped plates)

- **Status:** Accepted
- **Date:** 2026-06-29
- **Context doc:** [`docs/plans/repo-scaffold.md`](../plans/repo-scaffold.md) §4; builds on the `building` context's `Junction`/`FramingSolver` and ADR 0002

## Context

The `building` context already models a corner connection — `Junction`, `JunctionType::Corner`,
`JunctionMethod::{ThreeStud, California, TwoStudClip}`, the shared-post count, and an
owner-frames-once rule (`is_owner`) — and `FramingSolver::frame_wall` *can* place junction posts
when given `Junction`s. But two gaps make corners non-functional today:

1. **Nothing detects junctions.** `bim-core/session.rs` always frames a wall with an empty junction
   slice (`frame(..., &[], ...)`), so corner posts and plate laps never get emitted end-to-end.
2. **Corner detailing isn't parameterized for reuse.** We want a user-settable *default* corner
   type that the system applies, parametrically, at every detected corner — a repeatable connection
   between two walls — with per-junction overrides, and we want it to stay cheap to recompute on edit.

We also resolved the geometry model: a framing member is stored as a compact **recipe** (axis +
length + orientation + a shared section spec), and its 8 box corners are *derived* on demand
(render, clash, takeoff) — not stored.

## Decision

1. **Derive, don't store — corners are a rule, not saved geometry.** A `Junction` persists only its
   *parameters* (the participating walls, topology, orientation, method). The corner posts and the
   lapped top-plate assignments are produced by a **pure `JunctionDetailer`** function
   (`detail(junction, wall_sections, method) -> posts + plate-lap assignments`) and materialised into
   the existing `MemberPlacement` SoA buffer. No new buffer; junctions stay engine-internal (a
   `Junction` buffer is added only if/when JS needs them — YAGNI).

2. **Flyweight + per-junction transform.** The detailer computes members in **junction-local
   coordinates**; the result is placed by the junction's transform. One parametric definition serves
   every corner of the same class (e.g. a rectangle's four outside corners) — only the transform
   differs. `detail()` is pure and memoisable on `(method, sense, wall sections, angle)`, so
   identical corners reuse one computation. This is the repo's flyweight / reference-by-key idiom.

3. **Detection is pure geometry.** `detect_junctions(&[Wall]) -> Vec<Junction>` finds shared/abutting
   baseline endpoints and classifies each junction:
   - **Topology** — `Corner` (two walls, an end-to-end turn) vs `Tee` (a wall abutting another
     mid-span). `Cross` is **deferred**.
   - **Sense** — **Outside** (convex) vs **Inside** (concave), computed from the signed turn between
     the wall directions relative to each wall's **interior face** (the outward-framing rule: the
     drawn footprint is the interior face). In/out is *derived from the drawing*, never hand-tagged.
   - **Owner** — the wall with the **lower id** owns the shared members. Deterministic, so framing is
     **stable across recomputes** (same input → same studs).

4. **Orientation is derived; the method is the user's choice.** The user does not place studs or tag
   corners in/out. They set a **defaults table keyed by junction class**, and the system applies the
   matching method to every detected junction, with a per-`Junction` **override**:
   - default `outside → California`, `inside → ThreeStud`, `tee → ladder-block backing`.
   The defaults table is a flyweight config (project default + overrides), referenced by key like the
   other catalogs.

5. **The lapped double top plate is the same rule, one level up.** The `JunctionDetailer` also stamps
   the plate laps: per top-plate course it picks which wall runs *through* and which *butts*, and
   **staggers** the two courses (course 0 → owner through; course 1 → flips). Posts and plate laps
   come from one detailer so they cannot drift.

6. **Wholesale recompute for v1; architected for incremental.** Re-detail + re-frame the whole model
   on edit (it is O(walls + junctions) integer-tick math). Member rows are **owned per wall/junction**
   (contiguous SoA spans) so a future incremental dirty-set recompute can re-stamp only affected
   spans without changing the contract.

7. **v1 scope: `Corner` + `Tee`.** Cross junctions and the cross/tee lap convention beyond
   ladder-block backing are deferred. New enums (`JunctionType`, `CornerSense`) and any new member
   roles are **MODEL-defined** (`schema/` → `bun run codegen`); `building` owns detection + the
   detailer (reached via its `lib.rs` facade); `bim-core` composes them into the `Session` pipeline.

## Consequences

- The canonical model stays tiny (junction params, member recipes); the SoA buffer is the
  *materialised output*, and the 8 box corners / solids are a *view* computed on demand.
- The first implementation slice is the **junction detector** — the missing foundation — followed by
  wiring detected junctions into `frame_wall`, adding plate-lap detailing, and a plan-view visualizer.
- Moving a wall re-runs the recipes for affected members and the relevant junction rules; it never
  find-and-updates loose 3D points.
- "Set a default corner type → repeatable parametric detailing" is realised as the defaults table +
  the pure detailer; "give the user choice" is the per-junction override and the per-class defaults.

## Alternatives considered

- **Store the exploded corner studs (and/or all 8 corners per member).** Rejected: ~3× the data,
  multiple sources of truth to keep in sync on every edit, and no parametric reuse. Derive-on-demand
  is smaller and self-consistent.
- **Ask the user to tag each corner inside/outside (and handedness).** Rejected: the drawing already
  determines it; hand-tagging is tedious and drifts from geometry. Derive it; expose method choice
  instead.
- **A separate junction/clash solver that snaps stud corner-points together.** Rejected: corners are
  produced by *placing* parametric members from a rule anchored at the shared point; faces meet by
  construction. Point-coincidence is the result, not the constraint to solve.
