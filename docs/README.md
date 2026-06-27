# Documentation Index

Design documentation for the parametric residential framing engine. Organized by **status**: what's current, the analysis behind it, the still-active reference, and the superseded source artifacts kept for provenance.

```
docs/
├── schema/      current deliverable — the unified domain model
├── analysis/    the cross-schema audit that produced it
└── reference/   still-active bibliography (referenced, not duplicated)
```

## schema/ — current

| File | What it is |
|---|---|
| [`unified-schema.html`](schema/unified-schema.html) | **The deliverable.** One cohesive, future-proof schema — 12 layers, 178 types, 10-stage pipeline. Renders human cards + a machine-readable MODEL in the shared visual language. Includes migration map, alias table, changelog, and resolved/open decisions. **v1.0.1.** |
| [`unified-model.json`](schema/unified-model.json) | The standalone machine contract (the MODEL object), for codegen / tooling. |

Consolidates the six prior artifacts; designed for two forward requirements — **additional material types** (via the `DesignStandard` Strategy seam + open registry keys) and **project estimating / cost** (the `estimating-cost` layer).

## analysis/ — the audit

| File | What it is |
|---|---|
| [`cross-schema-analysis.md`](analysis/cross-schema-analysis.md) | Deep cross-schema architecture analysis, findings ranked by severity (S1→S3), with the recommended order of operations. The blueprint the unified schema was built from. |
| [`type-registry.json`](analysis/type-registry.json) | Machine-readable type-ownership registry: every type → canonical home → cross-refs, plus collisions, dangling refs, base-unit audit, and pipeline coherence. |

## reference/ — still active

| File | What it is |
|---|---|
| [`reference-library.html`](reference/reference-library.html) | 23 building-science titles + a `subjectIndex` mapping modeling subjects → `{book, anchor, note}`. The unified schema *references* this (via `CitationKey`); it was not absorbed, so it stays live. |

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
