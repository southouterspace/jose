# ADR 0001 — Monorepo tooling and layout

- **Status:** Accepted
- **Date:** 2026-06-27
- **Context doc:** [`docs/plans/repo-scaffold.md`](../plans/repo-scaffold.md)

## Context

The repo is a polyglot system: a Rust/WASM compute engine (the "brain") and a TypeScript
frontend (the "hands & eyes"), kept honest by a single domain MODEL that generates the
shared `BufferLayout`. We need a monorepo layout and toolchain that keeps the two
languages building together without drift.

## Decision

1. **Tooling: Bun + Cargo + Turborepo.** Bun is the package manager, JS/TS runtime, and
   test runner (one `bun.lock`, workspaces in the root `package.json`). Cargo owns the Rust
   workspace. Turborepo orchestrates and caches tasks across both.
2. **Backend lives in this repo** as `apps/api` on the Bun runtime (Neon/Drizzle +
   Cloudflare R2); Hono keeps Bun/Workers deploy targets open.
3. **Split top-level layout:** `crates/` (Rust engine), `packages/` (shared TS), `apps/`
   (deployables) — over a single unified `packages/`. Idiomatic to each toolchain and
   instantly legible in a polyglot tree.
4. **Codegen is the spine.** `schema/model/unified-model.json` is the single source of
   truth; `tooling/codegen` generates the TS surface (and, later, Rust + `BufferLayout`).
   Generated files are committed and drift-checked in CI (`bun run codegen:check`).

## Consequences

- One `bun install` and one `cargo` toolchain; CI gates drift mechanically.
- Adding a bounded context = a new crate under `crates/` (Phase 2+); no top-level churn.
- Turbo is optional early (Bun's `--filter` can stand in) but kept for cross-language
  caching and the codegen→build dependency graph.

## Alternatives considered

- **moonrepo / Bazel** — first-class polyglot graphs, but heavy setup/cognitive cost.
- **Unified `packages/` for everything** — fine for single-language repos; muddier here.
- **pnpm/npm instead of Bun** — viable, but Bun was chosen for speed and the
  runtime+test-runner+PM consolidation.
