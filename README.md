# Parametric Residential Framing Tool

A constrained BIM engine for parametric residential framing — a Rust/WASM compute engine
("brain") and a TypeScript frontend ("hands & eyes"), kept honest by a single domain MODEL
that generates the shared `BufferLayout`.

The repository is mid-scaffold: the **design schema is complete**, and the **monorepo
spine** (workspaces + codegen) has landed. Domain crates and apps land next, per the plan.

## Layout

```
schema/     ⭐ single source of truth — the domain MODEL (12 layers, 178 types)
crates/     🦀 Rust engine — one crate per bounded context (Phase 2+)
packages/   🌐 shared TS (generated model types, render mirror, tools)
apps/       deployable surfaces — web, api (Phase 4–5)
tooling/    repo tooling — codegen (the MODEL → TS/Rust spine)
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
