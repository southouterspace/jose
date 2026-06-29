# ADR 0011 — Advisory UI review in CI: deterministic gates vs. agent judgment

- **Status:** Accepted
- **Date:** 2026-06-29
- **Context doc:** builds on [ADR 0010](./0010-product-design-skill.md) (the `product-design` skill)
  and the existing CI ([`.github/workflows/ci.yml`](../../.github/workflows/ci.yml))

## Context

[ADR 0010](./0010-product-design-skill.md) split product-design enforcement into two legs:
deterministic checks for clear rules, agent guidance for judgment. CI today is entirely the first
leg — Biome/Ultracite, `tsc`, `codegen:check`, clippy — and every job is a **required** gate.

We want to fold a **Web Interface Guidelines reviewer** (keyboard, focus, animation, typography,
touch, copy — a port of Vercel's list, scoped to Jose) into CI. That reviewer is LLM-driven: its
output varies run to run and carries false positives. Wiring it in naively — as a required gate —
would make `main` hostage to model variance and noise. But leaving it out means the judgment leg
never runs on a PR at all.

The codebase reality bounds the problem: Jose's web surface is two `<canvas>`/SVG viewports + a
toolbar + a status bar, dark-only, client-rendered (Vite SPA). Most of the guideline list (forms,
images, i18n, SSR/hydration, virtualized lists, Tailwind) has **no surface here**, and a large slice
of the accessibility/anti-pattern rules is **already enforced** by the Ultracite/Biome preset that
CI already requires.

## Decision

1. **Three tiers, matched to where enforcement belongs.**
   - **Required deterministic gate — `ci.yml` (unchanged).** Biome/Ultracite already gates the
     mechanically-checkable accessibility and anti-pattern rules. We do not duplicate those in prose
     or in the reviewer.
   - **Advisory agent review — a new, separate workflow
     ([`ui-review.yml`](../../.github/workflows/ui-review.yml)).** Runs the
     `/web-interface-guidelines` reviewer on changed web files and posts findings as a single PR
     comment. It is `continue-on-error`, path-filtered, read-only, and **never a required check** —
     a finding (or a misconfiguration) cannot fail a PR.
   - **Offline evals — `tooling/scripts/evals/` (unchanged).** Validate the reviewer itself; they
     run on the skill, not per PR.

2. **One source of truth for the rules.** The rule list lives once, at
   [`.agents/skills/product-design/references/web-interface-guidelines.md`](../../.agents/skills/product-design/references/web-interface-guidelines.md).
   The slash command and the CI job both read it; neither carries its own copy — the same
   DRY-via-one-source discipline as the MODEL.

3. **Enforcement tiers are recorded per rule.** Each rule is tagged `[biome]` (already gated),
   `[agent]` (judgment), `[gap]` (a reliable deterministic check not yet built), or `[n/a]` (no
   surface in Jose). The reviewer reports only `[agent]` and `[gap]`; `[biome]` and `[n/a]` are
   suppressed as noise.

4. **Deterministic-subset migration is deferred, not skipped.** Several `[gap]` rules (`outline:
   none` without a focus replacement, `transition: all`, literal `...`, `user-scalable=no`) are
   reliable custom-check candidates. But Jose's current web code has near-zero surface for them
   (no transitions in `app.css`, a clean viewport meta), so authoring custom Biome/GritQL plugins
   now would be speculative and barely testable. They stay tagged `[gap]` and tracked in
   `coverage-gaps.md`; a plugin is written when real surface and a real failure exist (YAGNI).

## Consequences

- A new advisory workflow exists; it requires an `ANTHROPIC_API_KEY` repo secret and **skips cleanly
  when absent** (a guard step), so the workflow is safe to merge before the secret is configured.
- `ci.yml` and its required gates are unchanged; merge-blocking behavior is identical to before.
- The reviewer and the skill cannot drift on rules — they share one file.
- The advisory comment is idempotent (updated in place via a marker), so re-runs don't pile up.
- When a `[gap]` rule gains real surface, the path is defined: write the deterministic check, flip
  the tag to `[biome]`, and the reviewer stops reporting it.

## Alternatives considered

- **Make the reviewer a required gate.** Rejected: LLM variance and false positives would block
  merges on noise; an agent reviewer is advice, not a deterministic gate.
- **Run the full guideline list unscoped.** Rejected: most rules target forms/images/i18n/SSR that
  Jose doesn't have; the output would be noise. Tiering + `[n/a]` suppression keeps signal high.
- **Inline the rules into the slash command (Vercel's posted form).** Rejected here: it would create
  a second copy of the rules alongside the skill reference. We keep one source and point both at it.
- **Author the custom Biome plugins now.** Deferred: no real surface to check yet; speculative rules
  with false-positive risk violate YAGNI. Tracked as coverage gaps instead.
