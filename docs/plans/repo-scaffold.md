# Repository Scaffold Plan — DDD Monorepo

**Status:** Proposed · **Scope:** repository organization & tidiness conventions only (no application code written here)
**Subject:** Parametric Residential Framing Tool — a constrained BIM engine

This plan describes *how the repository should be organized and kept tidy* as it grows
from today's docs-only state into a working monorepo. It does **not** implement the
scaffold — it defines the target layout, the Domain-Driven Design (DDD) mapping, the
single-source-of-truth codegen spine, and the governance rules that keep the tree clean.

---

## 1. Why this shape

The existing schema (`docs/schema/unified-schema.html` + `unified-model.json`) already
implies the runtime architecture. The plan simply makes the directory tree *match the
model that is already designed*:

| The schema already says… | …so the repo should have |
|---|---|
| A Rust/WASM Web Worker owns the canonical SoA tick arrays + dependency-graph recompute (the "brain") | A **Rust workspace** of bounded-context crates + a wasm boundary crate |
| The browser main thread owns a read-only render mirror + ToolRunner (the "hands & eyes") | A **TS/JS workspace** for the frontend, render mirror, and tools |
| `BufferLayout` byte offsets are **GENERATED from the single domain MODEL** so the Rust writer and JS reader cannot drift | A **canonical `schema/` source** + a **`tooling/codegen/`** package that emits both Rust and TS |
| Persistence is Neon/Postgres + Drizzle snapshots + Cloudflare R2 blobs, *orthogonal to the domain* | A **backend service** kept out of the domain crates |
| 12 layers, each with a clear purpose and a `pipelineStage` | 12 → **bounded contexts**, wired into one pipeline at a composition root |

Three guiding principles fall out of this:

1. **Screaming architecture.** Top-level names announce the *domain* (materials, loads,
   structural, estimating), not the framework. A newcomer sees what the system *does*
   before they see what it's built with.
2. **One model, generated outward.** The domain MODEL is the single source of truth.
   Rust structs, TS types, and the `BufferLayout` keystone are all *generated* — never
   hand-maintained in parallel. This is the spine that keeps the two languages honest.
3. **The domain depends on nothing.** Geometry, materials, and structural logic are pure.
   Persistence, rendering, WASM, and the network are *adapters* at the edges. Dependencies
   point inward only.

---

## 2. Monorepo tooling (decided)

A polyglot Rust + TypeScript monorepo. The stack is **Bun + Cargo + Turborepo**:

- **Bun workspaces** — the package manager, JS/TS runtime, and test runner. One
  `bun install`, one `bun.lock`, `bun test`. Workspaces are declared in the root
  `package.json` (`"workspaces": ["apps/*", "packages/*"]`); Bun globs them.
- **Cargo workspace** (`Cargo.toml` at root) — manages all Rust crates, one lockfile,
  shared profiles. Bun does not touch Rust; `cargo` stays the Rust toolchain.
- **Turborepo** (`turbo.json`) — task orchestration + caching across both worlds
  (`build`, `test`, `lint`, `codegen`, `typecheck`). Turbo shells out to `cargo` for Rust
  tasks and to `bun` for TS tasks, so a single `bunx turbo run build` builds the whole
  repo, with `codegen` wired in as a graph dependency.
- **`rust-toolchain.toml`** + a pinned Bun version (`packageManager` / `.bun-version`) —
  so every checkout and CI runner is identical.

> **On Turbo:** Bun's built-in `bun run --filter` can fan tasks across workspaces, so the
> repo *can* start without Turbo and add it once the build graph grows. Turbo is kept in
> the plan for cross-language caching and the Rust→codegen→TS dependency graph, which
> Bun's filter alone doesn't model.
>
> **Alternatives considered.** `moonrepo`/`Bazel` give first-class polyglot graphs but add
> significant setup and cognitive cost; not worth it here. Bun + Cargo + Turbo is the
> lowest-ceremony path that still gives cached, incremental, cross-language builds.

---

## 3. Top-level layout

