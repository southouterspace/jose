# ADR 0003 — The wasm boundary and the BufferLayout keystone

- **Status:** Accepted
- **Date:** 2026-06-28
- **Context doc:** [`docs/plans/repo-scaffold.md`](../plans/repo-scaffold.md) §3, §5, §8 (Phase 4)

## Context

Phase 4 lands the first end-to-end **draw → recompute → render** slice: the composition
root (`bim-core`), the wasm boundary (`bim-wasm`), the JS reader/tool packages
(`render-mirror`, `tool-runner`), and the browser app (`apps/web`). Three structural
questions had to be settled, each of which recurs for every buffer and every boundary
method added later.

1. **Where the contexts are composed.** The domain crates are pure and acyclic, but
   *something* has to wire them into a running pipeline and own the canonical state. The
   plan's `system-architecture` mapping says this is *technical, not a domain*.
2. **How the Rust writer and the JS reader stay in sync.** The schema's keystone contract
   (`BufferLayout`) promises the SoA byte offsets are *generated from the one MODEL* so the
   two sides "provably cannot drift". Phase 4 is where that generated Rust first exists, so
   the mechanism has to be made real, not aspirational.
3. **Unsafe at the FFI seam.** The workspace denies `unsafe_code`, but `wasm-bindgen`'s macro
   expands to `unsafe` glue. The deny lint and the boundary crate are in tension.

## Decision

1. **`bim-core` is the composition root and owns the canonical buffer.** It is a crate but
   *not a bounded context* — it defines no new domain types. It holds the `Session` (the
   in-session system of record), translates a `Command` into a context pipeline
   (`promote wall → FramingSolver → write SoA`), and owns the `MemberBuffer`. Loads-analysis
   and the design-standard check slot in as later `Session` stages without changing the
   boundary or the buffer contract. `bim-wasm` is a thin marshaling shell over it.

2. **One codegen run emits both sides of the `BufferLayout`; CI gates the drift.** The SoA
   columns are declared once in [`schema/model/buffer-layouts.json`](../../schema/model/buffer-layouts.json).
   `tooling/codegen` computes the byte offsets (pure column-major: each column is `CAPACITY`
   contiguous elements at a generated offset, so a JS typed-array view is zero-copy over one
   column) and emits the **identical** table to two places:
   - Rust writer — `crates/bim-core/src/generated/layout.rs`
   - JS reader — `packages/model-types/src/generated/layout.ts`

   Both carry the same `LAYOUT_HASH` (an FNV-1a digest of the computed offsets). The engine
   reports its hash; the render mirror `assertLayout`s it at startup, so a stale build fails
   loudly instead of corrupting reads. `bun run codegen:check` fails the build on any
   uncommitted delta — the "cannot drift" guarantee is mechanical, across two languages.

3. **`bim-wasm` is the one crate that opts out of `unsafe_code = "deny"`.** It sets its own
   `[lints]` (`unsafe_code = "allow"`, keeping clippy and the rest) rather than inheriting the
   workspace lints. This is the single FFI seam — the same discipline that keeps render out of
   the domain keeps `unsafe` out of every crate *except* the boundary. CI additionally builds
   it for `wasm32-unknown-unknown` so the boundary is proven against its real target.

## Consequences

- A new SoA buffer or column is a row in `buffer-layouts.json` + `bun run codegen`; the Rust
  offsets, the TS offsets, and the hash all move together or CI is red. Hand-editing either
  generated file is caught by the drift check.
- The generated `layout.rs` is formatting-exempt for its one wrapped array via an item-level
  `#[rustfmt::skip]` (module-level `#![rustfmt::skip]` is nightly-only), so codegen's output
  is authoritative and `cargo fmt --check` stays green.
- `apps/web` typechecks before the wasm artifact exists: `src/wasm/engine.d.ts` is the
  hand-kept ambient contract mirroring the `#[wasm_bindgen]` surface; the generated `pkg/` is
  a git-ignored build output. The main/worker split is two tsconfigs (DOM vs WebWorker libs).
- The first slice ships a per-recompute **snapshot copy** of the buffer, not the zero-copy
  `SharedArrayBuffer` view. The reader code is identical either way; the zero-copy path is
  gated on cross-origin isolation and deferred to Phase 5 with persistence.

## Alternatives considered

- **Interleaved (array-of-structs) buffer layout** so column offsets are capacity-independent.
  Rejected: it makes a single column a *strided* read, which a plain `Int32Array` view cannot
  express — the schema's `TypedArrayView` (one contiguous typed array per column) wants pure
  column-major. The cost is that a grow past `CAPACITY` is a realloc, which is the expected
  SoA grow seam anyway.
- **Deriving the SoA columns directly from the rich `MemberPlacement` MODEL fields.** Rejected
  for the slice: those fields (`bracing[]`, `ends[2]`, an `Orientation` frame) don't map to
  flat scalar columns. A small explicit render-buffer spec is the honest "the domain declares
  its fields" made flat, and keeps the generated contract legible.
- **Gating the whole `#[wasm_bindgen]` surface behind `#[cfg(target_arch = "wasm32")]`** to
  avoid host `unsafe`. Rejected: it would drop the boundary from host `cargo check`/clippy and
  the host smoke test. Allowing `unsafe` in just this crate keeps full coverage on both targets.
