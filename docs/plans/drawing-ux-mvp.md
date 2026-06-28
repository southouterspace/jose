# Plan — Initial Drawing UX (space-first MVP)

The first interactive modeling slice: **draw a footprint in plan → push/pull it into a mass in
3D**, inside a React app shell. Framing is deferred. This plan sequences the work and records the
smaller engineering calls; the load-bearing decisions live in the ADRs it references.

## What we're building (and not)

**In:** a React + Vite app shell with two viewports (plan + 3D); draw a closed footprint in plan;
push/pull the top cap in 3D to set height; both panes mirror one canonical model.

**Out (deferred, by decision):** framing / studs in 3D ([ADR 0006](../adr/0006-world-space-placement-engine-side.md)),
general any-face push/pull and footprint carving (needs a BREP solid modeler —
[ADR 0007](../adr/0007-space-first-modeling-footprint-push-pull.md)), the elevation view, and
inspector/property panels.

## Decisions this plan rests on

- [ADR 0005](../adr/0005-frontend-application-stack-react-vite.md) — React + Vite for `apps/web`.
- [ADR 0007](../adr/0007-space-first-modeling-footprint-push-pull.md) — space-first flow; both panes
  are input surfaces; staged push/pull.
- [ADR 0008](../adr/0008-mvp-geometry-and-command-contract.md) — the geometry & command contract
  (`footprint` + `volume` buffers; `DrawFootprint` / `PushPull`; JS tessellates the mass; push/pull
  references the kernel's `TOP_FACE`).

## Phases (each keeps `main` green)

1. **Vite migration of `apps/web`.** Replace `bun build` + `serve.ts` with Vite; React shell
   skeleton (toolbar + two empty panes + status); wasm via `vite-plugin-wasm`; keep the engine
   worker (`new Worker(new URL(...))`, already Vite-native). Amend tooling per ADR 0005. *No
   geometry yet — just the rails.*
2. **Schema: the two buffers.** Add `footprint` (world-XY vertex columns + `spaceId`) and `volume`
   (`volumeId`, `height`, base-plane ref) to `schema/model/buffer-layouts.json`; `bun run codegen`;
   `LAYOUT_HASH` moves on both sides under the drift gate.
3. **Engine: the space-first pipeline.** In `bim-core`, handle `DrawFootprint` (closed ring → kernel
   `extrude` → `Volume`) and `PushPull` (→ `apply_push_pull` on `TOP_FACE`); write the `footprint` +
   `volume` buffers. `DrawWall` stays for the later framing layer.
4. **Plan view.** React-mounted 2D surface: draw/close a footprint, picks → `DrawFootprint`. The
   mid-draw polyline is **client-only** state (ADR 0008) — only the closed ring becomes a command.
5. **3D view.** React-mounted imperative Three.js viewport (per ADR 0005): tessellate the mass from
   the mirror; raycast the **top cap** → `PushPull { volumeId, faceIndex, distance }`; orbit
   otherwise.
6. **Sync.** One recompute → one buffer rewrite → both panes re-read the same mirror.

## Smaller engineering calls (mine to make; recorded so they're not re-litigated)

- **In-progress footprint** lives in the client; the engine sees only closed rings (ADR 0008).
- **Face picking** uses a raycast resolved to the canonical `TOP_FACE` — not a surface-normal
  guess; a vertical drag delta maps to push/pull `distance`.
- **Cameras:** plan is orthographic top-down (world XY); the 3D view opens at an angled orbit so the
  mass reads as a solid (the prototype's `(14, 12, 16)` framing is a good default).
- **Units:** ticks (1/32in) are canonical; the UI displays feet/inches and snaps picks to a sensible
  increment.
- **Selection/active-tool** state lives in React; TanStack is pulled in only if/when this state
  outgrows plain React state (ADR 0005's "selectively").

## Throwaway prototype

A disposable feel-spike (Vite + React + vanilla Three, geometry mocked in JS — no engine) validated
the draw→push/pull loop and the two-input-surface model before this plan was written. It is **not**
the architecture (it deliberately breaks the one-direction rule for speed) and is not committed by
default. Its only lasting output is the confidence behind ADR 0008's contract.
