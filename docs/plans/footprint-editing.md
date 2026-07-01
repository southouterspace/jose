# Plan — Footprint editing (P2 #9)

The jump from **sketch once** to **model**: make a drawn footprint editable in the plan view — **drag
a vertex** to move it, **drag an edge** to insert a vertex on it, **Delete** a selected vertex to
remove it. This is what finally cashes in the Select tool (P0 #3), which today picks a piece but does
nothing downstream. It is the P2 #9 item from
[`docs/analysis/sketchup-tools-analysis.md`](../analysis/sketchup-tools-analysis.md) §3; the
load-bearing call (a new engine command that re-extrudes instead of flattening) is recorded in a new
**ADR 0015**, drafted on approval of this plan.

## What we're building (and not)

**In:** three edit verbs on a *committed* footprint, all through the **Select** tool —
- **Vertex move:** press a ring vertex and drag; the vertex snaps through `plan-snap`; release commits.
- **Vertex insert:** press an *edge* and drag; a new vertex splits the edge at the press point and
  follows the drag; release commits the longer ring.
- **Vertex delete:** with a vertex selected, `Delete`/`Backspace` removes it (guarded to keep ≥ 3).

Plus the engine seam that makes any of it possible: an **`EditFootprint`** command that mutates the
ring **and re-extrudes at the current mass height**, so editing a footprint that's already been
pushed into a mass reshapes the mass instead of flattening it.

**Out (deferred, do not absorb):** whole-footprint / multi-piece **Move / Copy + linear array** (that's
P2 #10, its own plan); 3D / mass editing (dragging faces, edges of the solid); any **framing** (P3);
edge *sliding* (moving a whole edge parallel to itself — a move-two-vertices op we can add later);
re-deriving selection across an edit (cleared on recompute for now, per ADR 0013 §4).

## The load-bearing decision (ADR 0015)

A fresh `DrawFootprint` **carries no volume**, so re-sending it with the mutated ring flattens the
extruded mass: `session.rs::draw_footprint` sets `self.volume = None` (deliberately — a *new* draw
starts flat, and `redraw_footprint_replaces_and_resets_to_flat` locks that in). Editing must **not**
reuse `DrawFootprint`.

**Decision: a new `EditFootprint { vertices }` command that preserves the current height.** It carries
the *whole mutated ring* (same shape as `DrawFootprint`), validates it with the existing
`validate_ring`, and re-extrudes:

```rust
fn edit_footprint(&mut self, edit: EditFootprint) -> CommandOutcome {
    if self.footprint.is_empty() { return Rejected { NoTarget }; }   // nothing to edit
    if let Err(reason) = validate_ring(&edit.vertices) { return Rejected { reason }; }
    // Re-extrude the NEW ring at the CURRENT height (the whole point — don't flatten).
    let next_volume = match self.volume.as_ref().map(|v| v.height) {
        Some(h) => match self.extrude_with(&edit.vertices, h) { Some(v) => Some(v),
                    None => return Rejected { ZeroArea } },   // compute before mutating (no half-edit)
        None => None,                                          // still a flat face
    };
    self.record_history();                                     // undo is free (Session::apply)
    self.footprint = edit.vertices;
    self.volume = next_volume;
    self.rewrite_space_buffers();
    Accepted { member_count: self.buffer.len() }
}
```

Why the *whole ring*, not a semantic op (`MoveVertex{i}` / `InsertVertex{edge}` / `DeleteVertex{i}`):

- **It matches the transient-then-canonical pattern.** The client already owns the ring (the
  `FootprintMirror`) and resolves picks by index (ADR 0013). It applies the edit *transiently* during
  the drag and only the committed ring crosses the boundary — exactly like the mid-draw polyline. The
  engine stays the source of truth; it just validates and re-extrudes.
- **It reuses the boundary verbatim.** `editFootprint(xs, ys) -> String` is `drawFootprint`'s ABI
  (parallel tick columns, a rejection code or `""`), so the wasm shell, worker, and protocol grow by
  one near-identical arm each — no new marshaling shape, no per-op enum to keep in lock-step.
- **The prism model makes it exact.** A vertical extrusion of the edited ring at height `h` *is* the
  reshaped mass; no per-vertex kernel surgery is needed for the MVP solid.

The three verbs are therefore **client-side ring transforms** (move one vertex / splice one in / drop
one), each producing the ring that `EditFootprint` commits. Guards fall out of `validate_ring`: a
move that self-crosses → `SelfIntersecting`; a delete to 2 vertices → `TooFewVertices`; a collapsed
ring → `ZeroArea` — all already surfaced by the rejection toast, with the client blocking the obvious
ones up front (see below). No new `RejectReason`s.

Alternatives weighed (detailed in the ADR): **preserve-height semantics on `DrawFootprint`** (rejected
— it can't tell a brand-new footprint from an edit, and would break the flat-on-redraw contract);
**semantic edit ops in the command** (rejected — more boundary surface and an op-enum to keep in sync,
for authority the client already holds via the index-keyed mirror).

## The interaction model: a click selects, a drag edits

One rule keyed only by *what's under the cursor at press* and *whether the pointer moved*, extending
the Select tool without a new tool or mode:

| Press on… | Release in place (a **click**) | Drag past threshold (a **drag**) |
| --- | --- | --- |
| a **vertex** | select the vertex *(ADR 0013, unchanged)* | **move** that vertex |
| an **edge**  | select the edge *(unchanged)* | **insert** a vertex on the edge, then move it |
| the **face** | select the footprint *(unchanged)* | — *(whole-footprint move is P2 #10)* |
| empty space  | clear selection *(unchanged)* | — |

- **Direct, no pre-select step.** You grab a vertex and drag — it moves (and becomes the selection). A
  click still just selects, so ADR 0013 behavior is intact; the drag is the new affordance. (Requiring
  "select first, then drag" was considered and rejected as needless friction.)
- **Delete** is a key, not a gesture: with a vertex selected and focus outside the value box,
  `Delete`/`Backspace` commits the ring minus that vertex — **unless** the ring has 3 vertices, where
  the client blocks it and flags *"A footprint needs at least 3 corners."* (the existing
  `too_few_vertices` copy) rather than sending a doomed command.
- **Snapping the drag.** The moving/inserted vertex resolves through `plan-snap`
  (`resolveSnap` point snaps: endpoint / midpoint / on-edge) against the **rest of** the ring — the
  dragged vertex is excluded from the candidate set so it can't snap to its own old spot — with the
  tick grid as the fallback and the existing snap cue/badge shown. Shift / arrow axis-lock relative to
  the vertex's CCW neighbor is a small follow-on refinement, noted, not core.
- **Transient then canonical.** During the drag the plan view renders a **preview ring** (the mirror's
  vertices with the one substituted / inserted) and live edge-length labels; the canonical
  `FootprintMirror` is untouched until release. On release we send `EditFootprint` → recompute → the
  new mirror renders. This is the same discipline as `pendingPicks` while drawing.

## Reachable states & copy (Select tool)

Extend `tool-chrome.ts::selectStatus` (owned copy in `product-design/.../copy.md`):

- Nothing selected: *"Select — click to select; drag a vertex to move it, or an edge to add one."*
- Vertex selected: *"Selected a vertex — drag to move, Delete to remove, Esc to clear."*
- Edge selected: *"Selected an edge — drag it to add a vertex, Esc to clear."*
- Footprint selected: *"Selected the footprint — Esc to clear."* (no edit verb yet — P2 #10.)
- Mid-drag cursor: `grabbing`; over a draggable piece: `pointer` (both already in `surfaceCursor`).

## Phases (each keeps `main` green)

1. **Engine command.** Add `Command::EditFootprint(EditFootprint { vertices })` (`command.rs`) and
   `Session::edit_footprint` (`session.rs`) per the decision above, plus an `extrude_with(ring, h)`
   helper (factor out of `extrude_footprint`). Unit tests: edit preserves mass height (the DoD
   guarantee), edit of a flat face stays flat, a self-crossing / collapsing / 2-vertex edit is
   rejected with the right reason and leaves state + history untouched, and undo restores the pre-edit
   ring **and** mass.
2. **Wasm boundary.** `Engine::edit_footprint(xs, ys) -> String` (`bim-wasm/lib.rs`), mirroring
   `draw_footprint`; boundary test for accept (`""`) + a rejection code.
3. **Command type + protocol + worker.** Add `EditFootprintCommand` to the tool-runner `Command`
   union (data shape only — it's a Select-tool *gesture*, dispatched directly like `PushPullCommand`,
   **not** a `ToolRunner` pick), an `editFootprint` `EngineRequest` arm + `"editFootprint"` in the
   rejected union (`protocol.ts`), and the worker arm calling `engine.editFootprint` (`engine-worker.ts`).
4. **Store: the edit dispatch.** `store.editFootprint(vertices)` on `EngineStore`, following the
   `pushPull` shape (build the command, `dispatch`). Selection stays cleared-on-recompute (ADR 0013).
5. **Pure client-side ring transforms.** In `plan-selection.ts` (or a small `footprint-edit.ts`):
   `moveVertex(ring, i, p)`, `insertOnEdge(ring, edgeIndex, p)`, `deleteVertex(ring, i)` → the new
   ring, plus the `< 3` delete guard. Pure, unit-tested (no React), like the rest of the plan modules.
6. **Plan view: the drag gesture + preview.** A `dragRef` state machine mirroring the existing pan
   one (`startPan`/`continuePan`/`endPan`): pointer-down in Select mode hit-tests and arms a pending
   gesture; pointer-move past a small px threshold enters drag mode, resolves the point through
   `plan-snap` (excluding the dragged vertex), and updates the transient preview ring; pointer-up
   commits `store.editFootprint(previewRing)` or, if it never dragged, falls back to the existing
   click-select. Render the preview ring + snap cue while dragging.
7. **Delete key + status.** A global `Delete`/`Backspace` handler (beside the Esc-clears handler,
   skipped while typing) that reads `store.selection`; and the `selectStatus` copy above.

## Smaller engineering calls (mine to make)

- **Whole ring over semantic ops**, and **client-side transforms** — settled above; the client already
  holds the index-keyed ring, so the engine command stays `DrawFootprint`-shaped.
- **Click-vs-drag threshold** (a few viewBox px) disambiguates select from edit on one pointer stream —
  no separate Move tool, matching "make the Select tool useful."
- **Insert point = the press point projected onto the edge** (not forced to the midpoint), so it's
  precise; midpoint is then just a snap the drag can land back on.
- **Exclude the dragged vertex from snap candidates** so it snaps to *other* geometry, never itself.
- **Selection clears on the post-edit recompute** (ADR 0013 §4); re-deriving the moved index is a
  deferred nicety, not MVP.
- **No new nouns, no new `RejectReason`s.** Sync the **Selection** and **Vertex / Edge**
  `CONTEXT.md` entries (selection now *drives* editing; vertices/edges are draggable) and flip the
  plan-view surface spec's "No footprint editing after close" from deferred to built.

## Definition of done

Plan approved → ADR 0015 written → implement phases 1–7 → `bun run lint` + `bun run typecheck` +
`bun test` + `cargo fmt --all --check` + `cargo clippy --workspace --all-targets -- -D warnings` +
`cargo test --workspace` + `bun run codegen:check` all green → **render-verify the real plan viewport**:
draw a footprint, push it into a mass, then drag a vertex / insert a vertex / delete a vertex and
confirm the **mass reshapes and is preserved, not flattened**, and that undo restores it → sync
`CONTEXT.md` nouns + the plan-view surface spec → commit and push to
`claude/footprint-editing-p2-9-3kbo8l` (off the latest `main`). No PR unless asked.
</content>
</invoke>
