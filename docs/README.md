# Documentation Index

Design documentation for the parametric residential framing engine. Organized by **status**: what's current, the analysis behind it, the still-active reference, and the superseded source artifacts kept for provenance.

```
docs/
├── schema/      current deliverable — the unified domain model
├── analysis/    the cross-schema audit that produced it
├── reference/   still-active bibliography (referenced, not duplicated)
├── plans/       forward-looking plans (repo scaffold, rollout)
└── adr/         architecture decision records
```

## schema/ — current

| File | What it is |
|---|---|
| [`unified-schema.html`](schema/unified-schema.html) | **The deliverable.** One cohesive, future-proof schema — 12 layers, 178 types, 10-stage pipeline. Renders human cards + a machine-readable MODEL in the shared visual language. Includes migration map, alias table, changelog, and resolved/open decisions. **v1.0.1.** |

> The machine contract (the MODEL object, for codegen / tooling) now lives at [`schema/model/unified-model.json`](../schema/model/unified-model.json) — promoted out of `docs/` so it's a real build input. This `docs/schema/` directory keeps the human-readable rendering only.

Consolidates the six prior artifacts; designed for two forward requirements — **additional material types** (via the `DesignStandard` Strategy seam + open registry keys) and **project estimating / cost** (the `estimating-cost` layer).

## analysis/ — the audit

| File | What it is |
|---|---|
| [`cross-schema-analysis.md`](analysis/cross-schema-analysis.md) | Deep cross-schema architecture analysis, findings ranked by severity (S1→S3), with the recommended order of operations. The blueprint the unified schema was built from. |

> The type-ownership registry (every type → canonical home → cross-refs, plus collisions, dangling refs, base-unit audit) now lives at [`schema/registry/type-registry.json`](../schema/registry/type-registry.json) — promoted alongside the MODEL as a build input.

## reference/ — still active

| File | What it is |
|---|---|
| [`reference-library.html`](reference/reference-library.html) | 23 building-science titles + a `subjectIndex` mapping modeling subjects → `{book, anchor, note}`. The unified schema *references* this (via `CitationKey`); it was not absorbed, so it stays live. |

## plans/ — forward-looking

| File | What it is |
|---|---|
| [`repo-scaffold.md`](plans/repo-scaffold.md) | DDD monorepo scaffold: maps the 12 schema layers → bounded contexts, defines the Rust-engine / TS-frontend split, the MODEL→codegen spine, naming/boundary conventions, tidiness governance, and a phased rollout. **Phases 1–4 landed** (skeleton + spine, shared kernels, core contexts, the wasm boundary + frontend slice). |

## adr/ — decisions

| File | What it is |
|---|---|
| [`0001-monorepo-tooling-and-layout.md`](adr/0001-monorepo-tooling-and-layout.md) | Accepted: Bun + Cargo + Turborepo; backend in-repo; `crates/`+`packages/`+`apps/` split; codegen as the drift-checked spine. |
| [`0002-core-context-crate-layout-and-dependency-direction.md`](adr/0002-core-context-crate-layout-and-dependency-direction.md) | Accepted: hexagonal-as-deep-as-warranted crate layout; pipeline-order dependencies with downstream links held by opaque key (breaking the loads ↔ design-standard cycle). |
| [`0003-wasm-boundary-and-the-buffer-layout-keystone.md`](adr/0003-wasm-boundary-and-the-buffer-layout-keystone.md) | Accepted: `bim-core` composition root; the `BufferLayout` generated to both Rust + TS from one spec with a shared `LAYOUT_HASH`; `bim-wasm` as the single `unsafe`-allowed FFI seam. |

## Provenance — the five superseded source artifacts

The original self-contained artifacts were **fully consolidated into `schema/unified-schema.html`** (all 70 MODEL types captured directly or via a documented rename; `architecture.html`'s concepts formalized into the `system-architecture` layer) and then **removed** from the working tree. They remain recoverable from git history — last present at commit `7f3dbf3` (e.g. `git show 7f3dbf3:docs/archive/lumber-schema.html`).

| Original artifact | Became (in the unified schema) |
|---|---|
| `lumber-schema.html` | `materials-stock` + geometry-kernel primitives + render-adapter |
| `framing-solver-schema.html` | `building-placement` + `cut-optimization` + (structural → `design-standard-seam`) |
| `drawing-workspace-schema.html` | `workspace-render` |
| `design-standard-schema.html` | `design-standard-seam` (Strategy seam) + `reference-flyweights` |
| `architecture.html` | `system-architecture` (formalized into a MODEL) |

The type-level mapping lives in the **Migration Map** panel inside `unified-schema.html`; the audit that drove the consolidation is in [`analysis/`](analysis/cross-schema-analysis.md).
