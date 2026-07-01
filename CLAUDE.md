# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

A constrained BIM engine for parametric residential framing: a Rust/WASM compute engine
("brain") and a TypeScript frontend ("hands & eyes"), kept honest by a single domain MODEL
that generates the shared `BufferLayout`. The repo is scaffold-complete; crates land
incrementally per `docs/plans/repo-scaffold.md` §8.

## The one rule that governs everything: edit the model, not the generated files

`schema/model/` is the single source of truth. `tooling/codegen` reads it and emits the Rust
structs, the TS types, and the `BufferLayout` byte-offset table. Anything under a `generated/`
directory carries a `// @generated` header and **must not be hand-edited** — your change will be
overwritten and CI will fail.

```bash
bun run codegen        # regenerate the model surface after editing schema/
bun run codegen:check  # what CI runs — any drift vs committed generated files fails the build
```

A new SoA buffer or column is a row in `schema/model/buffer-layouts.json` + `bun run codegen`,
never a hand-edit to `layout.rs` or `layout.ts`.

## Commands

```bash
# JS / TS (run from repo root)
bun install                     # one bun.lock; deps share versions via the workspace catalog
bun run typecheck               # codegen + tsc across every package (turbo)
bun run lint                    # Biome (Ultracite preset) — check only
bun run format                  # Biome — apply fixes + format
bun run test                    # bun test across packages (turbo)
bun run build                   # codegen + turbo build (web bundles wasm)

# A single TS test (bun's runner)
bun test packages/tool-runner/src/index.test.ts
bun test -t "round-trips"       # by test-name pattern
turbo run test --filter=api     # one package's suite

# Rust (cargo workspace)
cargo check --workspace
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p loads-analysis tributary        # one crate, one test
cargo build -p bim-wasm --target wasm32-unknown-unknown   # prove the boundary builds

# Run the apps (per-package; no root dev script)
cd apps/web && bun run dev      # builds wasm-pack output, serves the draw→render slice
cd apps/api && bun run dev      # bun --hot Hono server (persistence boundary)
```

`apps/web` dev/build needs `wasm-pack` and the `wasm32-unknown-unknown` target installed.

## Architecture: the big picture

The system is two halves joined by one generated contract. Read these together —
`schema/README.md`, `docs/adr/0003-*`, and `packages/render-mirror/src/index.ts` — before
touching the boundary.

**The runtime loop (draw → recompute → render).** `apps/web` captures a draw gesture →
`tool-runner` turns picks into a `Command` → the command crosses into the `bim-wasm` worker →
`bim-core` (the composition root) runs the context pipeline and writes the canonical
Structure-of-Arrays buffer → the worker ships the bytes back → `render-mirror` cuts zero-copy
typed-array views over those same bytes and renders. One direction only: nothing on the render
side mutates canonical geometry.

**The `BufferLayout` keystone (why the two languages can't drift).** The SoA columns are declared
once in `schema/model/buffer-layouts.json`. Codegen computes byte offsets (pure column-major:
each column is `CAPACITY` contiguous elements, so a JS view is zero-copy over one column) and
emits the *identical* table to both `crates/bim-core/src/generated/layout.rs` (writer) and
`packages/model-types/src/generated/layout.ts` (reader), plus a shared `LAYOUT_HASH` (FNV-1a of
the offsets). The engine reports its hash; `render-mirror`'s `assertLayout` checks it at startup,
so a stale build fails loudly instead of corrupting reads. `codegen:check` makes "cannot drift" a
mechanical, cross-language guarantee.

**Rust crate layout & dependency direction (DDD, enforced by the compiler).** One crate per
bounded context under `crates/`, each exposing itself through a `lib.rs` facade. Cross-context
calls go through that facade — never into another context's `domain/`. Dependencies point inward
and the crate graph is acyclic; the pipeline order *is* the dependency order
(`materials → building → loads-analysis → design-standard`). Where the MODEL couples two contexts
in both directions, the upstream crate references the downstream one by **opaque key newtype**
(e.g. `ConnectionGraphRef` in `loads-analysis`), the same reference-by-key idiom the schema uses
for every cross-layer link.

**`bim-core` is a composition root, not a context.** It defines no domain types; it owns the
`Session` (in-session system of record), translates a `Command` into the context pipeline, and
owns the `MemberBuffer`. `bim-wasm` is a thin marshaling shell over it and is the *only* crate
that opts out of `unsafe_code = "deny"` (wasm-bindgen glue) — the single FFI seam.

**`apps/api` is a domain-orthogonal persistence boundary.** Neon/Drizzle + R2, structured as
ports/adapters so storage stays out of the domain, mirroring how render stays out of the domain.

## Conventions to respect (these encode YAGNI / DRY / DDD here)

- **DRY lives in the MODEL and the catalog.** Shared shapes come from `schema/` via codegen;
  shared dep versions come from the root `package.json` `workspaces.catalog` (members use
  `"catalog:"`). Don't re-declare either by hand.
- **YAGNI — don't fabricate empty layers.** Hexagonal structure goes only as deep as a context
  warrants: kernels and pure-domain crates skip `ports/`/`adapters/`; `design-standard` has real
  ones because its `DesignStandard` Strategy trait is a genuine port. Add a layer when it carries
  weight, not by template.
- **DDD boundaries are load-bearing.** Keep render and persistence out of `domain/`; reach other
  contexts only through their `lib.rs` facade; model downstream references as keys, not deps.
- **Structural changes are ADRs.** Nothing new lives at the top level, and no dependency direction
  or boundary changes, without an ADR in `docs/adr/`.
- **Linting/formatting is Biome with the Ultracite preset** (`biome.jsonc`). It deliberately
  excludes `docs/**` and `schema/**` (hand-authored deliverable/data, not source) and the
  `generated/` output; don't widen those without reason.
- **User-facing work starts with the `product-design` skill.** Any change to what a user sees,
  understands, chooses, or does in `apps/web` (a flow, viewport, toolbar, status/copy, tool, or a
  reachable state) — and any audit/review of it — **invokes the `product-design` skill first**, before
  writing code. It routes the canonical language (`apps/web/CONTEXT.md`), the per-surface specs, the
  one-direction rule, the reachable-state map, and the verify step (which includes rendering the real
  viewport — never claim visual verification from code alone). When you finish, sync the surface docs
  and `CONTEXT.md` nouns you touched. Skip only for engine/domain work in `crates/` with no
  user-visible effect, the MODEL/generated files, `apps/api`, or build tooling.
