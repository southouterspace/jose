# ADR 0015 — Footprint editing is an `EditFootprint` command that re-extrudes at the current height

- **Status:** Proposed
- **Date:** 2026-07-01
- **Context doc:** [`docs/plans/footprint-editing.md`](../plans/footprint-editing.md); the prioritized item is
  [`docs/analysis/sketchup-tools-analysis.md`](../analysis/sketchup-tools-analysis.md) §3 (P2 #9). Builds on
  the selection model of [ADR 0013](./0013-selection-model.md), the snapping model of
  [ADR 0014](./0014-plan-inference-and-snapping-model.md), and the command contract of
  [ADR 0008](./0008-mvp-geometry-and-command-contract.md).

## Context

Selection (P0 #3) can pick a vertex / edge / footprint, but nothing downstream consumes it — a drawn
footprint is immutable. P2 #9 makes it editable: **drag a vertex** to move it, **drag an edge** to
insert a vertex, **Delete** a selected vertex to remove it. One question is load-bearing and must be
answered before any of it: **how does an edit cross the boundary without flattening the mass?**

The trap is real. A footprint is drawn flat and later lifted into a mass by push/pull; the engine
holds the ring in `Session::footprint` and the extruded solid in `Session::volume`. `DrawFootprint`
deliberately **resets the volume to `None`** (`session.rs::draw_footprint`) so a *new* drawing starts
flat — a contract locked by `redraw_footprint_replaces_and_resets_to_flat`. Re-sending `DrawFootprint`
with the mutated ring would therefore **flatten an already-extruded mass** — the opposite of editing
it. Editing needs a command that mutates the ring **and preserves the current height**.

## Decision

1. **A new `EditFootprint { vertices }` command, carrying the whole mutated ring.** It is
   `DrawFootprint`-shaped (a closed world-XY tick ring) and crosses the same ABI
   (`editFootprint(xs, ys) -> String`: `""` on accept, a `RejectReason::code` on refusal). It is a
   sibling of `DrawFootprint`, not a replacement.

2. **`Session::edit_footprint` re-extrudes the edited ring at the *current* mass height.** It rejects
   when there is nothing to edit (`NoTarget`), validates the ring with the existing `validate_ring`
   (so a self-crossing / zero-area / <3-vertex edit is refused with its reason and leaves state and
   history untouched), and then:
   - if a mass exists, re-extrudes the **new** ring at `volume.height` — computed on a candidate
     first, so a kernel refusal can't half-apply the edit (the same discipline `push_pull` uses);
   - if the footprint is still a flat face (no volume), updates the ring and stays flat.
   An accepted edit records history, so **undo is free** (it routes through `Session::apply` like
   every other mutation, P0 #1).

3. **The three verbs are client-side ring transforms; only the committed ring crosses.** Move / insert
   / delete are computed on the render side against the index-keyed `FootprintMirror` (ADR 0013),
   applied *transiently* during the drag (the pattern of `pendingPicks` while drawing), and only the
   resulting ring is sent as `EditFootprint`. The engine stays the source of truth — it validates and
   re-extrudes; the client never mutates canonical geometry, it proposes a new ring and re-reads the
   mirror the engine ships back (the one-direction rule).

4. **No new reject reasons, no new nouns.** `validate_ring`'s existing `TooFewVertices` / `ZeroArea` /
   `SelfIntersecting`, plus `NoTarget`, cover every degenerate edit, and the rejection copy is already
   in `rejection.ts`. The client also blocks the one obvious case up front — a `Delete` that would
   drop a 3-vertex ring to 2 — so the common mistake never round-trips.

## Consequences

- **The mass reshapes, it doesn't collapse.** Editing a pushed footprint changes the solid's plan and
  keeps its height — the visible acceptance test for P2 #9.
- **Minimal boundary growth.** The wasm shell, worker, protocol, and tool-runner `Command` union each
  gain one arm that mirrors the `drawFootprint` / `pushPull` ones — no new marshaling shape and no
  per-op enum to keep in lock-step across two languages.
- **Selection clears on the post-edit recompute** (ADR 0013 §4). Re-deriving the moved/renumbered
  index so a selection survives an edit is a deferred nicety, not required for the MVP.
- **No schema or top-level change.** `EditFootprint` is a command variant and a `Session` method; it
  touches no `BufferLayout` column and no dependency direction. Per the repo rule this is still a
  contract change, hence this ADR.

## Alternatives considered

- **Preserve-height semantics on `DrawFootprint`.** Rejected: the engine could not tell a brand-new
  footprint from an edit of the current one, so it would either flatten edits or refuse to reset on a
  genuine redraw — breaking the flat-on-redraw contract. Two intents need two commands.
- **A semantic edit op in the command** (`EditFootprint { MoveVertex | InsertVertex | DeleteVertex }`).
  Rejected as more surface for less: it adds an op-enum that must stay in lock-step across the wasm
  boundary, to move authority the client already holds (it owns the index-keyed ring and the
  hit-test). The whole-ring form reuses `validate_ring` and the `drawFootprint` ABI verbatim.
- **A whole-footprint drag (move every vertex) as the first verb.** Rejected as out of scope — that is
  P2 #10 (Move / Copy + array), which also introduces copy and linear-array grammar; #9 is the
  per-vertex reshape that the selection model was built to enable.
</content>
