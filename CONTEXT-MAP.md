# Context Map

The bounded contexts in this repo and how they relate. Each context owns its ubiquitous
language in a local `CONTEXT.md` (a glossary — terms only, no implementation). This map is the
index; it is seeded lazily, so a context without a `CONTEXT.md` yet simply has not had its
language pinned down in a domain-modeling session.

The architecture itself is recorded in [`docs/plans/repo-scaffold.md`](./docs/plans/repo-scaffold.md)
and the ADRs under [`docs/adr/`](./docs/adr/); this map only names the contexts and their edges.

## Contexts

**Shared kernels** (pure-domain, zero-dependency; referenced by everything downstream):

- **geometry-kernel** (`crates/geometry-kernel`) — the Tick base unit, vectors, quaternions,
  planes, transforms, and the extrusion BREP kernel.
- **reference-data** (`crates/reference-data`) — citation keys, mechanical properties, and the
  design-value / prescriptive flyweight registries.

**Pipeline contexts** (the dependency order *is* the pipeline order):

- **materials** (`crates/materials`) — stock, dimensions/section properties, the
  piece→cut→takeoff provenance chain, the supplier/price catalog.
- **building** (`crates/building`) — wall/opening/junction promotion, member placement and
  install context, the grid-anchored framing solver.
- **loads-analysis** (`crates/loads-analysis`) — ASCE 7 sources, tributary area, load
  path/rollup, load combinations, member demand.
- **design-standard** (`crates/design-standard`) — the `DesignStandard` Strategy seam; a
  material-blind sizing core with an NDS wood leaf and stub leaves for other standards.

**Supporting contexts:**

- **cut-optimization** (`crates/cut-optimization`) — the cutting-stock solver and cut-plan handoff.
- **estimating** (`crates/estimating`) — the takeoff traceability chain and deterministic cost rollup.
- **drawings-export** (`crates/drawings-export`) — projection + hidden-line removal into sheets.

**Composition root & seams** (not bounded contexts — they wire and marshal, defining no domain types):

- **bim-core** (`crates/bim-core`) — the composition root: owns the `Session` and the canonical
  `MemberBuffer`, translates a `Command` into the context pipeline.
- **bim-wasm** (`crates/bim-wasm`) — the single FFI seam (wasm-bindgen glue).
- **apps/api** (`apps/api`) — the domain-orthogonal persistence boundary (Hono/Bun, Drizzle/Neon + R2).

**Frontend ("hands & eyes"):**

- **[web](./apps/web/CONTEXT.md)** (`apps/web` + `packages/render-mirror`, `packages/tool-runner`,
  `packages/model-types`) — the draw→render client: captures gestures, mirrors the canonical
  buffer read-only, and renders it. **Has a `CONTEXT.md`.**

## Relationships

- **materials → building → loads-analysis → design-standard** — the core pipeline; each context
  reaches the next only through its `lib.rs` facade. Where the model couples two contexts in both
  directions, the upstream one references the downstream by opaque key newtype (see
  [ADR 0002](./docs/adr/0002-core-context-crate-layout-and-dependency-direction.md)).
- **bim-core → (all pipeline contexts)** — the composition root drives the pipeline and writes
  the canonical Structure-of-Arrays buffer.
- **bim-core → bim-wasm → web** — the engine ships SoA bytes across the FFI seam; the **web**
  context cuts zero-copy views over those bytes and renders them. One direction only: render never
  mutates canonical geometry (see [ADR 0003](./docs/adr/0003-wasm-boundary-and-the-buffer-layout-keystone.md)
  and [ADR 0006](./docs/adr/0006-world-space-placement-engine-side.md)).
- **web → bim-wasm** — gestures cross into the engine as a `Command`; nothing else flows upstream.
- **(supporting contexts)** — `cut-optimization`, `estimating`, and `drawings-export` consume the
  pipeline's output bottom-up; see [ADR 0004](./docs/adr/0004-supporting-contexts-and-the-persistence-boundary.md).
