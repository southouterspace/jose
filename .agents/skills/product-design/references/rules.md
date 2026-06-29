# Rules

Rules with stable IDs, each traceable to a canonical source. A rule lives here when it needs product
or codebase **judgment** to apply. A rule that code can check **reliably** belongs in a deterministic
check instead (today: Biome/Ultracite via `biome.jsonc`; see "Deterministic checks" below) — keep
the judgment here and the mechanics there.

Cite a rule by ID in review findings and PRs (e.g. "violates `rule/display-feet-not-ticks`").

## Rule template

```
## rule/{stable-id}
Scope:   where it applies
Rule:    the decision, stated as an observable behavior
Why:     the user/system consequence if violated
Source:  canonical owner (ADR, CONTEXT.md, file:line, or a check)
Exceptions: when it does not apply (omit if none)
Bad:     a concrete violation
Good:    the corrected form
```

A new rule must clear the governance bar in [`AGENTS.md`](../AGENTS.md): current-source verification,
human acceptance, and all fields filled. Never promote one screenshot or one file into a rule alone.

---

## rule/one-direction-render
- **Scope:** anything on the render side (`apps/web`, `packages/render-mirror`, `packages/tool-runner`).
- **Rule:** the client mirrors canonical geometry and emits commands; it never owns or mutates
  canonical geometry. Canonical geometry renders **from a mirror**, transient gesture state stays
  local and visually distinct.
- **Why:** a second client-side source of truth desyncs from the engine and can corrupt reads; it
  breaks the keystone the whole architecture rests on. **P0** if violated.
- **Source:** [ADR 0003](../../../docs/adr/0003-wasm-boundary-and-the-buffer-layout-keystone.md),
  [ADR 0006](../../../docs/adr/0006-world-space-placement-engine-side.md); `apps/web/CONTEXT.md` "Mirror".
- **Bad:** mutating a mirror's vertices in place to "preview" an edit; rendering `pendingPicks` as the
  committed footprint.
- **Good:** keep the edit transient and local; send a command; re-render from the returned mirror.

## rule/display-feet-not-ticks
- **Scope:** every user-facing dimension (status text, labels, any readout).
- **Rule:** display feet/inches; never show raw ticks. Convert from canonical ticks at the boundary
  (1ft = 384 ticks).
- **Why:** ticks (1/32in) are an engine unit; showing them is meaningless to the user.
- **Source:** `docs/plans/drawing-ux-mvp.md` (units); `heightFeet` in `apps/web/src/app.tsx`.
- **Bad:** `mass ${volume.height} ticks tall`.
- **Good:** `mass ${(height / 384).toFixed(1)}ft tall`.

## rule/canonical-noun
- **Scope:** all user-facing copy, `aria-label`s, and new element/identifier names.
- **Rule:** use the term owned by `apps/web/CONTEXT.md`; never a synonym.
- **Why:** the ubiquitous language keeps the UI, the docs, and the engine speaking one vocabulary;
  synonyms erode it.
- **Source:** `apps/web/CONTEXT.md`; `copy.md`.
- **Bad:** a button labeled "Extrude"; an `aria-label="2D canvas"`.
- **Good:** "Push/Pull"; `aria-label="Plan drawing surface"`.

## rule/control-accessible-name
- **Scope:** every interactive control.
- **Rule:** every control has an accessible name — from its text, or an explicit `aria-label` for
  icon-only/ambiguous controls. The active state of a toggle uses `aria-pressed`.
- **Why:** an unnamed control is invisible to assistive tech; a missing pressed-state hides which
  tool is live. **P1** (P0 if it blocks the primary task for AT users).
- **Source:** `apps/web/src/app.tsx` (the tool buttons + `aria-pressed`); general accessibility.
- **Bad:** an icon-only tool button with no label.
- **Good:** `aria-label="..."` plus `aria-pressed={isActive}`.

## rule/stable-control-label
- **Scope:** buttons and tool controls.
- **Rule:** a control's label is stable across state; show progress/loading via the component's
  busy/disabled affordance and the status bar, not by relabeling.
- **Why:** a label that mutates is a moving target and breaks AT announcements and muscle memory.
- **Source:** `interface-quality.md`; `app.tsx` (labels are constant; status text carries progress).
- **Bad:** a tool button that switches from "Footprint" to "Drawing…" to "Done".
- **Good:** label stays "Footprint"; the status bar reports progress.

## rule/precondition-disabled-only
- **Scope:** enabling/disabling controls.
- **Rule:** disable a control only for a real precondition, and make the reason discoverable (status
  bar). Never disable as decoration or to hide an unfinished path silently.
- **Why:** an unexplained disabled control is a dead end.
- **Source:** `app.tsx` (`hasMass` gates Push/Pull); `pattern/precondition-disabled-tool`.
- **Bad:** Push/Pull disabled with no indication of why.
- **Good:** disabled until a mass exists, with the status bar pointing to "draw a footprint first".

## rule/destructive-names-action
- **Scope:** any destructive or irreversible action (none in the MVP yet).
- **Rule:** destructive CTAs follow **Verb + Noun** naming the real object and consequence; never
  "Confirm", "OK", or a bare verb. Provide undo when the system can honestly support it.
- **Why:** a vague destructive label hides the consequence; with no undo (a current coverage gap) the
  stakes are high.
- **Source:** `copy.md`; `coverage-gaps.md` (no-undo).
- **Bad:** a button reading "Confirm" that deletes the footprint.
- **Good:** "Delete footprint", proportional confirmation, undo if/when supported.

---

## Deterministic checks (the other leg)

Jose enforces the clear, mechanical rules with **Biome (Ultracite preset)** — `bun run lint` /
`bunx biome ci` — plus `bun run typecheck`. That covers a large class of correctness/a11y/style
issues without prose. The decision rule (from the Vercel pattern):

> Can code identify the failure without rendering? If yes, and the rule avoids likely false
> positives, and the fix is concrete → a check. Otherwise → agent guidance here. New standard or
> unresolved product policy → a human decides.

Jose has **no custom lint rules of its own yet** (only the Ultracite preset) — adding one (e.g. a
mechanical `display-feet-not-ticks` check) is a recorded **coverage gap**, not yet built. Don't claim
a rule here is auto-enforced unless it's actually in `biome.jsonc` or a check.
