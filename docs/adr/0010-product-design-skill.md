# ADR 0010 — The `product-design` skill: design decisions as routed, versioned context

- **Status:** Accepted
- **Date:** 2026-06-29
- **Context doc:** [`CONTEXT-MAP.md`](../../CONTEXT-MAP.md) (the **web** context); builds on
  [`apps/web/CONTEXT.md`](../../apps/web/CONTEXT.md), [ADR 0005](./0005-frontend-application-stack-react-vite.md),
  [ADR 0007](./0007-space-first-modeling-footprint-push-pull.md), and
  [ADR 0008](./0008-mvp-geometry-and-command-contract.md). Inspired by Vercel's `product-design` system.

## Context

The repo already treats engineering decisions as code: ADRs record structural decisions with
their evidence and rejected alternatives, `CONTEXT.md` files pin each context's ubiquitous
language, and a wall of deterministic checks (`codegen:check`, Biome/Ultracite, `clippy -D
warnings`) makes "you can't drift" mechanical. That machinery governs the **engine** well.

It does **not** govern the **product surface**. The drawing UX (`apps/web` — the app shell, the
plan view, the 3D view) is the one place a *user* sees, understands, chooses, and acts. The
decisions that shape it — both panes are input surfaces (ADR 0007); units are ticks but the UI
displays feet/inches (drawing-ux-mvp plan); Push/Pull is disabled until a mass exists
(`app.tsx`); the in-progress footprint is transient client state, never canonical (ADR 0008) —
live scattered across ADRs, code comments, and one engineer's head. A coding agent (or a new
contributor) editing the plan view has no single place that tells it *why* those choices are
correct, and nothing routes it to the canonical source at the moment of the edit.

This is exactly the gap Vercel describes in "Teaching agents product design": code shows agents
what shipped, not why it became the standard. The fix is a skill that delivers the reasoning
behind product decisions, backed by deterministic checks where a rule can be checked reliably.

## Decision

1. **Add a `product-design` skill** at `.agents/skills/product-design/`, structured as Vercel
   documents it: a skill-local `AGENTS.md` (load order, validation, governance), a `SKILL.md`
   (the runtime workflow and request modes), a `references/` directory (product judgment,
   interface quality, resilience, copy, per-surface decisions, rules, glossary, patterns,
   coverage gaps), and an `exemplars/` directory (decisions worth repeating, drawn from shipped
   PRs).

2. **Add a repo-level `AGENTS.md`** as the trigger: it tells coding agents *when* to load the
   skill and what is in/out of scope. This is the one new top-level file; it complements, and does
   not replace, `CLAUDE.md` (which governs the engine and the build).

3. **Route, never duplicate.** The skill is an index over Jose's existing canonical sources, not a
   second copy of them. Product vocabulary stays owned by [`apps/web/CONTEXT.md`](../../apps/web/CONTEXT.md);
   structural product decisions stay owned by the ADRs; the skill's `glossary.md` and `surfaces-*.md`
   files *point* to those owners. When the two disagree, the canonical owner wins and the skill is
   the thing that's stale.

4. **Three legs, matched to where they belong.** Clear, mechanically-checkable rules go to
   deterministic checks (Biome/Ultracite today; a custom check only when it earns its keep).
   Decisions that need product or codebase judgment go to the skill's references. New standards and
   unresolved product choices stay with people — the eval/intake structure under
   `tooling/scripts/evals/` ends at a review packet, never an automatic rule.

5. **Scope honestly to the evidence we have.** The skill is populated for the surface that exists
   and has shipped decisions — the space-first drawing UX. Where Jose has no standard yet (theming,
   error/permission states, responsive behavior, real accessibility passes), the skill records a
   **coverage gap** rather than inventing taste. Per YAGNI, reference files for absent surfaces stay
   lean stubs until evidence justifies filling them.

## Consequences

- A new top-level `AGENTS.md` and a new top-level `.agents/` directory exist. Both are
  documentation/governance, not source: like `docs/**`, they are excluded from Biome. Eval
  *fixtures* (intentionally-flawed "before" UI) are also excluded so the linter doesn't try to fix
  the very mistakes they encode.
- The skill is **additive and inert until invoked** — it changes no engine code, no build step, and
  no CI gate. It can be deleted with zero blast radius.
- Adding or changing a rule now has a defined home and a bar: current-source verification plus human
  acceptance, recorded with scope, rationale, evidence, exceptions, and a bad/good example
  (`AGENTS.md` governance section). This keeps the skill from rotting into folklore.
- The drawing UX gains a place to record design decisions as it grows past the MVP (selection,
  inspector panels, the elevation view), so the next surface lands with its reasoning attached.

## Alternatives considered

- **Fold product-design guidance into `CLAUDE.md`.** Rejected: `CLAUDE.md` is a single flat brief an
  agent reads whole; it has no mode-routing and no per-surface loading, and mixing engine-boundary
  rules with UI judgment dilutes both. A routed skill loads only what the task needs.
- **Start with an architecture/boundary skill instead of the UI.** A strong option (the DDD boundary
  rules are Jose's most-repeated judgment calls), but the user's intent here is the product surface,
  and the drawing UX already carries shipped decisions to encode. The boundary skill remains an open
  follow-up, not foreclosed by this ADR.
- **Lint-only, no skill.** Rejected: most product decisions (naming an action's consequence,
  choosing a surface's persistence, what a state should say) require context a linter cannot see.
  Deterministic checks handle the mechanical subset; they cannot carry the judgment.
- **Wait for a design system before writing any of this.** Rejected: the absence of a design system
  is itself a recorded coverage gap. The skill captures the decisions we *have* now and grows with
  the product.
