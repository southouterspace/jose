---
name: product-design
description: >-
  Single entry point for product design and user-facing implementation in apps/web — Jose's
  drawing UX (the app shell, the plan view, the 3D view). Use whenever work changes what a user
  sees, understands, chooses, or does: shaping a modeling flow; building or redesigning a viewport,
  toolbar, or status surface; reviewing a URL, screenshot, or diff; improving product copy, the
  ubiquitous-language naming of tools and objects, information hierarchy, layout, interaction,
  accessibility, responsive behavior, and reachable states (loading the engine, empty, drawing a
  footprint, mass present, push/pull active, disabled, error). Trigger on design, UX, UI, usability,
  flow, viewport, plan view, 3D view, toolbar, status bar, footprint, push/pull, tool, build,
  improve, fix, audit, review, polish, simplify, or production-ready requests. Also use when an
  engine or boundary change alters a user-visible outcome. Not for engine/domain work in crates/
  with no user-visible effect, the MODEL or generated files, persistence (apps/api), telemetry,
  build tooling, documentation, or tests with no shipped UI impact.
---

# Jose Product Design

Make the interface correct for the user, the model, and the engine's truth. Working code is not
enough: choose the right interaction, make scope and consequence clear, cover the states the model
can actually reach, never let the render side claim to own geometry it only mirrors, and verify the
rendered result.

## Operating Contract

- **Start with the job, not the pixels.** Identify who is acting, what they are trying to model,
  the object involved (footprint, mass, space, tool), and what the engine will change.
- **Define the outcome before the output.** Establish the current user problem, the desired
  behavior, the success signal, and the non-goals before choosing a surface or component.
- **Use evidence, not taste.** Trace each decision to product behavior, a canonical source
  (`apps/web/CONTEXT.md`, an ADR, `CLAUDE.md`), an accepted decision, or a verified adjacent pattern
  already in `apps/web/src`.
- **Honor the one-direction rule.** The render side is **eyes and hands, not the brain**. The client
  never owns or mutates canonical geometry; it mirrors the engine's SoA bytes and sends gestures as
  commands. A design that makes the UI a second source of truth is wrong before it is ugly
  ([ADR 0003](../../../docs/adr/0003-wasm-boundary-and-the-buffer-layout-keystone.md),
  [ADR 0006](../../../docs/adr/0006-world-space-placement-engine-side.md)).
- **Separate facts from decisions.** Mark assumptions and unresolved product choices explicitly; do
  not bury them inside an implementation detail.
- **Treat shipped code as evidence, not automatic precedent.** It proves what exists, not why it's
  correct. Check it against the current `CONTEXT.md` language, the ADRs, and the model's real states.
- **Choose the smallest coherent intervention.** Prefer a better default, clearer status text, or
  reuse before adding chrome, a panel, or a setting. The MVP deliberately has no inspector — don't
  add one to solve a labeling problem.
- **Decide before decorating.** Resolve which surface, which object, which interaction, and which
  states before styling or rewriting copy.
- **Design every reachable state — and only reachable ones.** Include the states the *model* can
  enter (see §4); don't invent states the engine can't produce, and don't stop at the happy
  populated case.
- **Verify the real surface.** Source inspection establishes behavior; a rendered viewport
  establishes visual and interaction quality. Never claim visual verification from code alone.
- **Keep one user-facing entry point.** Invoke `product-design`; route internally to the sources below.

## Request Modes

Resolve the mode from the user's verb and artifact before acting.

| Mode      | Typical request                                                              | Required behavior                                                                                                                                                       |
| --------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Shape     | "Design this flow", "How should selection work?", a feature brief with no settled UI | Frame the problem and evidence, compare material alternatives, then define the flow, states, acceptance criteria, risks, and open decisions. Do not edit unless asked. |
| Implement | "Build", "fix", "improve", "make it compliant"                              | Resolve the material product decisions, then implement the smallest coherent end-to-end change within scope. Do not absorb unrelated review findings.                  |
| Review    | "Audit", "critique", "what's wrong?", a code review                         | Inspect source and rendered evidence, then report prioritized findings. Do not edit unless asked.                                                                      |
| Copy      | "Fix the status text", "rewrite this label"                                 | Edit user-facing language, accessible names, and the JSX directly required. Report structural blockers without silently widening scope.                                |
| Harden    | "Polish", "production-ready", "handle the edge cases"                       | Preserve the settled direction while fixing state, resilience, responsive, accessibility, and finish defects.                                                          |

When intent is ambiguous, use the narrowest mode the verb supports. A URL, screenshot, route, or
component identifies *scope*; it does not by itself authorize edits.

A **material decision** changes the user's task, default, scope, consequence, navigation,
interaction surface, or reachable states. Copy mechanics, token swaps, and an established component
substitution usually are not material.

## Decision Authority

Resolve conflicts in this order:

1. The user's explicit goal and constraints.
2. Verified user/product evidence and system truth (what the engine and the model can actually do).
3. Repository-canonical guidance: [`CLAUDE.md`](../../../CLAUDE.md),
   [`apps/web/CONTEXT.md`](../../../apps/web/CONTEXT.md), the ADRs under
   [`docs/adr/`](../../../docs/adr/), and [`biome.jsonc`](../../../biome.jsonc).
4. Accepted product/design decisions and the exemplars under `exemplars/`, with stable evidence.
5. Verified adjacent shipped patterns in `apps/web/src`.
6. General interface heuristics.

## Workflow

### 1. Set scope and mode

Name the target surface (app shell / plan view / 3D view) and the request mode in your work plan or
review notes.

### 2. Load product context

