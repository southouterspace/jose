# ADR 0011 — Local in-session UI/React review: deterministic gates vs. agent judgment

- **Status:** Accepted
- **Date:** 2026-06-29
- **Context doc:** builds on [ADR 0010](./0010-product-design-skill.md) (the `product-design` skill)
  and the existing CI ([`.github/workflows/ci.yml`](../../.github/workflows/ci.yml))

## Context

[ADR 0010](./0010-product-design-skill.md) split product-design enforcement into two legs:
deterministic checks for clear rules, agent guidance for judgment. CI today is entirely the first
leg — Biome/Ultracite, `tsc`, `codegen:check`, clippy — and every job is a **required** gate.

We want to add an agent-judgment review layer for user-facing web code: a **Web Interface
Guidelines** reviewer (keyboard, focus, animation, typography, touch, copy — ported from Vercel and
scoped to Jose) and a vendored **React best-practices** reviewer (re-render / JS-perf hygiene). Both
are LLM-driven: output varies and carries false positives.

The first instinct was to run them as a CI job. But running Claude **headlessly on GitHub's servers
requires an `ANTHROPIC_API_KEY` secret** — there's no authenticated human in an Actions runner. The
maintainer does not want to manage that secret. The key insight that resolves it: **a Claude Code
session is already authenticated.** Run the reviewers *in-session* — locally, where the work happens
— and the key requirement disappears entirely. The deterministic gates, meanwhile, are just the
shell commands in `ci.yml`; they run locally too.

Codebase reality bounds the rule sets. Jose's web surface is two `<canvas>`/SVG viewports + a
toolbar + a status bar, dark-only, **React + Vite client SPA** (no Next.js, no RSC/SSR, no data
fetching, no forms/images/i18n). Most guideline/React-perf rules have **no surface here**, and a
large slice of the accessibility/anti-pattern rules is **already enforced** by the required Biome
gate.

## Decision

1. **Three tiers, matched to where enforcement belongs.**
   - **Required deterministic gate — `ci.yml` (unchanged).** Biome/Ultracite, `tsc`, `codegen:check`,
     clippy. The mechanically-checkable accessibility/anti-pattern rules already live here; we do not
     duplicate them in prose or in a reviewer.
   - **Local in-session agent review — no CI job, no API key.** The `/web-interface-guidelines` and
     `/react-best-practices` commands run inside a Claude session against changed web files. A
     `/ci-local` command orchestrates the whole thing (deterministic gates + both reviews) so
     "run CI locally" is one invocation. `bun run ci:local` runs the deterministic JS/TS spine on
     its own.
   - **Offline evals — `tooling/scripts/evals/` (unchanged).** Validate the reviewers themselves;
     run on the skill, not per change.

2. **No `ANTHROPIC_API_KEY`, no Actions reviewer job.** The previously-considered
   `.github/workflows/ui-review.yml` is **dropped**. The judgment leg runs in-session under the
   maintainer's existing Claude auth.

3. **One source of truth per rule set.** The Web Interface Guidelines rules live once at
   [`.agents/skills/product-design/references/web-interface-guidelines.md`](../../.agents/skills/product-design/references/web-interface-guidelines.md);
   the React rules are **vendored** (MIT) at
   [`.agents/skills/vercel-react-best-practices/`](../../.agents/skills/vercel-react-best-practices/).
   The slash commands read those files; they don't carry their own copy (DRY).

4. **Enforcement tiers / scope recorded per rule.** Web rules are tagged `[biome]` (already gated),
   `[agent]` (judgment), `[gap]` (reliable check not built), `[n/a]` (no surface in Jose). The React
   skill is scoped by `jose-scope.md` (Vite SPA + Three.js → most `server-*`/`async-*`/hydration
   rules are N/A). Reviewers report only the applicable, judgment rows.

5. **Deterministic-subset migration is deferred, not skipped.** Several `[gap]` rules (`outline:
   none` without a focus replacement, `transition: all`, literal `...`) are reliable custom-check
   candidates, but Jose's web code has near-zero surface for them today. Authoring custom
   Biome/GritQL plugins now would be speculative and barely testable — deferred (YAGNI), tracked in
   `coverage-gaps.md`.

## Consequences

- **No repo secret, no new Actions workflow.** `ci.yml` and its required gates are unchanged; merge
  behavior is identical to before. Nothing to configure.
- **The reviews are as reliable as the habit of running them.** A local in-session review runs when
  the maintainer (or an agent) invokes `/ci-local` (or the skill auto-routes on a web edit). There is
  **no automatic gate on every PR** — the tradeoff for needing no key/secret. The repo `AGENTS.md`
  records the habit: run `/ci-local` before pushing web changes. This matches a Claude-driven
  workflow, where an agent is in the loop anyway.
- Reviewers and rule sources can't drift — each shares one file. The React skill is vendored with
  attribution and a pinned source, so it can be refreshed deliberately.

## Alternatives considered

- **Advisory CI workflow with `ANTHROPIC_API_KEY`** (the prior draft of this ADR). Rejected: the
  maintainer doesn't want to manage the secret, and a session already provides auth. Trading an
  always-on CI gate for a key-free in-session review is the accepted tradeoff.
- **Make either reviewer a required gate.** Rejected: LLM variance/false positives would block merges
  on noise; agent review is advice, not a deterministic gate.
- **Run the full rule lists unscoped.** Rejected: most rules target forms/images/i18n/SSR/Next.js
  that Jose doesn't have; the output would be noise. Tiering + scope keep signal high.
- **Author the custom Biome plugins now.** Deferred: no real surface to check yet (YAGNI).
