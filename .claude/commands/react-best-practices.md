---
description: Review React/JS code for performance best practices (Vercel skill, Jose-scoped)
argument-hint: <file-or-pattern>
---

# React best-practices review

Review these files: $ARGUMENTS

**Sources of truth (read both first):**
1. `.agents/skills/vercel-react-best-practices/SKILL.md` — the rule index (70 rules, 8 categories).
2. `.agents/skills/vercel-react-best-practices/jose-scope.md` — **which categories apply to Jose** (a
   React + Vite client SPA with a Three.js renderer; no Next.js / RSC / SSR).

## How to run it

1. Read the two files above. Apply the scope: report findings only for `[applies]`/`[partial]`
   categories; **do not report `[n/a]` rules** (server-*, hydration, swr, next/dynamic, etc.) — they
   have no surface in Jose.
2. Read the target files in `$ARGUMENTS` (default to changed React/TS in `apps/web/**`,
   `packages/render-mirror/**`, `packages/tool-runner/**` if none given).
3. Respect Jose's established patterns over generic advice: the imperative Three.js ref idiom
   (`three-view.tsx`) is intentional and *is* `advanced-event-handler-refs` — flag deviations, not
   the pattern. Never suggest caching canonical geometry client-side (breaks the one-direction rule).
4. For a rule's full example, point to the upstream `rules/<id>.md` (not vendored).

Output concise, high signal. No preamble.

## Output format

Group by file. `file:line`. Name the rule id. Fix only if non-obvious.

```text
## apps/web/src/plan-view.tsx

apps/web/src/plan-view.tsx:43 - gridLines() rebuilt every render; hoist or useMemo  [rerender-memo / js-*]

## apps/web/src/engine-store.ts

✓ pass
```
