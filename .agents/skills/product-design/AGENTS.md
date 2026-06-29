# product-design — load order, validation, governance

This file governs the skill itself: how its files load, how a change is validated, and the bar a
new rule must clear. `SKILL.md` owns the runtime workflow; this file owns the skill's integrity.
Read it before *changing the skill*; read `SKILL.md` before *doing product work*.

## Load order

1. **`SKILL.md`** — always. Resolves the request mode and routes the rest.
2. **The repo + surface `CONTEXT.md`/`AGENTS.md` chain** — the canonical owners the skill points to.
   Never load a skill reference in place of its owner; load the owner and let the reference index it.
3. **`references/` — only what the task needs**, per the routing table in `SKILL.md` §5. A copy pass
   loads `copy.md`; a 3D-view change loads `surfaces-3d-view.md`; a material flow change loads
   `product-judgment.md` first. Do not preload the whole directory.
4. **`exemplars/` — when an analogous decision already shipped.** Precedent, not law: an exemplar
   shows a decision that was made and why, plus the mistakes to avoid. Verify it still matches the
   current code before leaning on it.

## Canonical owners (the skill routes here; it does not duplicate them)

| Domain | Canonical owner |
| ------ | --------------- |
| Product vocabulary (the ubiquitous language of the drawing UX) | [`apps/web/CONTEXT.md`](../../../apps/web/CONTEXT.md) |
| Structural product decisions (what the front door is, what's deferred) | the ADRs under [`docs/adr/`](../../../docs/adr/) |
| The MVP scope and the engineering calls behind it | [`docs/plans/drawing-ux-mvp.md`](../../../docs/plans/drawing-ux-mvp.md) |
| The geometry/command contract (buffers, `DrawFootprint`/`PushPull`) | [ADR 0008](../../../docs/adr/0008-mvp-geometry-and-command-contract.md) |
| Engine/boundary rules, build, lint config | [`CLAUDE.md`](../../../CLAUDE.md), [`biome.jsonc`](../../../biome.jsonc) |

If a `references/` file and its owner disagree, **the owner is right and the reference is stale** —
fix the reference, don't fork the decision.

## Validation — before you trust the skill on a change

1. Confirm the request mode and the target surface are named (see `SKILL.md` §1).
2. Confirm every cited rule resolves to a current canonical source — not a stale comment or a single
   shipped file. Shipped code is *evidence*, not automatic precedent (`SKILL.md` operating contract).
3. Run the repo's deterministic checks for any code you touched: `bun run lint`, `bun run typecheck`,
   and `bun test` for the affected package. The skill never overrides a green-CI requirement.
4. For any *visual* claim, verify against a rendered surface, not source alone. Source establishes
   behavior; only a rendered viewport establishes visual and interaction quality.

## Governance — changing the skill

Treat a skill change like a product change. The bar:

- **Add or change a rule only after current-source verification and human acceptance.** Never promote
  one screenshot, one shipped file, or one reviewer comment into a universal rule by itself.
- **Record, for every rule:** scope, rationale, evidence (a link to the source, PR, or ADR),
  exceptions, and a bad/good example. The template lives in `references/rules.md`.
- **Prefer the narrowest destination.** In order: a canonical source (ADR, `CONTEXT.md`), a routed
  reference, an exemplar, a deterministic check, or a coverage gap. Put a clear, mechanically-checkable
  rule in a linter — not in prose — and keep judgment in prose with its evidence.
- **A new standard or unresolved product choice stays with people.** The intake structure under
  [`tooling/scripts/evals/`](../../../tooling/scripts/evals/) ends at a review packet; a human decides
  whether a candidate becomes a rule, a reference, an exemplar, a check, an eval, or no change.
- **Removal is maintenance.** A rule that needs many exceptions to stay true moves back to judgment;
  a rule that stops helping is deleted. Record the change and its reason.

## What this skill does *not* do

- It does not evaluate or modify engine/domain code, the MODEL, generated files, persistence, or the
  build. Those are `CLAUDE.md`'s. The skill stops at the user-facing surface.
- The eval harness tests **copy and interface-language behavior and the per-surface rules** — it does
  not score the broader product-design workflow or the engine.
