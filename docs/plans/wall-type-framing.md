# Plan — Wall-type / assembly framing (P3 #11)

Where Jose stops being SketchUp-lite and becomes a framing BIM tool: choose a **wall type**
(stud size + on-center spacing) for the space, and the engine derives and renders the actual
framing — studs, plates, corner posts — from the drawn footprint and its mass. This is the P3 #11
item from [`docs/analysis/sketchup-tools-analysis.md`](../analysis/sketchup-tools-analysis.md) §3;
the load-bearing calls (an always-derived framing stage, the world-space member render contract,
and where the assembly choice lives) are recorded in a new **ADR 0016**, drafted on approval of
this plan.

## What we're building (and not)

**In:**
- A **wall type** for the current space — one of four enumerated assemblies (2×4 / 2×6 stud ×
  16″ / 24″ o.c.), default **2×4 @ 16″** — chosen from a compact, always-visible control in the
  toolbar and carried by a new `SetWallAssembly` engine command.
- A **derive-framing stage** in the session pipeline: footprint ring + mass height → one `Wall`
  per edge → the existing `frame_walls` (junctions detected and detailed per ADR 0009) → the
  `MemberPlacement` SoA buffer. Framing is derived **automatically whenever a mass exists** —
  there is no "generate" button; the model is always consistent (recomputed on draw, edit,
  push/pull, undo/redo, and assembly change).
- **Outward framing** (ADR 0007 §4): the footprint is the interior face; each wall's body grows
  outward by the assembly thickness, corners detailed by the existing detector/detailer.
- **Member rendering in the 3D view**: studs/plates/posts as solid boxes colored by role, read
  from a `MemberMirror` over the engine's bytes (one-direction rule intact). Once framing exists
  it **replaces the translucent mass walls**; the pickable top cap stays, so push/pull keeps
  working.

