# ADR 0002 — Core-context crate layout and dependency direction

- **Status:** Accepted
- **Date:** 2026-06-28
- **Context doc:** [`docs/plans/repo-scaffold.md`](../plans/repo-scaffold.md) §4, §8 (Phase 3)

## Context

Phase 3 lands the four core-domain contexts in pipeline order — `materials` → `building`
→ `loads-analysis` → `design-standard` (the strategy seam). Two structural questions had
to be settled while implementing them, and both are worth recording because they will
recur for every future context.

1. **Internal layout.** The scaffold plan (§4.1) prescribes a hexagonal
   `domain/ application/ ports/ adapters/` layout for core crates, while noting the shared
   kernels skip `ports/`/`adapters/` because empty layers are noise.
2. **Dependency direction.** The MODEL couples `loads-analysis` and `design-standard` in
   **both** directions: loads references the connection graph, the ASD/LRFD philosophy, and
   the strategy (all single-homed in design-standard); design-standard consumes
   `MemberDemand` and `LoadCombination` from loads. A Cargo crate graph cannot contain a
   cycle, so this must be broken.

## Decision

1. **Layout is hexagonal but only as deep as the context warrants.** Each crate exposes its
   bounded context through a `lib.rs` facade and organizes code by the layers that actually
   exist:
   - `materials` and `building` carry domain types (and, for `building`, an `application/`
     service — the `FramingSolver`). They have no outward I/O at this phase, so they have no
     `ports/`/`adapters/` — the same "don't fabricate empty layers" rule the kernels follow.
   - `loads-analysis` splits `domain/` (sources, tributary, combination, demand) from
     `application/` (the path/rollup/solver services).
   - `design-standard` is the first context where `ports/`/`adapters/` are *real*: the
     `DesignStandard` Strategy trait is a **port** the application core (`SizingArbiter`)
     depends on, and each material leaf (`NdsWood`, `AisiCfs`, …) is an **adapter** that
     implements it. This is the textbook hexagonal seam, so it gets the full structure.

2. **The pipeline order is the dependency order; downstream cross-references become opaque
   keys.** `loads-analysis` must not depend on `design-standard` (loads is upstream). The
   loads → design-standard references in the MODEL are therefore modeled as opaque key
   newtypes defined inside `loads-analysis` (`ConnectionGraphRef`, `DesignPhilosophyRef`,
   `DesignStandardRef`) — exactly the "reference by key" idiom the schema already uses for
   every cross-layer link. `design-standard` is the downstream crate and depends on
   `loads-analysis` by value (`MemberDemand`, `LoadCombination`). The crate graph stays
   acyclic:

   ```
   geometry-kernel ─┐
   reference-data ──┼─→ materials ─→ building ─→ loads-analysis ─→ design-standard
                    └──────────────────────────────────────────┘
   ```

   The `LoadSolver` likewise applies a generic, material-blind governing-combination rule
   and records which combo it chose; the schema delegates that pick to the strategy, which a
   downstream caller can override — but the loads crate never reaches forward to do it.

## Consequences

- Adding a new material is adding a leaf in `design-standard/src/adapters/` that implements
  `DesignStandard`; the arbiter and the upstream crates are untouched — the forward
  extensibility requirement is mechanical, and `cargo check` enforces it.
- The base-unit invariant is encoded in the types: linear quantities are `Tick`s; area /
  volume / money / engineering demand are derived reals. No area is typed tick².
- Future contexts follow the same two rules: facade + only-the-layers-you-need internally,
  and dependencies point in pipeline order with downstream links held by key.

## Alternatives considered

- **A shared "structural-kernel" crate** holding the types both loads and design-standard
  reference, to break the cycle by extraction. Rejected: the coupled types (connection
  graph, philosophy) are genuinely design-standard domain concepts; hoisting them would blur
  a bounded-context boundary to satisfy the build graph. Opaque keys keep the boundary intact.
- **Full `ports/`/`adapters/` in every crate.** Rejected as the noise the plan itself warns
  against — `materials`/`building` have no outward I/O to adapt at this phase.
