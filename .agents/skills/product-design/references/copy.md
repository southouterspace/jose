# Copy

User-facing language: status text, control labels, accessible names. Load for Copy mode and whenever
you touch a string a user (or a screen reader) reads. The canonical product nouns are owned by
[`apps/web/CONTEXT.md`](../../../apps/web/CONTEXT.md); this file governs *how* to write with them.

## Use the canonical noun, always

Jose has a deliberate ubiquitous language. Copy uses the term from `CONTEXT.md` and nothing else:

| Use | Not | Source |
| --- | --- | ------ |
| footprint | outline, polygon, sketch, perimeter | `CONTEXT.md` "Footprint" |
| mass | block, box, model, solid | `CONTEXT.md` "Mass" |
| space | room, zone, area | `CONTEXT.md` "Space" |
| push/pull | extrude, drag, pull-up | `CONTEXT.md` "Push/pull" |
| plan view | 2D view, top view, floorplan | `CONTEXT.md` "Plan view" |
| 3D view | model view, scene, perspective view | `CONTEXT.md` "3D view" |
| tool | mode, gesture handler | `CONTEXT.md` "Tool" |
| viewport | pane, canvas, window | `CONTEXT.md` "Viewport" |

A copy change that introduces a synonym for one of these is wrong even if it reads well. If you think
a term should change, that's a `CONTEXT.md` change (its owner), not a copy tweak — see governance in
`AGENTS.md`.

## Units: feet/inches to the user, never ticks

Canonical geometry is ticks (1 tick = 1/32in; 1ft = 384 ticks). **Every user-facing dimension is
feet/inches.** "mass 8.0ft tall", not "3072 ticks". `app.tsx`'s `heightFeet` already converts; any
new readout must too. (`rule/display-feet-not-ticks`.)

## Status text

The status bar tells the user **what's active and what to do next**, in one line:

- Lead with state, then the next action: *"Push/Pull active — drag the top cap in 3D to set the mass
  height."*
- Use present tense and the canonical noun. Address the user's action, not the system's internals
  ("click the first to close", not "ring closure pending").
- Count concretely when it helps: *"Drawing footprint — 3 point(s); click the first to close."*
- Don't relabel **controls** to show progress — that's the status bar's job (`interface-quality.md`).

## Labels and accessible names

- **Controls are stable, verb-or-noun labels.** Tool buttons read "Footprint" / "Push/Pull".
- **Every control has an accessible name.** A text button gets it from its text; an icon-only or
  ambiguous control needs an explicit `aria-label` (`rule/control-accessible-name`). Today the
  viewports and drawing surfaces carry `aria-label`s (`Plan viewport`, `Plan drawing surface`,
  `3D viewport`); keep them accurate when a surface is renamed.
- **`aria-label` uses the canonical noun too.** "Plan drawing surface", not "2D canvas".

## Destructive / irreversible language

Nothing in the MVP is destructive yet. If you add a destructive action, its label is **Verb + Noun**
naming the real object and consequence ("Delete footprint"), never a bare "Confirm", "OK", or a lone
verb (`rule/destructive-names-action`). And don't add one without reading `coverage-gaps.md` — there
is no undo, so the bar for an irreversible action is high.

## Tone

Spare and direct, matching the product. No decorative or jokey copy; a string earns its place by
clarifying structure, state, or the next action.
