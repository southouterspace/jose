# Jose scope for vercel-react-best-practices

The upstream skill targets React **and Next.js**. Jose's web app is a **React + Vite client SPA**
(`apps/web`) with an **imperative Three.js renderer** and a wasm engine in a Web Worker. There is
**no Next.js, no RSC/SSR, no app-router, no server actions, no SWR, no SSR hydration**, and the only
data "fetch" is the worker message boundary. So most of the 70 rules have no surface here.

Apply this scope before reporting findings: run only the **[applies]** categories; treat
**[n/a]** as noise and don't report it (note it as a future coverage gap if that surface ever lands).

## Category scope

| Category | Status | Why |
| -------- | ------ | --- |
| 1. Eliminating Waterfalls (`async-`) | **[mostly n/a]** | No API routes / RSC. The only async is wasm worker init + `postMessage`; `async-parallel` applies *if* independent worker/asset loads are ever awaited in series. |
| 2. Bundle Size (`bundle-`) | **[partial]** | `bundle-barrel-imports` / `bundle-analyzable-paths` apply (Vite tree-shaking; watch barrel imports from `three` and the workspace packages). `bundle-dynamic-imports` is `next/dynamic` → **n/a**; a plain dynamic `import()` for a heavy view is the Vite equivalent. `bundle-defer-third-party` → n/a (no analytics). |
| 3. Server-Side (`server-`) | **[n/a]** | No server runtime. Entire category off. |
| 4. Client Data Fetching (`client-`) | **[partial]** | `client-event-listeners` / `client-passive-event-listeners` apply — the 3D view adds pointer/resize listeners (`three-view.tsx`); confirm dedup + passive where scroll-like. `client-swr-dedup` → n/a. `client-localstorage-schema` → n/a (no localStorage yet). |
| 5. Re-render (`rerender-`) | **[applies]** | The core React surface (`app.tsx`, `plan-view.tsx`, `engine-store.ts`). High-value: `rerender-use-ref-transient-values`, `rerender-no-inline-components`, `rerender-derived-state(-no-effect)`, `rerender-functional-setstate`, `rerender-dependencies`. |
| 6. Rendering (`rendering-`) | **[partial]** | `rendering-hoist-jsx`, `rendering-conditional-render` (ternary not `&&`), `rendering-svg-precision` (the plan view emits SVG) apply. `rendering-hydration-*` → n/a (no SSR). `rendering-activity`/resource-hints → low surface. |
| 7. JavaScript Performance (`js-`) | **[applies]** | The hot paths: `mass-tessellation.ts`, the SoA mirror reads, grid generation in `plan-view.tsx`. `js-hoist-regexp`, `js-early-exit`, `js-combine-iterations`, `js-index-maps`, `js-cache-property-access`, `js-set-map-lookups`, `js-min-max-loop` are all relevant. |
| 8. Advanced (`advanced-`) | **[applies]** | Directly matches the imperative-renderer pattern: `advanced-event-handler-refs`, `advanced-use-latest`, `advanced-init-once`, `advanced-effect-event-deps`. |

## Jose-specific notes

- **The imperative Three.js renderer already uses the ref idiom these rules prescribe.**
  `three-view.tsx` keeps `toolRef`/`pushPullRef`/`handleRef` so pointer handlers read live values
  without re-subscribing — that *is* `advanced-event-handler-refs` / `rerender-use-ref-transient-values`.
  Treat that as the established pattern (`patterns.md#pattern/imperative-renderer-in-react`); flag
  deviations from it, not the pattern itself.
- **The mount-once scene effect** is `advanced-init-once` in spirit. The rebuild effect's dependency
  array (`[store.footprint, store.volume]`) is the place to apply `rerender-dependencies` /
  `advanced-effect-event-deps` scrutiny.
- **`mass-tessellation.ts` and per-frame/per-rebuild loops** are where `js-*` rules earn their keep —
  this is geometry math on every footprint/height change.
- **Respect Jose's canonical decisions over generic perf advice.** React state for tool/selection is
  deliberate (ADR 0005); don't recommend a state library on perf grounds. The one-direction render
  rule and the mirror-reads pattern are load-bearing — never suggest caching canonical geometry
  client-side to "save a read."

## Reporting

Same terse format as `/web-interface-guidelines`: group by file, `file:line`, name the rule id (e.g.
`rerender-no-inline-components`), fix only if non-obvious. Report only `[applies]`/`[partial]`
categories. For a deep example of a rule, point to the upstream `rules/<id>.md` (not vendored).
