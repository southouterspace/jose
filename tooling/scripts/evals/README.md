# product-design evals

Tests whether the `product-design` skill actually changes an agent's behavior on UI it hasn't seen.
Lint rules are deterministic; agent behavior varies — so we test the skill on fixtures and score the
result against a rubric.

> **Status: scaffolding.** The fixtures, the rules checklist, and the loop below are real and usable
> by hand (or by an agent), but there is **no runner wired into CI and no automated judge yet** —
> that's a recorded coverage gap (`.agents/skills/product-design/references/coverage-gaps.md`). This
> directory is a documented starting point, the way Vercel describes, not a gate.

## The loop

1. **Edit the before-state.** Point an agent at a fixture's `before/`, with the skill available, and
   the fixture `prompt`. It produces an edited result.
2. **Judge against the rubric.** Check the result against the `rules` listed for that fixture in
   `fixtures.json`, using the criteria in `rules-checklist.json`. Score **rule correctness**
   separately from **similarity to `after/`** — `after/` is a reference, not gospel; shipped code can
   carry a flaw the agent should *improve*, not reproduce.
3. **Measure the skill's effect.** Run the same fixture **without** the skill and compare — does the
   guidance change the outcome?

## Fixtures vs. holdouts

- A **fixture** (`holdout: false`) is derived from a decision documented in the skill (an exemplar or
  rule). It tests that the agent applies guidance it can see.
- A **holdout** (`holdout: true`) hides its expected edits from the skill — it tests whether the
  guidance *generalizes* to UI the skill doesn't describe.

## Layout (mirrors Vercel)

```
tooling/scripts/evals/
├── README.md
├── fixtures.json          # the fixture index: id, surface, mode, holdout, prompt, rules, source
├── rules-checklist.json   # rule id → how to check it (judge criteria, P-level, source)
└── <fixture-id>/
    ├── before/            # the starting UI (intentionally flawed; excluded from Biome)
    └── after/             # the reference result (one acceptable solution, not the only one)
```

Fixtures live under Biome's ignore list (see `biome.jsonc`) so the linter doesn't "fix" the very
mistakes a `before/` encodes.

## Retrieval vs. application

Test two different failures separately (per Vercel's finding that agents often fail to *load* an
available skill):

- **Retrieval:** did the agent load the skill at all, given the repo `AGENTS.md` trigger?
- **Application:** given the skill, did it follow the rule?

A fixture's `prompt` is phrased to exercise the trigger words in `AGENTS.md`; record both whether the
skill loaded and whether the rules passed.
