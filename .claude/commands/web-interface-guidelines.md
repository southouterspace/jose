---
description: Review UI code for Web Interface Guidelines compliance (Jose-scoped)
argument-hint: <file-or-pattern>
---

# Web Interface Guidelines review

Review these files for compliance: $ARGUMENTS

**Source of truth:** read
`.agents/skills/product-design/references/web-interface-guidelines.md` and apply its rules. Do not
use a remembered rule list — that file is canonical and Jose-scoped, and it tags every rule with an
enforcement tier.

## How to run it

1. Read the rules file above. Note the **enforcement tiers**.
2. Read the target files in `$ARGUMENTS` (default to changed web files if none given:
   `apps/web/**`, `packages/render-mirror/**`, `packages/tool-runner/**`).
3. Report **only `[agent]` and `[gap]` findings.** Skip:
   - `[biome]` rows — already gated by `bun run lint` (the required CI check); re-reporting is noise.
   - `[n/a]` rows — no surface in Jose (forms, `<img>`, i18n, SSR/hydration, virtualized lists);
     flagging them is noise until that surface exists.
4. Respect the recorded **exceptions** (e.g. `touch-action: none` on the drawing surfaces is
   intentional — do not flag it) and the **canonical surface nouns** in `apps/web/CONTEXT.md`.
5. For a `[gap]` finding, name the rule so it can later graduate to a deterministic check.

Read files, check against the rules, output concise but comprehensive — sacrifice grammar for
brevity. High signal-to-noise. No preamble.

## Output format

Group by file. Use `file:line` (clickable). Terse findings. State issue + location; skip the
explanation unless the fix is non-obvious.

```text
## apps/web/index.html

apps/web/index.html:6 - missing <meta name="theme-color"> matching page background  [gap]

## apps/web/src/app.tsx

apps/web/src/app.tsx:53 - status bar is a live region; add aria-live="polite"  [agent]

## apps/web/src/three-view.tsx

✓ pass
```
