# Analysis — SketchUp drawing tools & UI, and a prioritized feature list for Jose

**Purpose.** Decompose what actually makes SketchUp's drawing experience good, hold it against
Jose's current space-first drawing slice, and produce a *prioritized, tiered* list of must-have
features to build. This is an opinionated planning artifact, not a spec — each item points at the
ADRs/plans/files it lands in.

**Framing rule (read first).** Jose is a **constrained** parametric residential-framing BIM tool,
not a general 3D modeler. The goal is **not** to clone SketchUp's ~40-tool surface. It is to steal
the handful of deep ideas that make drawing feel effortless and map them onto *footprints, masses,
and framing* — and to deliberately **skip** the sculpting toolkit (see §4, YAGNI). SketchUp is the
*feel* to reach; the space-first flow ([ADR 0007](../adr/0007-space-first-modeling-footprint-push-pull.md))
is the road there.

---

## 1. What SketchUp actually gets right (the analysis)

SketchUp won on a small number of load-bearing ideas, not breadth. In rough order of how much each
one matters to a drawing tool:

### 1.1 The inference engine — the crown jewel
Continuous, colored, *modeless* snapping that lets you draw precisely without dialogs:
- **Point inference:** endpoint (green), midpoint (cyan), intersection, center, on-edge (red),
  on-face (blue), origin.
- **Linear inference:** on-axis (solid red/green/blue lines), from-point (dotted), parallel,
  perpendicular, tangent.
- **Inference locking:** `Shift` locks the current inferred direction; arrow keys lock to a specific
  axis. This turns a suggestion into a constraint on demand.

The magic is that it is *always on, always visual, never modal.* You never open a "snap settings"
dialog — the model tells you what it found and you accept or override it.

### 1.2 The Measurements box (VCB — Value Control Box) — type-exact precision
A single always-listening field (bottom-right). You start a gesture, then just *type*: `10'` sets a
length, `10',8'` a rectangle's sides, `24` a radius, `45` a rotate angle, `5x` a linear array count,
`/3` subdivides. No focus click, no modal. You can type a value *after* committing to adjust it.
This is what makes SketchUp both fluid and CAD-precise at once.

### 1.3 Direct manipulation + modifier keys — one tool, many verbs
Click-move-click (rubber-band), not click-drag — less fatigue, fewer misfires. Modifiers multiply
each tool: `Move`+`Ctrl` = copy; then `5x` = linear array. `Rotate`+`Ctrl` = radial array.
`Push/Pull`+`Ctrl` = leave the source face. Fewer tools, each deep.

### 1.4 Push/Pull — the signature verb
Pick a face, push/pull it perpendicular into 3D; double-click repeats the last distance. It is the
single gesture that made "2D sketch → 3D mass" feel like play.

### 1.5 Auto-face generation & healing
Close a coplanar loop of edges → a face appears. Split a face with an edge → two faces. Delete a
shared edge → faces heal. Geometry is *live topology*, not a static drawing. (This is exactly the
capability ADR 0007 flags as needing a general BREP modeler — deferred.)

### 1.6 Groups & Components — instancing (the BIM-critical one)
Components are definition-based, instanced, reusable; edit one, all update. They carry insertion
axes, can glue to faces, and can **cut openings**. This is *the* feature that makes SketchUp usable
for repeated elements — and studs, joists, rafters, windows, and wall types **are** repeated
elements. For a framing tool this is where "drawing app" becomes "building model."

### 1.7 Always-available navigation
Orbit (middle-drag), Pan (`Shift`+middle), Zoom (scroll), **Zoom Extents**. Crucially, navigation
never interrupts the active tool — you can orbit mid-draw. Losing your geometry off-screen is
impossible.

### 1.8 Selection + editing model
Click / double-click (face+edges, or enter a group) / triple-click (all connected); window-select
(left→right encloses, right→left crosses); `Shift` toggles, `Ctrl` adds. Selection is the
precondition for *editing existing geometry* — SketchUp is nothing if what you drew is immutable.

### 1.9 Modeless UI chrome
A big viewport, thin toolbars, a **contextual status bar** ("Select first corner"), the VCB, the
Instructor panel (animated per-tool help), Entity Info (properties of the selection), the Outliner
(hierarchy), Tags (visibility). Dialogs are rare; the status bar + VCB replace them. Tools are
**sticky** (stay active for repeated use). And **undo/redo** (`Ctrl+Z`) is bedrock under all of it.

---

## 2. Where Jose stands today (built / partial / missing)

Grounded in the current code (`packages/tool-runner`, `apps/web/src/plan-view.tsx`,
`three-view.tsx`, `crates/bim-core`) and the product-design surface specs.

