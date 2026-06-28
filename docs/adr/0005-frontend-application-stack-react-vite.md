# ADR 0005 — Frontend application stack: React + Vite for `apps/web`

- **Status:** Accepted
- **Date:** 2026-06-28
- **Context doc:** [`docs/plans/repo-scaffold.md`](../plans/repo-scaffold.md) §2, §8 (Phase 4); supersedes the web-bundler slice of [ADR 0001](./0001-monorepo-tooling-and-layout.md)

## Context

The Phase 4 slice shipped `apps/web` as deliberately minimal "hands & eyes": vanilla
TypeScript, a single immediate-mode `<canvas>`, no framework, and a hand-rolled `Bun.serve`
dev script (`apps/web/serve.ts`) bundling on start via `bun build`. The next step — an initial
drawing UX with **side-by-side plan and 3D viewports**, a tool palette, and an app shell that
organizes them — introduces real, interactive UI state (active tool, selection, pane focus and
sizing) and a tight edit→see loop that the original slice was not built to carry.

Two structural questions had to be settled, and both recur for every screen added later.

1. **Whether the shell adopts a UI framework.** The two render surfaces (the 2D plan canvas
   and the 3D canvas) are inherently *imperative* — a retained-DOM tree does not help them and
   tends to fight them. The surrounding chrome (toolbar, status, splitter, and the inspector
   panels that will follow), by contrast, is ordinary interactive UI that benefits from a
   reactive model.
2. **What bundles the app.** [ADR 0001](./0001-monorepo-tooling-and-layout.md) recorded **Bun**
   as the bundler. `bun build` has no dev-server HMR (the current `serve.ts` rebundles only on
   process start), and Bun's React Fast Refresh story is not yet at parity with the established
   React tooling, which assumes a Vite-class host.

## Decision

1. **The app shell is React.** React owns the chrome — app shell, tool palette, status, panel
   layout — and the interactive state behind it. The two viewports remain imperative renderers
   mounted into React-managed containers (`ref` + an effect that drives the canvas / Three.js
   scene); React never owns canonical geometry or per-frame draw state. **TanStack** libraries
   are adopted **selectively** — pulled in where shell/tool/selection or server state actually
   warrants them, not wholesale — consistent with the repo's YAGNI posture.

2. **`apps/web` is bundled by Vite; the scope is exactly this one package.** Vite provides the
   dev server (HMR / React Fast Refresh) and the production build. The wasm engine loads via
   `vite-plugin-wasm`; the engine worker continues to spawn through
   `new Worker(new URL("./engine-worker.ts", import.meta.url), { type: "module" })`, which Vite
   supports natively and which `apps/web/src/main.ts` already uses — so the worker seam survives
   the migration unchanged. Everything else is untouched: **Bun** remains the workspace package
   manager and runtime, `packages/*` and `tooling/codegen` keep building under Bun, `apps/api`
   stays Hono-on-Bun, and **Turbo** remains the orchestrator — its `build` target for the web
   package simply invokes `vite build`.

3. **This supersedes the web-bundler portion of [ADR 0001](./0001-monorepo-tooling-and-layout.md)
   only.** `apps/web` no longer builds with `bun build` + `serve.ts`. The rest of ADR 0001
   (Bun + Cargo + Turborepo as the monorepo spine) stands unchanged.

## Consequences

- The first chunk of the drawing-UX work is a **build migration** (Bun→Vite for `apps/web`,
  wasm-pack wiring moved under `vite-plugin-wasm`, `serve.ts` retired) before any plan/3D pixels
  appear. This is a one-time tax; the existing dev port (`5173`) already matched Vite's default.
- A new frontend dependency surface (Vite + its plugins, React, selected TanStack libs) lands in
  `apps/web` only. The "one bundler everywhere" simplicity of ADR 0001 is traded for app-grade
  dev ergonomics where they pay off; the libraries and the server keep the Bun toolchain.
- The imperative/React boundary is load-bearing and mirrors the engine boundary: just as render
  stays out of the domain, React stays out of per-frame rendering. Viewports expose imperative
  handles; React drives them via effects, never the reverse.

## Alternatives considered

- **Stay vanilla, defer a framework.** Rejected given a *formed* roadmap of interactive chrome
  (plan + 3D panes now, inspector/editor panels next). The framework earns its keep on that
  chrome; adopting it now avoids a disruptive retrofit once panels land.
- **Keep Bun as the bundler and add a React HMR layer on top.** Rejected: it fights the
  toolchain to save a dependency, and the React/TanStack ecosystem assumes a Vite-class host.
- **Adopt Vite workspace-wide.** Rejected as out of scope and YAGNI: the libraries and the Hono
  server build fine under Bun; Vite's value is app dev-server ergonomics, which only `apps/web`
  needs. Scoping it keeps the monorepo story "Bun + Cargo + Turbo" with one app-level exception.
