# ADR 0013 — Selection is presentation state, keyed by ring index, cleared on recompute

- **Status:** Proposed
- **Date:** 2026-07-01
- **Context doc:** [`docs/plans/selection-model.md`](../plans/selection-model.md); the prioritized item is
  [`docs/analysis/sketchup-tools-analysis.md`](../analysis/sketchup-tools-analysis.md) §3 (P0 #3); rides the
  tool-chrome framework of [ADR 0012](./0012-tool-chrome-framework.md) and the plan camera of P0 #2.

## Context

Selection is "the precondition for *all* editing" (the analysis §1.8): vertex drag, edge move, and
move/copy (P2) each need a *thing that is currently picked*, and a future properties panel needs to read
it. Before building those, one question has to be answered once: **where does the selection live, and what
does it point at**, so the editing verbs that follow all share one answer.

Two forces frame it:

- **The one-direction rule (ADR 0003 / 0008).** Canonical geometry lives in the engine's SoA buffers; the
  render side never mutates it. A *selection* is not geometry — it is a transient pointer into geometry,
  owned by the UI. Putting it in the engine would invert the boundary for no benefit.
- **The mirror is rebuilt every recompute.** Each `space` snapshot yields fresh `FootprintMirror` bytes;
  a selection that names "vertex 3" is only meaningful against the ring that was on screen when it was made.

## Decision

1. **Selection is presentation state, in the store — never in the engine.** It sits beside `activeTool`
   in `useEngineStore`, exposed as `selection` + `select()` / `clearSelection()`. The engine and the wasm
   boundary are untouched: no command, no buffer, no `Session` field. This is the same discipline that
   keeps render out of the domain, applied to selection.

2. **A selection is a small tagged union, keyed by ring index.**
   ```ts
   type Selection =
     | { kind: "footprint"; spaceId: number }   // the whole face
     | { kind: "vertex";    index: number }      // a ring vertex
     | { kind: "edge";      index: number };     // edge i → (i+1) mod n
   ```
   It references the *current* `FootprintMirror` by position, not by identity — there are no stable entity
   ids on the render side, and the ring is small.

3. **Hit-testing is screen-space, with a fixed priority.** `hitTest(camera, vertices, screenPoint)`
   projects the ring through the `PlanCamera` (P0 #2) into viewBox pixels and resolves **vertex → edge →
   face**, vertex tolerance ≥ edge tolerance. Screen-space keeps the pick tolerance constant at every zoom;
   the priority makes corners forgiving and lets a click inside the ring fall through to the face. The
   function is pure (`plan-selection.ts`), unit-tested like the camera.

4. **Selection persists across tool switches but is cleared on every recompute.** Switching tools keeps the
   selection (SketchUp: select, then pick an edit tool). A new `space` snapshot clears it — the safe rule
   while selection has no edit consequence yet; when P2 edits land they re-derive it deliberately rather
   than inheriting a possibly-stale index.

5. **Select is a tool-chrome entry, not a runner tool.** Like `pushpull`, the Select tool emits no
   `Command`, so it is `runnerBacked: false` and never reaches `ToolRunner`; the store's `activate` already
   routes non-catalog keys to UI state. Adding it is one registry row — the ADR 0012 framework paying off.

## Consequences

- **P2 editing has its seam.** Vertex drag / edge move / delete now have a defined thing to act on; each
  becomes "read `selection` → emit an edit `Command`" without re-litigating where selection lives.
- **It extends to 3D for free-ish.** Because selection is store-level, the 3D view can contribute mass /
  face selections later without moving the state; only the union grows.
- **No schema, boundary, or top-level change.** Selection touches only `apps/web`; per the repo rule this
  is a front-end structure, hence an ADR, but it introduces no new directory and no dependency-direction
  change.
- **Index-keying is a known trade.** A selection is only valid against the ring it was made on, so it is
  cleared on recompute. Stable ids would survive edits, but the render side has none by design (the mirror
  is positional); revisit if editing ever needs selection to survive a recompute.

## Alternatives considered

- **Selection in the engine `Session`.** Rejected: it inverts the one-direction rule (render state leaking
  into the domain) and buys nothing — no domain logic needs to know what the user has highlighted.
- **Stable entity ids on the render side.** Rejected as premature (YAGNI): the ring is positional and
  small, clearing-on-recompute is sufficient for the MVP, and inventing an id vocabulary is a schema-level
  decision to defer until edits actually need selection to outlive a recompute.
- **Hit-test in world space.** Rejected: a fixed world tolerance would shrink to sub-pixel when zoomed out
  and balloon when zoomed in; screen-space tolerance is what makes picking feel the same at every zoom.