```
jose/
├─ README.md
├─ CONTRIBUTING.md            # how to add a context, naming rules, where files go
├─ Cargo.toml                 # Rust workspace root (members = crates/*)
├─ rust-toolchain.toml
├─ package.json               # Bun workspace root ("workspaces": apps/*, packages/*) + root dev tooling
├─ bun.lock                   # single JS/TS lockfile
├─ turbo.json                 # cross-language task pipeline (bunx turbo)
├─ .github/
│  ├─ workflows/              # CI: lint, test, codegen-drift check, build
│  └─ CODEOWNERS              # per-context ownership
│
├─ schema/                    # ⭐ SINGLE SOURCE OF TRUTH — the domain MODEL
│  ├─ model/                  # canonical machine contract (promoted unified-model.json)
│  ├─ registry/               # type-ownership + alias registry (from analysis/)
│  └─ README.md               # "edit here; everything downstream is generated"
│
├─ crates/                    # 🦀 the engine — Rust workspace (the "brain")
│  ├─ geometry-kernel/        # shared kernel: Plane, Transform, ticks, BREP primitives
│  ├─ reference-data/         # shared flyweights (MechanicalProperties, StockSpec, …)
│  ├─ materials/              # Materials & Stock context
│  ├─ building/               # Building Model + Placement context
│  ├─ loads-analysis/         # Loads & Analysis context
│  ├─ design-standard/        # Structural — DesignStandard Strategy seam + leaves
│  ├─ cut-optimization/       # Cut & Optimization context
│  ├─ estimating/             # Estimating & Cost context
│  ├─ drawings-export/        # Drawings Export context
│  ├─ project-context/        # Project Context (cross-cutting input)
│  ├─ bim-core/               # composition root: wires contexts into the 10-stage pipeline
│  └─ bim-wasm/               # wasm-bindgen boundary: Channel A (commands) + B (SoA buffers)
│
├─ packages/                  # 🌐 TS/JS workspace (the "hands & eyes" + shared TS)
│  ├─ model-types/            # GENERATED TS types + BufferLayout reader (from schema/)
│  ├─ render-mirror/          # read-only typed-array views over Rust SoA buffers
│  ├─ tool-runner/            # ToolRunner + drawing tools (snap, grammar, intents)
│  ├─ workspace-ui/           # drawing-workspace UI components
│  └─ persistence-client/     # typed client for the backend API
│
├─ apps/
│  ├─ web/                    # the browser app: main thread + worker bootstrap + canvas
│  └─ api/                    # backend (Bun runtime): persistence boundary (Neon/Drizzle, R2 blobs)
│
├─ tooling/
│  ├─ codegen/                # ⭐ MODEL → Rust structs + TS types + BufferLayout keystone
│  └─ scripts/                # repo maintenance, drift checks, release helpers
│
├─ tests/
│  ├─ e2e/                    # full-pipeline / browser end-to-end
│  └─ fixtures/               # shared sample models, golden files
│
└─ docs/                      # design docs (existing) + ADRs + plans
   ├─ schema/  analysis/  reference/    # current content, unchanged
   ├─ adr/                    # Architecture Decision Records
   └─ plans/                  # forward-looking plans (this file)
```

**`crates/` vs `packages/` vs `apps/`** is the load-bearing distinction:

- `crates/` = the **domain** and the engine. Pure, testable, framework-free. This is where
  DDD lives.
- `packages/` = **shared TS** consumed by apps (generated types, render mirror, tools).
- `apps/` = **deployable surfaces** (the web app, the backend). Thin shells that compose
  packages and crates; they hold wiring and config, not domain logic.

---

## 4. DDD mapping — 12 layers → bounded contexts

Each schema layer becomes a bounded context. The mapping is near one-to-one because the
schema was already designed with clean seams.

