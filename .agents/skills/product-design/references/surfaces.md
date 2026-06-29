# Surfaces

The drawing UX has three surfaces. This file routes to the per-surface reference and gives the
shared state inventory. Load the specific `surfaces-*.md` for the surface you're touching.

| Surface | What it is | Owner of its language | Reference |
| ------- | ---------- | --------------------- | --------- |
| **App shell** | The toolbar, the status bar, and the layout holding the viewports. React-owned chrome; holds no geometry. | [`apps/web/CONTEXT.md`](../../../apps/web/CONTEXT.md) "App shell" | [`surfaces-app-shell.md`](./surfaces-app-shell.md) |
| **Plan view** | The top-down, orthographic 2D surface for drawing/editing the footprint. | `CONTEXT.md` "Plan view" | [`surfaces-plan-view.md`](./surfaces-plan-view.md) |
| **3D view** | The perspective, orbitable viewport showing the mass; interactive for push/pull. | `CONTEXT.md` "3D view" | [`surfaces-3d-view.md`](./surfaces-3d-view.md) |

Deferred surfaces (named in `CONTEXT.md`, not built — see `coverage-gaps.md`): the **elevation
view**, and any **inspector / property panel**.

## Shared reachable-state inventory

The states the *model* can enter, end to end. Every surface change should account for the states it
can be in. (Source: `app.tsx` `statusText`, `engine-store.ts`, the mirrors.)

| State | Trigger | App shell (status bar) | Plan view | 3D view |
| ----- | ------- | ---------------------- | --------- | ------- |
| **Engine loading** | worker not yet `ready` | "Loading engine…" | inert | inert |
| **Ready / empty** | ready, no picks, no footprint | "Ready — Footprint tool active; click to place vertices" | empty grid, crosshair | empty grid |
| **Footprint in progress** | ≥1 transient pick, ring open | "Drawing footprint — N point(s); click the first to close" | dashed polyline + vertex dots | unchanged |
| **Footprint closed** | ring closed (≥3 verts) | "Footprint: N vertices" | solid footprint polygon | mass appears, camera frames it |
| **Mass present** | volume exists | "Footprint: N vertices · mass Hft tall" | unchanged | translucent mass + named top cap |
| **Push/Pull active** | Push/Pull tool selected (needs a mass) | "Push/Pull active — drag the top cap in 3D to set the mass height" | unchanged | top cap grabbable; orbit otherwise |
| **Push/Pull disabled** | no mass yet | tool button disabled | — | — |
| **Error / rejected command** | worker/command failure | **undefined — coverage gap** | **undefined** | **undefined** |
| **Compact viewport** | narrow window | no breakpoint — **coverage gap** | panes crush | panes crush |

When you add a state, add its row here, its status line in `app.tsx`, and its visual in the surface.
When a row says "coverage gap," design the state before you claim the surface handles it, and update
`coverage-gaps.md`.

## Routing by task

- Changing the toolbar, the tools, tool-enable rules, or status text → `surfaces-app-shell.md` + `copy.md`.
- Changing how the footprint is drawn, snapped, closed, or shown → `surfaces-plan-view.md`.
- Changing push/pull, orbit, camera framing, picking, or the mass mesh → `surfaces-3d-view.md`.
- Changing a user-facing noun → `glossary.md`, then the owner `apps/web/CONTEXT.md`.
