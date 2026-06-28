# Contributing

How the repo is organized and kept tidy. The full rationale is in
[`docs/plans/repo-scaffold.md`](docs/plans/repo-scaffold.md); this is the short version.

## Layout

```
schema/     ⭐ single source of truth — the domain MODEL (edit here)
crates/     🦀 the Rust engine — one crate per bounded context (Phase 2+)
packages/   🌐 shared TS (generated model types, render mirror, tools)
apps/       deployable surfaces (web, api)
tooling/    repo tooling — codegen lives here
docs/       design docs, ADRs, plans
```

`crates/` = domain & engine (pure, framework-free). `packages/` = shared TS. `apps/` =
thin deployables that wire things together. Nothing else lives at the top level without an
ADR.

## The golden rule: edit the model, not the generated files

`schema/model/unified-model.json` drives codegen. Files under any `generated/` directory
carry a `// @generated` header and **must not be hand-edited**. After changing the model:

```bash
bun run codegen        # regenerate
bun run typecheck      # verify
```

CI runs `bun run codegen:check` and fails on any drift.

## Common commands

```bash
bun install            # install JS/TS deps (one bun.lock)
bun run codegen        # regenerate the model surface
bun run typecheck      # turbo-run typecheck across packages
bun run build          # codegen + turbo build
cargo check --workspace
cargo fmt --all && cargo clippy --workspace --all-targets
```

## Adding a bounded context (Phase 2+)

1. `crates/<context>/` with the standard hexagonal layout: `domain/ application/ ports/
   adapters/`. Dependencies point inward only.
2. Register the crate in the root `Cargo.toml` `members`.
3. Cross-context calls go through a context's `lib.rs` facade — never into another
   context's `domain/`.
4. Record any structural decision as a new ADR in `docs/adr/`.