| Schema layer | Bounded context | Home | Kind |
|---|---|---|---|
| `geometry-kernel` | Geometry Kernel | `crates/geometry-kernel` | **Shared Kernel** |
| `reference-flyweights` | Reference Data | `crates/reference-data` | **Shared Kernel** (catalog) |
| `materials-stock` | Materials & Stock | `crates/materials` | Core domain |
| `building-placement` | Building & Placement | `crates/building` | Core domain |
| `loads-analysis` | Loads & Analysis | `crates/loads-analysis` | Core domain |
| `design-standard-seam` | Structural (Strategy seam) | `crates/design-standard` | Core domain |
| `cut-optimization` | Cut & Optimization | `crates/cut-optimization` | Supporting |
| `estimating-cost` | Estimating & Cost | `crates/estimating` | Supporting |
| `drawings-export` | Drawings Export | `crates/drawings-export` | Supporting |
| `project-context` | Project Context | `crates/project-context` | Generic/input |
| `workspace-render` | Drawing Workspace + Render | `packages/{render-mirror,tool-runner,workspace-ui}` | Interface/UI |
| `system-architecture` | Runtime & Persistence | `crates/bim-wasm`, `apps/api`, `tooling/codegen` | **Technical, not a domain** |

Notes that drive the boundaries:

- **`system-architecture` is not a domain context.** It describes *how the domain runs*
  (worker, channels, buffers, persistence), so it is deliberately *not* a domain crate. It
  becomes the wasm boundary (`bim-wasm`), the backend (`apps/api`), and the codegen tool.
  Keeping it separate is the same discipline that kept render out of `materials` (finding
  S2-A in the analysis).
- **Two shared kernels.** `geometry-kernel` (Plane, Transform, ticks) and `reference-data`
  (intrinsic flyweights) are depended on by many contexts but depend on nothing. They are
  the only crates allowed broad inbound edges.
- **The Strategy seam is a context boundary.** `design-standard` exposes a
  material-agnostic `DesignStandard` trait; `nds`, `aisi_cfs`, `aci_concrete` are leaf
  modules/features behind it. New materials = new leaf, **no core edits** — exactly the
  extensibility the schema promises.

### 4.1 Inside each domain crate (hexagonal)

Every bounded-context crate uses the same internal layout so the repo is predictable:

```
crates/materials/
├─ Cargo.toml
├─ src/
│  ├─ lib.rs           # public context API (the "facade")
│  ├─ domain/          # entities, value objects, invariants — PURE, no deps outward
│  ├─ application/     # use cases / services that orchestrate the domain
│  ├─ ports/           # traits the context needs from the outside (e.g. a repository)
│  └─ adapters/        # implementations: generated buffer layout, persistence, etc.
└─ tests/              # context-level integration tests
```

**The dependency rule (enforced, not aspirational):**
`adapters → ports → application → domain`, inward only. `domain/` imports nothing from
`application/`, `ports/`, or other contexts except the two shared kernels. Cross-context
talk goes through `lib.rs` facades, never into another context's `domain/`.

---

## 5. The codegen spine (the keystone)

This is the single most important tidiness mechanism in the repo, and it is *already
designed into the schema* (the `BufferLayout` "keystone contract").

```
schema/model/  ──►  tooling/codegen/  ──►  crates/*/src/adapters/generated.rs   (Rust)
                                      └─►  packages/model-types/src/*.ts         (TS types)
                                      └─►  packages/model-types/src/layout.ts    (BufferLayout)
```

Rules:

- **Edit the model, never the generated files.** Generated files carry a
  `// @generated — do not edit` header and live in clearly named `generated/` paths.
- **CI fails on drift.** A `codegen --check` job regenerates and diffs; any uncommitted
  delta fails the build. This is what makes "the Rust writer and JS reader provably cannot
  drift" a *mechanical guarantee* rather than a hope.
- **The model is versioned** (`meta.version`, currently `1.0.1`); generated artifacts
  inherit it so a buffer mismatch is caught at load, not at runtime corruption.

---

## 6. Naming & file-placement conventions

Keep the tree skimmable by making placement obvious:

- **Crates:** kebab-case domain nouns (`loads-analysis`), no `lib-`/`core-` prefixes except
  the composition root (`bim-core`) and boundary (`bim-wasm`).