| SketchUp pillar | Jose today | Evidence |
|---|---|---|
| **1.1 Inference** | **Partial — surprisingly good.** Grid snap, `Shift` axis-lock, alignment-guide inference to existing vertices' rows/cols, ring-close snap all exist in `tool-runner`. Missing: midpoint/edge/intersection snap, parallel/perpendicular, colored point cues, arrow-key axis lock. | `tool-runner/src/index.ts` (`inferAlignment`, axis lock, `gridTicks`) |
| **1.2 VCB / typed dimension** | **Partial.** Plan view has a SketchUp-style length box (`parseLength`, feet/inches) **and** a live length label on the rubber-band segment (`plan__dim`). Missing: typed/live **angle**, persistent **length labels on committed edges**, typed **rectangle W,D**, and any **height readout or typed height** in 3D push/pull (gesture-only, no on-screen distance). | `plan-view.tsx` (`plan__dim`, value box); `three-view.tsx` (no readout at all) |
| **1.3 Modifiers / array** | **Missing.** No Move/Copy, no array, no modifier verbs beyond `Shift` axis-lock. | — |
| **1.4 Push/Pull** | **Built (top-cap only).** Raycast top cap → drag → `PushPull`. Any-face deferred by ADR 0007 §3. | `three-view.tsx`, `command.rs` |
| **1.5 Faces / topology** | **Built for the one case.** Close ring → footprint face → extrude to mass. General carving needs BREP (deferred). | `session.rs`, ADR 0007 |
| **1.6 Components / assemblies** | **Missing (domain-latent).** 13 `FramingRole`s and a `MemberPlacement` buffer exist but nothing user-facing; no wall types, no openings, no reusable units. | `render-mirror`, `building::FramingRole` |
| **1.7 Navigation** | **3D: yes** (OrbitControls). **Plan: yes** — scroll-zoom (to cursor), middle-drag pan, and Fit / Shift+Z Zoom-Extents landed (P0 #2), via a stateful `PlanCamera`. Zoom-Extents in 3D still absent. | `plan-camera.ts`, `plan-view.tsx` |
| **1.8 Selection / editing** | **Selection: yes** (P0 #3) — a Select tool picks a vertex/edge/footprint with a hover affordance and Esc-to-clear, via a pure screen-space `hitTest`. Editing (vertex drag/insert/delete, post-close footprint edit) still missing — it builds on this. | `plan-selection.ts`, ADR 0013 |
| **1.9 Chrome / undo** | **Partial.** Toolbar + status bar exist. **No undo/redo. No error/rejected-command state. No hover cues.** | `coverage-gaps.md` |

**Headline:** the *drawing feel* is further along than expected (inference + typed length are
partly done). The real holes are **editing, navigation, undo, selection, and the framing-domain
layer** — plus the resilience states.

---

## 2b. Two cross-cutting concerns (the UI must carry the new tools)

These are not one more row in the tier list — they are the substrate every future tool rides on. If
they aren't built as a *framework*, each added tool re-wires the chrome by hand and the UI rots.

### 2b.1 Live measurement feedback — the in-canvas HUD

The single most important thing a drawing UI does is **tell you what you're about to make, before
you commit it.** You place a point; you must see not just the line but its **length and angle**;
you push/pull and you must see the **live distance**. Today this is uneven:

- **Plan view:** the rubber-band segment *does* show a live length (`plan__dim`). But there's **no
  angle**, **no length on committed footprint edges**, and no running perimeter/area.
- **3D view:** push/pull has **no distance readout whatsoever** — you drag the cap blind. This is the
  concrete gap: you should see the height accumulating (e.g. `8' 1"`) pinned to the cursor/cap as you
  drag, and be able to type it.

Treat the HUD as **one shared concern**, not per-tool text nodes:
- **Ephemeral overlays** anchored to the gesture: length + angle on the active segment; distance on
  the active push/pull; a snap/inference badge ("Endpoint", "On edge", "Parallel") when the inference
  engine fires.
- **Persistent labels** on committed geometry: edge lengths on the footprint (toggleable), overall
  width×depth, height on the mass.
- **One place to render it.** Plan is SVG (`<text>`), 3D needs an overlay layer (CSS/`CSS2DRenderer`
  billboards, or an HTML layer over the canvas). Pick one 3D-label mechanism now so push/pull,
  dimensions, and selection tags all reuse it — don't grow three.
- **The typed value and the live readout are the same channel.** The live length *is* the VCB
  placeholder already (`plan-view.tsx`); keep that identity as it extends to angle and to 3D height.

### 2b.2 A tool-chrome framework — adding a tool should be declarative

> Drafted as [ADR 0012](../adr/0012-tool-chrome-framework.md).


`tool-runner` already has the right backbone: `TOOL_CATALOG` is a data-driven registry, and adding a
tool is "a row + a `commit()` case" engine-side. The **front-end chrome around it is not yet a
framework** — the toolbar, status copy, cursor, value box, and canvas overlays are hand-wired for
footprint and push/pull specifically. Before piling on Rectangle, Move, Openings, etc., make the UI
consume a tool's declaration so a new tool lights up the whole chrome for free. A tool should
declare, in one place:

- **Toolbar presence:** label, icon, enabled-predicate (push/pull is already gated on "a mass
  exists" — generalize that).
- **Status-bar copy** per phase ("Click first corner" → "Click opposite corner"), so the contextual
  status line is data, not `if (tool === …)` ladders.
- **Cursor** and **which surface(s)** it's active on (plan, 3D, or both).
- **VCB semantics:** what a typed value *means* for this tool (length? angle? W,D? height? array
  `5x`?) and how it commits.
- **Canvas overlays / affordances** it contributes (rubber-band, guides, the grabbable-cap
  highlight, HUD labels from 2b.1).
- **Keybinding** (single-key tool switch, SketchUp-style: `L`, `R`, `P`, `M`…).

This is the concrete meaning of "the UI properly supports new tool additions": the registry that
exists engine-side should have a front-end twin, and the HUD (2b.1), status bar, and toolbar should
all read from it. Build it *now*, while there are only two tools to migrate, not after there are
eight.

---

## 3. Prioritized must-have feature list

Tiers are ordered by "does the tool feel broken/frustrating without it" → "does it deliver the
SketchUp magic" → "does it unlock the reason this is a *framing* tool." Each item names where it
lands.

### P0 — Table stakes (a modeling tool is not credible without these)

> **Foundational substrate (build first, see §2b):** the **live measurement HUD** — especially a
> **push/pull distance readout in 3D** (the one you called out: today you drag the cap blind) plus
> **angle** on the plan segment and **length labels on committed edges** — and the **tool-chrome
> framework** so the toolbar/status/VCB/overlays are declarative per tool. These two underpin every
> feature below; ship them before adding tools, not after.

1. **Undo / redo.** Nothing reverses a draw or a push/pull today. This is the single most glaring
   gap and it *gates* everything destructive (vertex delete, clear, opening-cut all assume undo
   exists). Needs a command history in `Session` (the engine already funnels every mutation through
   `Session::apply` — an ordered command log + replay/inverse is the natural seam). `Ctrl+Z`/`Ctrl+Shift+Z`.
2. ~~**Plan pan / zoom + Zoom-Extents.**~~ ✅ **Landed.** The plan's world↔screen transform is now a
   stateful `PlanCamera` (`plan-camera.ts`, pure + unit-tested): scroll zooms toward the cursor,
   middle-drag pans, and a **Fit** button / **Shift+Z** run Zoom-Extents (framing the footprint +
   in-progress picks, with a degenerate-geometry fallback). Off-view geometry is reachable again.
3. ~~**Selection model.**~~ ✅ **Landed.** A **Select** tool (ADR 0012's third) picks a footprint /
   vertex / edge in the plan via a pure screen-space `hitTest` (priority vertex → edge → face), with a
   hover affordance and `Esc` / empty-click to clear. Selection is presentation state in the store
   (never the engine), keyed by ring index and cleared on recompute
   ([ADR 0013](../adr/0013-selection-model.md)). The precondition for P2 editing; extends to 3D next.
4. **Rejected-command & degenerate-geometry feedback.** A self-intersecting/zero-area footprint, an
   engine rejection, or a `LAYOUT_HASH` mismatch currently fails silently. Surface it (status bar +
   inline cue), preserve the user's in-progress input, and say what to fix. (`resilience.md`,
   `coverage-gaps.md`.)

### P1 — The SketchUp magic (the reason to emulate SketchUp at all)

5. **Inference engine v1.5 — finish the snapping.** Build on the existing `inferAlignment`/axis-lock
   base: add **midpoint & on-edge** snap to the footprint, **parallel/perpendicular** to existing
   edges, **colored point cues** (endpoint/midpoint/edge), and **arrow-key axis lock** (in addition
   to `Shift`). Modeless and visual — never a settings dialog.
6. **Finish the measurement HUD (extends the P0 substrate, §2b.1).** Angle on the plan segment,
   persistent edge-length labels on the committed footprint, snap/inference badges, and running
   width×depth. (The bare push/pull distance readout is the P0 slice; this is the rest.)
7. **Typed dimension everywhere (VCB parity).** Extend the value box to **angle** and to a **rectangle
   W,D** (see #8), and a **typed height** for 3D push/pull so height isn't gesture-only — all routed
   through the one tool-declared VCB channel (§2b.2), not per-tool inputs.

### P2 — Editing (immutable geometry is a dead end)

8. **Rectangle tool.** Most residential footprints are rectangular; a 2-click rectangle with typed
   `W,D` is dramatically faster than the polyline for the 80% case. A new `ToolDefinition` row +
   `Command` variant that emits the same closed `DrawFootprint` ring.
9. **Footprint editing.** Drag a vertex; insert/delete a vertex; move an edge. Requires selection
   (#3) and a new edit `Command` (the engine stores the ring in `Session::footprint` — mutate and
   re-extrude). This is the difference between "sketch once" and "model."
10. **Move / Copy (+ linear array).** SketchUp's core edit verb: move a selection; `Ctrl` copies;
    typed `5x` arrays. Highest-leverage single edit tool once selection exists.

### P3 — Framing-domain unlocks (where Jose stops being SketchUp and becomes BIM)

11. **Wall-type / assembly selection.** The footprint offsets **outward** into a framed wall
    assembly (ADR 0007 §4). Choosing 2×4 @ 16" o.c. vs 2×6 @ 24" is the product's whole reason to
    exist. The `FramingSolver` + `MemberPlacement` buffer already exist engine-side — wire selection
    UI → framing → render the members.
12. **Openings (doors / windows).** SketchUp's "glued component that cuts a hole," specialized:
    place an opening on a wall, size it, let the solver emit king/jack/cripple/header/sill (roles
    already in the vocab). The natural next gesture after push/pull.
13. **Elevation / section view.** Framing is a vertical story; plan + 3D can't show stud layout.
    The `MemberPlacement` buffer is built and unrendered — an elevation surface is where it lights
    up. (Deferred in the MVP plan; this is the first post-MVP surface.)
14. **Multi-story / levels.** Stack footprints into stories; carry framing per level.

### P4 — Later / polish

15. **Components / reusable assemblies.** Save a wall type, a window unit, a stair — instanced and
    editable-once. (Pillar 1.6 fully realized.)
16. **Permanent dimensions & annotations.** Dimension *entities* and text, not just live readouts.
17. **Tags / visibility.** Show/hide massing vs. framing vs. annotations.
18. **General any-face push/pull.** L/T footprint carving, recesses, openings-as-topology — needs
    the kernel to grow from a prism model into a general **BREP solid modeler** (its own ADR, per
    ADR 0007 §3).
19. **Responsive/stacked breakpoint, touch ergonomics, design tokens.** The `1fr 1fr` grid crushes
    on narrow windows; there is no token layer yet (`coverage-gaps.md`).

---

## 4. Deliberately SKIP (YAGNI — do not build)

A constrained framing tool does not need most of SketchUp's surface. Skipping these is a feature,
not a shortfall — it keeps the tool legible and the codebase honest (repo YAGNI ethos):

- **Follow Me, Freehand, Arc, Circle, Polygon(n-gon), rotated-rectangle** — curved/organic geometry
  has no place in orthogonal residential framing. (Straight edges + rectangles cover it.)
- **Scale tool, Position Camera, Walk, Look Around** — presentation/architectural-visualization
  tools, not modeling-for-framing.
- **Paint Bucket / materials / textures, 3D Text, Sandbox/terrain** — out of domain.
- **Full component browser / 3D Warehouse** — a curated *assembly* library (P4 #15) is the useful
  subset; a general model marketplace is not.

If any of these is later justified, it needs an ADR — not a toolbar button by reflex.

---

## 5. Quick wins vs. big rocks (sequencing note)

- **Quick wins (days, high felt-value):** the **push/pull distance readout** (§2b.1 — the gap you
  flagged, and the highest felt-value single change), Zoom-Extents + plan pan/zoom (#2), plan-segment
  angle (#6), typed height in 3D (#7), rejected-geometry status message (#4). These sharpen the
  *existing* slice without new architecture.
- **Foundational (unlock everything after):** the **tool-chrome framework** (§2b.2), **undo/redo**
  (#1), and **selection** (#3). Build the chrome framework while only footprint + push/pull exist to
  migrate; build undo/selection before P2 editing — every edit and destructive action depends on them.
- **Big rocks (own plan/ADR each):** Wall-type framing (#11), openings (#12), elevation view (#13),
  and — much later — the BREP modeler behind general push/pull (#18).

**Recommended first cut:** P0 in order (undo → pan/zoom → selection → error states), then P1 #6/#7
as fast follows, then open a plan for P3 #11 (wall-type framing) since that is the product's
differentiator and the engine scaffolding for it already exists.