Before proposing UI, read the applicable `AGENTS.md`/`CONTEXT.md` chain, any supplied brief or
design, and the product logic that determines what the engine mutates: the command contract
([ADR 0008](../../../docs/adr/0008-mvp-geometry-and-command-contract.md)), what is canonical vs.
transient client state, and what the mirror exposes.

### 3. Model the product decision

For Shape, Implement, Harden, a full Review, or any material flow change, read
[`references/product-judgment.md`](./references/product-judgment.md) and write a compact internal
brief: user, job, current behavior, desired outcome, success signal, non-goals, object, scope,
action, consequence, reversibility, and open decisions.

### 4. Map the surface and states

Inventory entry points, viewports, overlays, transitions, and exits. Map only the states the
**model** can reach. For the MVP that is: engine loading; ready/empty; footprint in progress
(transient picks); footprint closed; mass present; Push/Pull active; Push/Pull disabled (no mass
yet); and the responsive/large-value variants. See
[`references/surfaces.md`](./references/surfaces.md) for the per-surface state inventory.

### 5. Load the routed references

| Need | Load |
| ---- | ---- |
| Product / flow / object decision | [`references/product-judgment.md`](./references/product-judgment.md) + [`apps/web/CONTEXT.md`](../../../apps/web/CONTEXT.md) |
| Implementation, material visual change, or full review | [`references/interface-quality.md`](./references/interface-quality.md) |
| Copy, labels, status text, accessible names | [`references/copy.md`](./references/copy.md) + [`references/surfaces.md`](./references/surfaces.md) routing |
| The exact object/tool names | [`references/glossary.md`](./references/glossary.md) → [`apps/web/CONTEXT.md`](../../../apps/web/CONTEXT.md) (the owner) |
| App shell (toolbar, tools, status bar) | [`references/surfaces-app-shell.md`](./references/surfaces-app-shell.md) |
| Plan view (drawing the footprint) | [`references/surfaces-plan-view.md`](./references/surfaces-plan-view.md) |
| 3D view (push/pull, orbit, picking) | [`references/surfaces-3d-view.md`](./references/surfaces-3d-view.md) |
| Reusable interaction patterns | [`references/patterns.md`](./references/patterns.md) |
| Overflow, large values, units, network/error resilience | [`references/resilience.md`](./references/resilience.md) |
| Deterministic rules with stable IDs | [`references/rules.md`](./references/rules.md) |
| Where we have no standard yet | [`references/coverage-gaps.md`](./references/coverage-gaps.md) |

### 6. Decide, then implement

For each non-mechanical change, be able to answer: what user problem does this solve, why is this
surface/interaction appropriate, what consequence must it communicate, which evidence supports it,
and what is the smallest coherent change?

### 7. Verify

1. Confirm the primary job and acceptance criteria.
2. Run `bun run lint`, `bun run typecheck`, and `bun test` for the affected package.
3. Inspect a compact and a wide viewport (the two-pane grid collapses badly when narrow — see
   `coverage-gaps.md`).
4. Exercise every materially changed reachable state from §4.
5. Verify pointer/touch behavior in both viewports, keyboard reachability of the toolbar, and that
   `aria-pressed`/`aria-label`/disabled states match the real tool state.
6. Test a large footprint, a tall mass, and a tiny one (status text and framing must not break).
7. Re-read `apps/web/CONTEXT.md` to confirm any new user-facing noun uses the canonical term.

## Product Design Standards

- Make the user's primary task (draw a space) and primary action (the active tool) unmistakable.
- Preserve the user's context unless changing it solves a verified problem — e.g. a height-only
  push/pull must **not** re-frame the camera the user is mid-orbit on (`three-view.tsx`).
- Name the exact object, scope, and consequence in copy. Use the canonical noun
  (`apps/web/CONTEXT.md`): footprint, mass, space, push/pull, plan view, 3D view, tool — not
  outline, box, room, extrude, 2D view, scene, or mode.
- Use the active tool's affordance and `aria-pressed`; disable a tool only for a real precondition
  (Push/Pull is disabled until a mass exists) and let the status bar say why.
- Keep transient state transient: the in-progress footprint is client-only; only a closed ring
  becomes a `DrawFootprint`. Never render raw picks as if they were canonical geometry.
- Prefer strong defaults and direct gestures over configuration the user must learn (the MVP has no
  settings; keep it that way unless evidence demands one).
- Use hierarchy, spacing, and the existing CSS classes before adding containers or new chrome.
- Display canonical ticks as **feet/inches** in the UI; ticks (1/32in) never surface to the user.
- Make destructive or irreversible gestures proportional and recoverable; today nothing is
  destructive — if you add one, see `coverage-gaps.md` before shipping it.
- Do not add decorative motion, color, or copy unless it clarifies structure, state, or the model.

## Review Output

Lead with findings, ordered by user impact:

- **P0:** blocks the primary task (can't draw or push/pull), a severe accessibility failure, or a
  violation of the one-direction rule that can corrupt or desync canonical geometry.
- **P1:** likely task failure, a misleading consequence, a missing critical state, or a major
  responsive/accessibility defect.
- **P2:** meaningful friction, inconsistency, weak hierarchy, or a recoverability issue.
- **P3:** minor craft or consistency improvement.

For each finding include: file/line or rendered location, verification status (source vs. rendered),
the canonical source, the user consequence, and the smallest concrete fix.

## Skill Integrity

See [`AGENTS.md`](./AGENTS.md) for the full governance bar. In short: add or change a rule only after
current-source verification and human acceptance; record scope, rationale, evidence, exceptions, and
a bad/good example; prefer the narrowest destination; keep deterministic checks mechanical and
judgment in prose with its evidence; never promote one screenshot, one file, or one comment into a
universal rule by itself.
