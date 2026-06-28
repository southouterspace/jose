# Parametric Residential Framing Tool

A constrained BIM engine for parametric residential framing — a Rust/WASM compute engine
("brain") and a TypeScript frontend ("hands & eyes"), kept honest by a single domain MODEL
that generates the shared `BufferLayout`.

The repository is mid-scaffold: the **design schema is complete**, the **monorepo spine**
(workspaces + codegen) and **shared kernels** have landed, the **four core domain contexts**
(`materials`, `building`, `loads-analysis`, `design-standard`) are in place, and the
**boundary + frontend slice** now closes the loop end-to-end: a `bim-core` composition root +
the `bim-wasm` wasm boundary + the `render-mirror` / `tool-runner` packages + the `apps/web`
browser app — **draw a wall → recompute in Rust → render the framed elevation**. The
supporting contexts and backend (`cut-optimization`, `estimating`, `drawings-export`,
`apps/api`) land next, per the plan.

## Layout

```
schema/     ⭐ single source of truth — the domain MODEL (12 layers, 178 types)
crates/     🦀 Rust engine — contexts + bim-core (composition root) + bim-wasm (boundary)
packages/   🌐 shared TS (generated model types, render mirror, tool runner)
apps/       deployable surfaces — web (slice landed), api (Phase 5)
tooling/    repo tooling — codegen (the MODEL → TS/Rust spine, incl. BufferLayout)
docs/       design docs, ADRs, plans
```

## Start here

- **Repo organization & roadmap** → [`docs/plans/repo-scaffold.md`](docs/plans/repo-scaffold.md)
- **How to contribute / keep it tidy** → [`CONTRIBUTING.md`](CONTRIBUTING.md)
- **The schema (human-readable)** → [`docs/schema/unified-schema.html`](docs/schema/unified-schema.html)
- **Index of every document** → [`docs/README.md`](docs/README.md)

## Quickstart

```bash
bun install         # JS/TS deps
bun run codegen     # generate the model surface from schema/
bun run typecheck   # verify across packages
cargo check --workspace
```
