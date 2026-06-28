# Web ("hands & eyes")

The browser client. It captures draw gestures, sends them into the engine as commands, mirrors
the canonical buffer the engine ships back, and renders it. It never owns or mutates canonical
geometry — it is eyes and hands, not the brain.

## Language

**App shell**:
The frame that organizes the UI — the toolbar/tool palette, the status area, and the layout that
holds the viewports. React-owned chrome; it holds no canonical geometry.
_Avoid_: layout, frame, chrome (as the proper noun for this).

**Viewport**:
One rendering surface showing the model from a fixed projection. The drawing UX has two: the
**plan view** and the **3D view**. An imperative renderer mounted into a React-managed container.
_Avoid_: pane, canvas, window.

**Plan view**:
The top-down (world XY), orthographic viewport. The **drawing surface**: gestures are picked
here and become commands.
_Avoid_: 2D view, top view, floorplan.

**3D view**:
The perspective, orbitable viewport showing members as solids. A read-only mirror of the same
canonical buffer — never a drawing surface in this slice.
_Avoid_: model view, perspective view, scene.

**Elevation view**:
A viewport looking at a single wall face (`x` along the baseline, `z` up). The Phase 4 slice's
only view; deferred to a later inspector pane in the drawing UX.
_Avoid_: section, side view.

**Drawing surface**:
The one viewport in which a gesture is picked and turned into a command. Currently the plan view,
and only ever one viewport at a time.
_Avoid_: active canvas, input view.

**World space**:
The shared coordinate system every viewport renders, in ticks. Members arrive already placed in
world space by the engine (baseline position, orientation, and through-wall depth composed in).
_Avoid_: global space, scene space, model space.

**Wall-local space**:
A single wall's own frame (`x` along the baseline, `z` up, `y` through-wall depth). The engine's
internal framing space; the wall→world transform lifts it into world space before the buffer ships.
_Avoid_: local space, member space.

**Mirror**:
A read-only, zero-copy view over the engine's canonical SoA buffer (e.g. `MemberMirror`). The
only way the client reads geometry; there is no second, client-side model.
_Avoid_: model, store, cache, snapshot (the snapshot is the *bytes*; the mirror is the *view*).

**Display mesh**:
Presentation geometry the 3D view tessellates from a member's world-space segment and draw width
(boxes from segments). It carries no authority — it is pixels for one frame, not a source of
truth, exactly as the 2D view's stroked line is.
_Avoid_: model geometry, canonical mesh, the model.

**Tool**:
A picking state machine that turns gestures in the drawing surface into a `Command` (e.g. the
wall tool: two picks → a `DrawWall`). The **active tool** is the one currently receiving picks.
_Avoid_: mode, gesture handler.
