# ADR 0004 — The supporting contexts and the persistence boundary

- **Status:** Accepted
- **Date:** 2026-06-28
- **Context doc:** [`docs/plans/repo-scaffold.md`](../plans/repo-scaffold.md) §1, §4, §8 (Phase 5)

## Context

Phase 5 lands the last three domain contexts and the backend: `cut-optimization`, `estimating`,
`drawings-export`, and `apps/api`. They sit *downstream* of the core — cut and estimating are the
bottom-up cost path, drawings-export is the terminal output stage, and the API persists the
result. Four structural questions had to be settled, each of which recurs as these contexts grow.

1. **Where the cut solver reads SKU facts.** The schema is emphatic that `StockOption` /
   `Offcut` carry **no** intrinsic SKU data — length, pack, and price live on the materials
   `SupplierSku` / `PriceQuote` flyweights and are read *by reference* (the S6 flyweight fix). A
   runnable solver still needs those numbers.
2. **How a material stays out of the cut and cost code.** The schema promises that adding
   cold-formed steel, rebar, concrete, or masonry is *data, never a new branch* — gated only by
   `stockForm` behind the `DesignStandard` seam. The supporting contexts must encode that promise
   mechanically, not aspirationally.
3. **How the cost layer stays open to new cost databases.** Estimating must validate bottom-up
   against an external top-down benchmark (RSMeans / ENR / historical) without the core depending
   on any one of them.
4. **How persistence stays orthogonal to the domain.** The plan is explicit that Neon/Drizzle
   snapshots and R2 blobs are *adapters at the edge*; the domain crates must not learn about them.

## Decision

1. **The cut solver reads SKU facts through a `StockCatalog` port.** `cut-optimization` defines a
   `StockCatalog` trait (`stock_length` / `pack_size` / `unit_price` by `SkuKey`); the
   `CuttingStockSolver` depends on the trait, never on a concrete catalog, so `StockOption` and
   `Offcut` stay pure references and the flyweight is never copied. The composition root backs the
   port with the materials catalog; tests back it with a small in-memory map. The solver itself is
   **first-fit-decreasing + offcut pool** (`ffdPlusPool`): cuts placed longest-first into the
   tightest open stick, then a pooled offcut, then a fresh bought stick — material-blind
   throughout.

2. **`stockForm` is a typed gate, not a branch.** `CutEligibility::classify(Form)` returns the
   verdict (`linear` ⇒ eligible; `sheet`/`cast`/`unit` ⇒ routed away to nest / formwork / direct),
   and `solve` *requires* it: an ineligible batch produces an empty, routed-away plan. The
   estimating side mirrors this — a `MaterialLine` carries a `StockFormPath` discriminator selecting
   the pricing path (linear → exact cut-list waste; sheet → nest; cast → volume; unit → count) and
   `CostType` is a closed, material-blind economic enum. Neither solver nor cost line ever names a
   species. Adding a material is a new `stockForm` classification upstream, never an edit here.

3. **`CostBenchmark` is the cost-side Strategy seam.** Estimating exposes a `CostBenchmark` port —
   the structural twin of `design-standard`'s `DesignStandard` interface — that returns an
   independent top-down cost and its variance against the bottom-up rollup. `RsMeansBenchmark` is
   one leaf; ENR / supplier-index / historical-job plug in beside it with zero core edits. The
   bottom-up half (`CostRollup`) applies the markup/allowance stack **deterministically**: each
   `Markup` declares its own `AppliesToBase`, so markup-on-markup (OH → profit → bond → tax) is
   reproducible data; drawn allowances enter the markup base, undrawn remainder is carried.

4. **`apps/api` is a domain-orthogonal snapshot boundary.** The backend is a thin Hono app
   (Bun runtime, Workers-compatible) over two ports — `SnapshotStore` and `BlobStore` — with
   in-memory adapters by default and Drizzle/Neon + R2 behind the same interfaces in production.
   It round-trips an **opaque** `payload` wrapped in a `SnapshotEnvelope` stamped with the
   `MODEL_VERSION` and the `LAYOUT_HASH` keystone (from `@jose/model-types`); a snapshot from a
   different `BufferLayout` is rejected at load with **409 stale layout**, extending the keystone's
   "cannot drift" guarantee to the persistence edge. No domain crate depends on `apps/api`.

## Consequences

- The three crates land in the Cargo workspace, so the full Rust CI (`fmt` + `clippy -D warnings`
  + `test`) covers them, and the `wasm32` build of `bim-wasm` is unaffected (they are not on the
  wasm boundary's dependency path).
- `cut-optimization → estimating` is a real crate edge: `TakeoffBuilder` walks a `CutPlan` into
  traceable `TakeoffItem`s, so the bottom-up chain `Stock/Offcut → CutAssignment → PieceProvenance
  → materials::Piece → TakeoffItem → MaterialLine` compiles end-to-end. Drawings-export depends
  only on `geometry-kernel` + `reference-data` (it is a parallel terminal consumer, never a
  dependency of the cost path).
- `apps/api` adds `hono` + `drizzle-orm` to the Bun lockfile. The Drizzle table definitions are
  pure schema (no driver), so they typecheck and migrate without a live database; the runtime
  store is in-memory until `DATABASE_URL` is set. CI's frozen-lockfile install + `typecheck` +
  the codegen drift gate all stay green.
- These contexts are **not** wired into `bim-core`'s hot `Session` path. Like loads-analysis and
  the design-standard check (Phase 3/4), they are composed *when* the cost/export/persistence
  pipeline runs, not on every draw recompute — keeping the draw → render slice lean.

## Alternatives considered

- **Copying `stock_length` onto `StockOption` for solver convenience.** Rejected: it reintroduces
  the S6 flyweight duplication the schema explicitly removed. The `StockCatalog` port keeps the
  number single-sourced on the SKU at the cost of one trait call.
- **A single overloaded `CostLine` array carrying material *and* labor.** Rejected per the
  schema's own correction: material prices off a `SupplierSku`/`PriceQuote`, labor off a
  `ResourceRate` with contextual burden/region overrides on the line. Separate `MaterialLine` /
  `ResourceLine` / `SubcontractLine` keep the two flyweight families from drifting.
- **Bundling a real cost database (RSMeans rows) into the estimating crate.** Rejected: it would
  couple the core to one source. The `CostBenchmark` seam + an injected unit-cost table exercises
  the variance math while leaving the database an adapter concern.
- **Persisting typed domain rows (a column per `Estimate` field) via Drizzle.** Rejected for the
  boundary: it would make persistence track every domain schema change and re-couple the edge to
  the model. A version-stamped `jsonb` snapshot keeps persistence orthogonal; the `LAYOUT_HASH`
  guard catches the staleness a typed schema would have caught at migration time.
