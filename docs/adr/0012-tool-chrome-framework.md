# ADR 0012 — The tool-chrome framework: declarative tools drive the drawing UI

- **Status:** Proposed
- **Date:** 2026-07-01
- **Context doc:** [`docs/analysis/sketchup-tools-analysis.md`](../analysis/sketchup-tools-analysis.md) §2b;
  builds on [ADR 0008](./0008-mvp-geometry-and-command-contract.md) (the command contract) and
  [ADR 0005](./0005-frontend-application-stack-react-vite.md) (React + Vite; the imperative 3D view);
  the product-design surface specs
  ([`surfaces-plan-view.md`](../../.agents/skills/product-design/references/surfaces-plan-view.md),
  [`surfaces-3d-view.md`](../../.agents/skills/product-design/references/surfaces-3d-view.md)) own the
  language.

## Context

The drawing UI is about to grow from two tools to roughly eight. The SketchUp analysis
([§3](../analysis/sketchup-tools-analysis.md)) queues Rectangle, Move/Copy, Select, footprint
editing, Openings, and more on top of today's Footprint + Push/Pull (and the latent, engine-only
Wall).

The **engine/commit side is already declarative.** `packages/tool-runner` is "one runner plus a
flyweight catalog of `ToolDefinition`s, never N tool classes": `TOOL_CATALOG` is a data table, and
adding a tool's *commit grammar* is a catalog row plus a `ToolRunner.commit` case. That half scales.

The **front-end chrome around it is not a framework.** Everything a tool needs to *appear and
behave* in the UI is hand-wired per tool across `apps/web/src/app.tsx`, `plan-view.tsx`, and
`three-view.tsx`:

- the toolbar buttons and their enabled-predicate (Push/Pull is gated on "a mass exists" by an
  ad-hoc check);
- the contextual **status-bar copy** (`if (tool === …)`-style phrasing);
- the **cursor** and **which surface** a tool is active on;
- the **value-entry box** — a plan-only, hard-labeled "Length" input (`plan__measure`) that has no
  meaning for Push/Pull (which has *no* typed height) or a future Rectangle (which wants `W,D`);
- the **canvas overlays / HUD** — rubber band, alignment guides, close target, and the live length
  label (`plan__dim`) are bespoke SVG nodes; the 3D view has **no measurement layer at all**, which
  is why Push/Pull is dragged blind (no distance readout — the concrete gap from
  [§2b.1](../analysis/sketchup-tools-analysis.md)).

Each new tool therefore means editing three view files and growing branch ladders. That is the rot
this ADR prevents.

**A boundary constrains where the fix can live.** `tool-runner` is deliberately pixel-free:
"screen-space/pixels are the app's concern and never appear here." Cursors, keybindings, React,
status prose, and CSS classes must **not** migrate into it. The chrome descriptor needs a home on
the presentation side, linked to the pure catalog by key.

## Decision

1. **Split the tool model in two, along the existing boundary.** `tool-runner`'s `TOOL_CATALOG`
   stays the **pure commit grammar** (picks → `Command`; ticks only). A new **presentation-side tool
   registry** in `apps/web` (e.g. `src/tools/tool-chrome.ts`) holds the **chrome descriptor**, keyed
   by the same catalog key. `tool-runner` stays boundary-pure; the app owns pixels.

2. **The chrome descriptor is the single declaration the UI reads.** One record per tool, carrying:
   - **toolbar** — label, icon, and an `enabled(state)` predicate (generalizing today's Push/Pull
     "needs a mass" gate);
   - **surfaces** — which panes it is active on (`plan`, `3d`, or both);
   - **cursor** and **keybinding** (single-key tool switch, SketchUp-style: `L`, `R`, `P`, `M`…);
   - **status copy per phase** — e.g. Rectangle: "Click first corner" → "Click opposite corner";
   - **value grammar** — what a typed value *means* for this tool and how it commits (length /
     angle / `W,D` / height / array `5x`);
   - **overlay/HUD contributions** — the ephemeral labels and affordances the tool feeds the shared
     HUD (segment length+angle, Push/Pull distance, snap/inference badge, grabbable-cap highlight).

