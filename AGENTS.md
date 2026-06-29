# AGENTS.md

Repo-level guidance for coding agents. Two companion files govern this repo:

- **[`CLAUDE.md`](./CLAUDE.md)** governs the **engine and the build** — the MODEL-as-source-of-truth
  rule, the crate/context boundaries, the WASM keystone, the commands, and the conventions. Read it
  for anything touching `schema/`, `crates/`, `packages/`, `tooling/`, or the build.
- **This file** routes you to the **`product-design` skill** for anything a *user* sees, understands,
  chooses, or does.

## When to load the product-design skill

When shaping, editing, or reviewing user-facing UI in `apps/web`, load
[`.agents/skills/product-design/SKILL.md`](./.agents/skills/product-design/SKILL.md).

**Applies to:**

- the app shell, the plan view, and the 3D view (`apps/web/src/**`)
- copy and accessible names, interaction, hierarchy and layout, responsive behavior, and the
  reachable states of the drawing UX (loading, empty, drawing-in-progress, mass present, push/pull
  active, disabled, error)
- the read side of the boundary — how the engine's canonical geometry is presented
  (`packages/render-mirror`, `packages/tool-runner`), when a change alters what the user sees or does

**Skip:**

- engine/domain work in `crates/**` with no user-visible effect — that's `CLAUDE.md`'s domain
- the MODEL and generated files (`schema/**`, any `generated/**`) — edit the model, run codegen
- persistence (`apps/api`), telemetry, build tooling, and documentation with no shipped UI impact
- tests with no shipped UI change

When you load the skill, **report which surfaces and references you loaded**, and make sure your
findings cite those sources. Loading the skill and following its rules are two different things;
say which you did.

## Running CI locally (no API key)

CI is just the commands in `.github/workflows/ci.yml`; they run in-session. The agent-judgment
reviewers run here too — a Claude session is already authenticated, so they need **no
`ANTHROPIC_API_KEY`** (that's only for headless GitHub Actions). See
[ADR 0011](./docs/adr/0011-advisory-ui-review-in-ci.md).

- **`bun run ci:local`** — the deterministic JS/TS spine (Biome + `codegen:check` + typecheck + test).
  Add the Rust gates (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`) for a full run.
- **`/ci-local`** — orchestrates everything in one shot: the deterministic gates **plus** the two
  in-session reviewers below, on the changed web files.
- **`/web-interface-guidelines <files>`** — keyboard/focus/animation/touch/typography/copy review,
  Jose-scoped (`.agents/skills/product-design/references/web-interface-guidelines.md`).
- **`/react-best-practices <files>`** — React re-render / JS-perf review, vendored from Vercel and
  scoped to Jose's Vite + Three.js SPA (`.agents/skills/vercel-react-best-practices/`).

**Habit:** before pushing changes to web files, run `/ci-local` (or at least the relevant reviewer)
on the diff. There is no automatic PR gate for the reviewers — running them is the gate.
