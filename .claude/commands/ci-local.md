---
description: Run the full CI suite locally in this session (deterministic gates + agent reviews), no API key
---

# Run CI locally

Run everything CI runs — plus the in-session agent reviews — and report **one pass/fail summary**.
A Claude session is already authenticated, so the reviewers need no `ANTHROPIC_API_KEY` (ADR 0011).
This is a gate/review run: **do not edit files** unless the user asks.

## Steps

1. **Ensure deps.** If `node_modules/.bin/turbo` is missing, run `bun install --frozen-lockfile`.

2. **Deterministic JS/TS gate.** Run `bun run ci:local`
   (Biome → `codegen:check` → typecheck → test). Capture pass/fail per step.

3. **Deterministic Rust gate** (skip if `crates/` has no `Cargo.toml`):
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo check --workspace`
   - `cargo test --workspace`
   - `cargo build -p bim-wasm --target wasm32-unknown-unknown` (if `crates/bim-wasm` exists)

4. **Agent reviews** on the changed web files
   (`git diff --name-only origin/main...HEAD -- 'apps/web/**' 'packages/render-mirror/**' 'packages/tool-runner/**'`;
   fall back to the working-tree diff, or all of `apps/web/src` if none):
   - run `/web-interface-guidelines <files>`
   - run `/react-best-practices <files>`

5. **Summary.** One block:
   - each deterministic gate: ✅/❌ (with the failing command + first error on ❌)
   - reviews: the `[agent]`/`[gap]` (web) and `[applies]`/`[partial]` (react) findings, grouped by file
   - a final verdict: **GREEN** only if every deterministic gate passed (reviews are advisory — list
     them but they don't flip the verdict).
