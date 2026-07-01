# Web ("hands & eyes")

The browser client. It captures modeling gestures, sends them into the engine as commands,
mirrors the canonical geometry the engine ships back, and renders it. It never owns or mutates
canonical geometry — it is eyes and hands, not the brain.

The MVP modeling loop is **space-first** (see [ADR 0007](../../docs/adr/0007-space-first-modeling-footprint-push-pull.md)):
draw a footprint in plan, push/pull it into a mass in 3D. Framing is a deferred derived layer.

## Language

**App shell**:
The frame that organizes the UI — the toolbar/tool palette, the status area, and the layout that
holds the viewports. React-owned chrome; it holds no canonical geometry.
_Avoid_: layout, frame, chrome (as the proper noun for this).

**Viewport**:
One rendering surface showing the model from a fixed projection. The drawing UX has two — the
**plan view** and the **3D view** — and in this MVP *both* are input surfaces (each for a
different operation). An imperative renderer mounted into a React-managed container.
_Avoid_: pane, canvas, window.

**Plan view**:
The top-down (world XY), orthographic, 2D CAD viewport. The surface for drawing and editing the
**footprint** — pure geometry; no framing or studs are shown here.
_Avoid_: 2D view, top view, floorplan.

**3D view**:
The perspective, orbitable viewport showing the **mass**. Interactive for **push/pull** (drag the
top cap to set height); otherwise a read-only mirror of the canonical volume. Shows massing solids,
not framing, in the MVP.
_Avoid_: model view, perspective view, scene.

**Elevation view**:
A viewport looking at a single face (e.g. a wall, `x` along the baseline, `z` up). The Phase 4
slice's only view; deferred in the drawing UX.
_Avoid_: section, side view.

**Footprint**:
The closed 2D profile drawn in plan (a kernel `Path2D`) that bounds a **space** — drawn as a polyline
(the footprint tool) or as a box (the **rectangle tool**). It is the **interior face** of any framing
later derived from it — framing offsets outward from it, never inward (see _outward framing_).
_Avoid_: outline, polygon, sketch, perimeter.

**Vertex** / **Edge**:
The corner points of a **footprint** ring and the straight segments between them. They are the
selectable sub-parts of the footprint in the **plan view**; `edge` _i_ runs from vertex _i_ to
vertex _i_+1 (the last edge closes the ring back to vertex 0).
_Avoid_: point, node, handle (vertex); side, line, segment (edge).

**Push/pull**:
The 3D-view gesture that extrudes a footprint's top cap to set the mass's height — the coupling
between the plan (2D footprint) and the 3D (solid). In the MVP it is **vertical, top-cap only**;
general any-face push/pull is a later kernel phase.
_Avoid_: extrude (the kernel verb), drag, pull-up.

**Mass** (massing volume):
The 3D solid produced by extruding a footprint — a kernel `Volume`. What the 3D view renders in
the MVP, before any framing exists.
_Avoid_: block, box, model, solid (bare).

**Space**:
The enclosed region the user draws (its footprint + its mass). The canonical *input* of the
space-first flow, from which walls and framing are later derived.
_Avoid_: room, zone, area.

**Outward framing** (interior face):
The rule that derived framing grows *outward* from the drawn footprint: the footprint is the
interior face, so interior clear dimensions are preserved and the exterior footprint grows by the
assembly thickness. (Framing itself is deferred; this records the rule.)
_Avoid_: wall offset, inset framing.

**World space**:
The shared coordinate system every viewport renders, in ticks (1/32in). Geometry arrives already
placed in world space by the engine.
_Avoid_: global space, scene space, model space.

**Mirror**:
A read-only, zero-copy view over the engine's canonical SoA buffer (e.g. `MemberMirror`). The only
way the client reads geometry; there is no second, client-side model.
_Avoid_: model, store, cache, snapshot (the snapshot is the *bytes*; the mirror is the *view*).

**Tool**:
A picking state machine that turns gestures in a viewport into a `Command` (the footprint tool and
the **rectangle tool** in plan; the push/pull tool in 3D) — or, for the **select tool**, into a
**selection** rather than a command. The **active tool** is the one currently receiving picks.
_Avoid_: mode, gesture handler.

**Rectangle tool**:
The plan tool that draws a **footprint** from two opposite corners (a fast, axis-aligned path for the
rectangular common case) — the same closed ring a polyline footprint produces. Its value box takes a
**size** (`W,D`, e.g. `24', 16'`).
_Avoid_: box, rect, room.

**Selection**:
The user's current pick in the **plan view** — a **footprint**, one of its **vertices**, or one of
its **edges**. Presentation state only: it lives in the client store, is keyed by position in the
canonical ring, and is cleared whenever the engine recomputes. It *names* geometry; it never owns or
mutates it (the one-direction rule, [ADR 0013](../../docs/adr/0013-selection-model.md)). The
precondition for footprint editing, which is deferred.
_Avoid_: highlight, focus, active element, active object.

**Snap** (inference):
While drawing, the cursor **snaps** to existing geometry — an **endpoint**, an edge **midpoint**, or a
point **on an edge** — shown by a colored marker + a badge, so a pick lands exactly. Screen-space,
presentation-only, resolved in `plan-snap.ts` ([ADR 0014](../../docs/adr/0014-plan-inference-and-snapping-model.md)).
_Avoid_: magnet, grid-lock, gravity.