**Out (deferred, do not absorb):** openings / doors / windows (P3 #12 — `Opening` framing already
works engine-side; no UI for it here); the elevation/section view (P3 #13); multi-story (#14);
floor/roof/sheathing assemblies (the `AssemblyKind` stubs stay stubs); per-junction corner-method
overrides or a corner-rules UI (ADR 0009 §4's defaults table stays engine-internal); framing in
the **plan view** (stays pure geometry per `CONTEXT.md`); member picking/selection in 3D.

## The load-bearing decisions (ADR 0016)

### 1. Framing is an always-derived pipeline stage, parameterized by a session-held assembly

The engine scaffolding exists but the space-first flow never calls it. The missing piece is a
composition stage in `session.rs`:

```rust
fn rewrite_space_buffers(&mut self) {
    // …footprint + volume rows as today…
    self.buffer.clear();
    if let Some(volume) = self.volume.as_ref() {
        let walls = walls_from_footprint(&self.footprint, volume.height, &self.assembly);
        for member in frame_walls_placed(&mut self.framer, &walls, &self.assembly.rules()) {
            self.buffer.push(member_row(&member));
        }
    }
}
```

- `walls_from_footprint` normalizes the ring to CCW (derived copy — the canonical footprint is
  never mutated), then promotes **one `Wall` per edge**: baseline = the edge (the interior face,
  per outward framing), `WallId` = edge index + 1 (deterministic → stable framing across
  recomputes, ADR 0009 §3), height = the mass height, thickness + spacing from the assembly,
  `WallRole::Exterior`, `interior_on_left = true` (the CCW convention `Wall::promote` documents).
- **No mass ⇒ no framing.** A flat footprint has no wall height to frame at; the member buffer is
  empty until the first push/pull, and empties again if a redraw flattens the space. This is the
  "when does it recompute" answer: wholesale, on every accepted command, exactly when a volume
  exists (ADR 0009 §6 — O(walls + junctions) integer math, cheap).
- The **assembly is session state, in the undo history.** `Session` gains
  `assembly: WallAssembly` (default 2×4 @ 16″), snapshotted alongside footprint + volume, so undo
  restores the wall type a mass was framed with. `SetWallAssembly` records history and reframes;
  setting the identical assembly is an accepted no-op (no history entry).
- The domain type lives in `building` (it is the product's core vocabulary, not composition
  glue): `WallAssembly { stud: StudSize, spacing: SpacingModule }` with
  `StudSize::{TwoByFour, TwoBySix}` → `depth_ticks()` (112 / 176) and the existing
  `RuleSet::light_frame_wall` specs. `AssemblyKind` (the rule-pack key) is unchanged.

### 2. The member render contract becomes a world-space box recipe (a MODEL change)

Today's `MemberPlacement` rows are **wall-local** ("composing the wall→world transform is a later
pipeline stage" — that stage is now). And a segment + `width` cannot orient a 3D box: a stud's
cross-section needs the through-wall dimension and which way the wall faces (footprints are not
axis-aligned in general).

**Decision: each row is a world-space box recipe** — centerline segment (`x0..z1`, now world
ticks), `width` (cross-section dimension in the wall plane, unchanged column), plus **three new
columns** in `schema/model/buffer-layouts.json` (+ `bun run codegen`, never hand-edits):

| column | type | meaning |
| --- | --- | --- |
| `depth` | i32 | through-wall cross-section dimension, ticks (3.5″/5.5″ for field members; the post's other plan dimension) |
| `nx`, `ny` | i32 | the wall's outward normal direction in plan (unnormalized; the reader normalizes) |

One uniform recipe reconstructs every role: box axes = segment direction `d̂`, normal `n̂`, and
`d̂ × n̂`; extents = length × depth × width. A vertical stud gets its 1.5″ face along the wall and
its 3.5″/5.5″ depth through it; a flat plate gets 1.5″ vertically and the wall depth through it —
no per-role cases in the reader. A `wallId` column was considered and deferred (YAGNI — rendering
doesn't need it; it returns with elevation-view picking, P3 #13).

**The world composition lives in `building`** (the context is literally "Building Model +
Placement"; ADR 0006 says placement is engine-side): a new facade entry
`frame_walls_placed(solver, walls, rules) -> Vec<PlacedMember>` wraps the existing `frame_walls`
per-wall loop and resolves each member to world — field members rotated/translated by their
wall's baseline and offset **outward** so the cross-section spans baseline → baseline +
thickness·n̂ (the footprint stays the interior face); corner posts (already world plan AABBs from
the detailer) pass through with their real plan size, which today is lost in `member_row`.
`FramingSolver::frame` keeps its wall-local contract (its stability tests stay meaningful);
`bim-core`'s `member_row` becomes a 1:1 flattening of `PlacedMember`. The legacy `draw_wall` path
routes through the same placement so the row shape has one writer.

Also in `building`: **generalize the junction detailer's hardcoded 2×4 constants**
(`STUD_WIDE = 112` in `junction_detail.rs`) to the participating walls' thickness, so a 2×6
corner details at 5.5″. (`STUD_NARROW`/`PLATE_THICKNESS` = 1.5″ hold for both sizes.)

### 3. The wall type is a toolbar control, not a tool and not an inspector

The MVP has no inspector and shouldn't grow one for an enumerated choice. A "Framing" *tool* was
considered and rejected: a tool is a picking state machine, and choosing an assembly involves no
viewport gesture. Value-box grammar was rejected too: the VCB is per-tool typed *measurement*
entry, and burying the wall type there makes the current choice invisible.

**Decision: a native `<select>` in the toolbar** (a third group beside the tools and History,
`aria-label="Wall type"`), listing the four assemblies (`2×4 @ 16″` …), always enabled once the
engine is ready. Choosing dispatches `SetWallAssembly`; the select's *value renders from engine
state* — the `space` snapshot now reports the session's current assembly — so undo/redo keeps the
control honest and the client never owns the choice (one-direction rule). Modeless, visible,
keyboard-accessible, zero new framework. The active wall type is thereby always legible, which is
the point of a framing tool.

### 4. Framing replaces the mass solid; the top cap stays

Outward framing means members wrap the *outside* of the mass — the stud's interior face is
coplanar with the mass's side wall, so overlaying both z-fights. A visibility toggle is P4 #17
and new chrome we don't need yet.

**Decision:** once members exist, the 3D view renders **framing + the translucent top cap** and
drops the translucent side walls — the framing *is* the walls now, and the cap remains the named,
pickable push/pull face (its behavior, camera-preservation rules, and the drag's live preview are
untouched; the preview keeps rendering the simple prism during the drag and snaps to framed truth
on command return). Members render as one disposable `Group` of box meshes colored by a small
role→color map (studs/kings/jacks/cripples in wood tones, plates/sills distinct, posts and
headers highlighted). ~150 meshes for a large room is trivial; `InstancedMesh` per role is the
noted scaling path, not built now.

## Reachable states & copy

- **Flat footprint (no mass):** unchanged — flat face in 3D, no framing, member buffer empty.
- **Mass exists:** framed walls at the current wall type + translucent cap. The footprint status
  line gains the wall type: *"Footprint: 4 vertices · mass 8.0ft tall · 2×4 @ 16″ framing"*.
- **Wall type changed:** immediate reframe (stud grid re-spaces, walls thicken outward); undoable.
- **Push/pull drag:** prism preview while dragging (transient), framed on release.
- **Undo/redo:** restores footprint + mass + wall type together; select and status follow.
- **Redraw:** new footprint starts flat → framing clears (falls out of "no mass ⇒ no framing").
- Feet/inches everywhere user-facing (`2×4 @ 16″`); ticks never surface.

## Phases (each keeps `main` green)

1. **MODEL: the box-recipe columns.** Add `depth`, `nx`, `ny` to `MemberPlacement` in
   `schema/model/buffer-layouts.json` (and update its note: coordinates are world-space) →
   `bun run codegen`. Extend `MemberRow`/`MemberBuffer` writes (Rust) and `MemberMirror` reads
   (TS); fix the layout-lock tests. `LAYOUT_HASH` changes — the drift guard proves both sides
   moved together.
2. **`building`: assembly + world placement.** `WallAssembly`/`StudSize`; parameterize
   `junction_detail` post sizes by wall thickness; `PlacedMember` + `frame_walls_placed` (facade
   exports). Tests: a CCW rectangle frames all four sides world-placed with every cross-section
   **outside** the ring; a 2×6 assembly thickens walls and corner posts; spacing 24″ re-spaces
   the grid; a CW-drawn ring frames identically to its CCW twin.
3. **`bim-core`: the derive stage + command.** `walls_from_footprint`; `assembly` in
   `Session`/`SpaceSnapshot`; `rewrite_space_buffers` frames when a volume exists;
   `Command::SetWallAssembly`; `draw_wall` rerouted through the shared placement. Tests: framing
   appears on first push/pull and clears on redraw; edit reshapes framing; assembly change
   reframes and undo restores the prior wall type + members; member count is stable across
   identical recomputes.
4. **Boundary + protocol + store.** wasm `setWallAssembly(stud, spacingInches) -> String`
   (rejects an unknown stud size with a stable code); the worker's `space` response gains
   `memberCount`/`memberBuffer` (transferred) + the current assembly; store exposes
   `members: MemberMirror | null`, `wallAssembly`, and `setWallAssembly()`.
5. **3D view: member rendering.** The role→color map; build/dispose the members `Group` in
   `rebuildMass` from the box recipes; drop the translucent side walls when members exist; keep
   the cap, camera, and drag preview exactly as specified in the 3D-view surface spec.
6. **App shell + copy + docs.** The Wall type select; the status-line addition; sync
   `apps/web/CONTEXT.md` (new **Wall type** and **Framing / member** entries; update **Mass**,
   **3D view**, **Outward framing** — no longer deferred), the 3D-view + app-shell surface specs,
   and the copy reference.

## Smaller engineering calls (mine to make)

- **Four assemblies, one axis of choice** — the 2×2 stud × spacing matrix, default 2×4 @ 16″.
  19.2″ o.c. and custom spacing are deferred until asked for (the engine already accepts any
  module).
- **`WallRole::Exterior`** for derived perimeter walls (they are the exterior envelope; `Bearing`
  stays the legacy `draw_wall` value).
- **Stable wall ids from edge order** so the owner-wall rule and stud grid don't reshuffle on
  recompute; spacing stays anchored at each wall's start (the solver's existing stability
  guarantee).
- **Corner-method defaults stay ADR 0009's** (`outside → California`, `inside → ThreeStud`,
  `tee → TwoStudClip`) — no UI, no override surface.
- **Degenerate-ring guards are already upstream**: `validate_ring` rejects before framing ever
  runs; collinear ring vertices yield an inline splice the detector ignores (plates butt, no
  corner) — acceptable, tested, not special-cased.
- **No new `RejectReason` on the ring path**; the boundary's unknown-assembly code
  (`unknown_assembly`) gets a `rejection.ts` line, though the select makes it unreachable from
  the UI.
- **Member buffer capacity (4096) is ample** — ~5,000 ft of wall at 16″ o.c.; the existing
  push-returns-false overflow behavior stands.

## Definition of done

Plan approved → ADR 0016 written → implement phases 1–6 → `bun run lint` + `bun run typecheck` +
`bun test` + `cargo fmt --all --check` + `cargo clippy --workspace --all-targets -- -D warnings` +
`cargo test --workspace` + `bun run codegen:check` all green → **render-verify the real 3D
viewport**: draw a footprint, push/pull it into a mass, and confirm studs + plates + corner posts
render at the mass height, offset **outward** (interior clear dimensions preserved), corners
detailed; switch to 2×6 @ 24″ and confirm the walls thicken and the stud grid re-spaces; undo and
confirm the prior wall type returns; verify push/pull still works over the framed model → sync
`CONTEXT.md` + surface specs → commit and push to `claude/wall-type-framing-p3-11-1ildjy` (off
the latest `main`). No PR unless asked.