- **TS packages:** scoped, kebab-case (`@jose/render-mirror`). Apps are unscoped names.
- **One concept, one home.** The type-ownership registry (`schema/registry/`) is the
  authority for *where each type lives*; cross-references point at the canonical home
  (this is the discipline that resolved the 11 collisions / 13 dangling refs in the audit).
- **Generated vs authored** are never mixed in one directory.
- **Tests live next to what they test** (`#[cfg(test)]` + `tests/` per crate; `*.test.ts`
  per package). Only cross-cutting/e2e tests live in top-level `tests/`.
- **No "utils" / "common" / "misc" dumping grounds.** Shared code is a named context
  (`geometry-kernel`, `reference-data`) or it doesn't exist.

---

## 7. Tidiness governance (how it stays clean)

| Mechanism | What it enforces |
|---|---|
| **Module-boundary lints** | Rust: crate graph forbids inward→outward and cross-`domain` edges (encoded via crate visibility + an arch test). TS: `dependency-cruiser` rules forbid `apps → apps`, deep imports into other packages' internals. |
| **`codegen --check` in CI** | The model is the source of truth; generated drift fails the build. |
| **`cargo-deny` + `pnpm audit`** | License/advisory/duplicate-dependency hygiene. |
| **CODEOWNERS per context** | Each `crates/<context>` and `packages/<pkg>` has an owner; changes route for review. |
| **ADRs (`docs/adr/`)** | Every structural decision (new context, boundary move, tool swap) is a numbered, immutable record. The schema's "resolved/open decisions" already model this habit. |
| **`CONTRIBUTING.md`** | One page: "to add a bounded context, do X; types go in the registry; generated files are off-limits." Makes the right thing the easy thing. |
| **Formatters pinned** | `rustfmt` + `prettier` + `eslint`/`clippy` run in CI and pre-commit; no style debates. |
| **No orphan top-level dirs** | Anything new at the root requires an ADR. The 9 top-level entries in §3 are the whole vocabulary. |

---

## 8. Phased rollout (no big bang)

The repo is docs-only today; the scaffold lands incrementally so `main` is always green.

1. **Skeleton + spine.** Workspace roots (`Cargo.toml`, `pnpm-workspace.yaml`, `turbo.json`),
   `schema/` promoted from `docs/schema/`, `tooling/codegen/` emitting types from the MODEL,
   and the `codegen --check` CI gate. *Nothing domain yet — just the rails.*
2. **Shared kernels.** `geometry-kernel` + `reference-data` crates, fully tested. They
   unblock everything else.
3. **Core contexts.** `materials` → `building` → `loads-analysis` → `design-standard`
   (the seam), in pipeline order, each behind its facade.
4. **Boundary + frontend.** `bim-wasm` + `packages/{model-types,render-mirror,tool-runner}`
   + `apps/web` — first end-to-end slice (draw → recompute → render).
5. **Supporting + backend.** `cut-optimization`, `estimating`, `drawings-export`,
   `apps/api` (Neon/Drizzle, R2).

Existing docs stay where they are; `docs/schema/` content is *promoted* (copied, with a
pointer left behind) into `schema/` so the machine contract becomes a real build input
rather than a reference document.

---

## 9. Decisions (resolved)

1. **Monorepo tool** — **Bun + Cargo + Turborepo** (§2). Bun is package manager, runtime,
   and test runner for TS; Cargo for Rust; Turbo for cross-language caching/task graph.
2. **Backend in this repo** — **yes**, `apps/api` lives here on the Bun runtime
   (Neon/Drizzle + Cloudflare R2). Hono keeps Bun/Workers deploy targets open.
3. **Top-level split** — **split layout**: `crates/` (Rust engine), `packages/` (shared
   TS), `apps/` (deployables). Clearer than a unified `packages/` for a polyglot repo and
   idiomatic to each toolchain (Cargo crates; Bun globs `packages/*`).

---

*This document is a plan only. No workspace files, crates, or packages are created by it.*