3. **One HUD layer per surface; tools contribute, they don't own text nodes.** Plan renders labels
   as SVG `<text>`; the 3D view gains **one** billboard/label layer (a `CSS2DRenderer` or an HTML
   overlay over the canvas — chosen once, reused by every tool, dimensions, and selection tags — do
   not grow three). The descriptor declares *what* to show; each surface owns *how* to render it.
   This retires the bespoke `plan__dim`-only readout and gives Push/Pull its missing **distance
   readout** as a first consumer, not a special case.

4. **The value box becomes tool-driven and multi-surface.** Replace the plan-only "Length" input
   with a value box bound to the active tool's declared value grammar. Push/Pull gets a **typed
   height**; Rectangle gets typed `W,D`; the live readout remains the box's placeholder (the
   identity that already holds in `plan-view.tsx`). One channel, not per-tool inputs.

5. **The status bar is data, not branches.** The contextual status line reads the active tool's
   phase copy from the descriptor. The `if (tool === …)` prose ladders in `app.tsx` are deleted.

6. **Catalog↔chrome parity is enforced.** A unit test asserts every `TOOL_CATALOG` key has exactly
   one chrome descriptor and vice versa — a tool cannot ship half-wired. This mirrors the repo's
   "cannot drift" discipline (`codegen:check`, `LAYOUT_HASH`) applied to the tool surface.

## Consequences

- **Adding a tool is declarative again on both halves:** one `TOOL_CATALOG` row (commit grammar) +
  one chrome descriptor row (+ an engine `Command` case only if the tool needs a new one). No edits
  scattered across three view files; no new `if (tool === …)` branches.
- **The two concrete gaps close as by-products.** Push/Pull's distance readout falls out of the
  shared HUD (§3); its typed height falls out of the tool-driven value box (§4). The
  [§2b.1](../analysis/sketchup-tools-analysis.md) HUD work is now "implement the layer + declare
  contributions," not per-tool plumbing.
- **`tool-runner` keeps its purity.** The presentation registry absorbs exactly the UI concerns the
  boundary rule says the runner must not hold.
- **One-time migration cost, paid cheap.** Footprint and Push/Pull are re-expressed through the
  framework and Wall's latent toolbar entry is wired up — three tools, done while the surface is
  small. Deferring means ripping out N bespoke wirings later.
- **The imperative 3D view gains a label layer.** It is subject to ADR 0005's dispose discipline
  (dispose on rebuild/unmount; a leak here is a real defect) — the new HUD layer must honor it.
- **Not a schema/MODEL change.** No buffer, no codegen, no boundary-direction change. It *is* a new
  front-end structure, so it takes an ADR per the repo rule (structural changes are ADRs), but it
  touches only `apps/web` (+ leaving `tool-runner` intact).

## Alternatives considered

- **Fold the chrome into `tool-runner`'s `ToolDefinition`.** Rejected: it pollutes the boundary-pure
  package with pixels, cursors, keybindings, React, and copy — precisely what "screen-space never
  appears here" excludes. (`label` already sits at that border; we do not widen it further.)
- **Keep hand-wiring per tool (status quo).** Rejected: with ~6 tools queued, per-tool branches
  across three files is the rot this exists to stop. Building the framework while only two tools need
  migrating is the cheap moment — this is a YAGNI-*aware* call, not a speculative one: the roadmap,
  not a hypothetical, justifies it now.
- **Generate the descriptor from `schema/model/` via codegen.** Deferred (YAGNI): tools are not
  domain MODEL types, and a hand-authored front-end registry with a parity test (§6) gives the
  drift-safety without inventing a schema surface. Revisit only if tools ever become data-driven from
  the domain.
- **A separate ADR for the measurement HUD.** Rejected as fragmentation: the HUD is one of the
  surfaces the descriptor feeds. Splitting it would let the value box and overlays drift from the
  tool model they belong to; they are decided together here.
