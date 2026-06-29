# Product judgment

Load for any Shape, Implement, Harden, or material flow change. This is the lens for *what* to
build, before `interface-quality.md` governs *how well* it's built.

## The compact brief

Before proposing UI, write this down (internally is fine). If you can't fill a field, that's an open
decision to surface, not a blank to skip.

- **User** — who is acting. Today: one modeler, drawing a residential space.
- **Job** — what they're trying to accomplish, in their words ("enclose an 8×8 room and give it a
  height"), not in component terms.
- **Object** — the canonical noun being acted on: footprint, mass, space, tool. Use
  [`apps/web/CONTEXT.md`](../../../apps/web/CONTEXT.md)'s term.
- **Current behavior** — what happens today, verified against the running app, not assumed.
- **Desired outcome** — the behavior that solves the job.
- **Success signal** — how the user (and you, in §7 verification) knows it worked.
- **Non-goals** — what this explicitly does not do. The MVP's non-goals are large and load-bearing
  (no framing, no any-face push/pull, no inspector, no elevation view — ADR 0007/0008); respect them.
- **Action / scope / consequence** — what command fires, what it changes, and what the user must be
  able to see and predict about that change.
- **Reversibility** — can the user undo it? Today there is no undo (a coverage gap); design as if a
  mistake costs a redraw, and don't add an irreversible gesture without reading `coverage-gaps.md`.
- **Open decisions** — the product choices not yet settled. Name them; don't bury them in code.

## The decisions that are already settled (don't re-litigate)

These are accepted; treat them as the ground you build on, and cite the ADR if you propose changing
one (that's an ADR-level change, not a UI tweak):

- **Space-first is the front door.** The user draws a *space* (footprint → push/pull → mass); walls
  and framing are a *derived* layer, deferred. `DrawWall` is off the MVP path
  ([ADR 0007](../../../docs/adr/0007-space-first-modeling-footprint-push-pull.md)).
- **Both panes are input surfaces.** Plan draws the footprint; 3D does push/pull. The 3D view is not
  read-only (this revised the earlier view; ADR 0007 §2).
- **The engine is the only source of truth.** The client mirrors SoA bytes and sends commands; it
  holds no second model. The in-progress footprint is the one piece of transient client state, and
  it is *not* geometry until the ring closes (ADR 0008).
- **Push/pull is vertical, top-cap only, in the MVP.** Any-face push/pull needs a general BREP
  modeler and is explicitly a later phase (ADR 0007 §3). Don't design UI that implies it works yet.
- **Units are ticks internally, feet/inches to the user.** 1ft = 384 ticks; 1 tick = 1/32in.

## Smallest coherent intervention

Jose's MVP is deliberately spare — a toolbar, two viewports, a status bar. Before adding chrome:

1. Can a **better default** solve it? (e.g. the right starting camera angle, a sensible snap.)
2. Can **clearer status text** solve it? The status bar is the primary feedback channel; use it
   before adding a panel.
3. Can you **reuse** an existing surface or pattern? (See `patterns.md`.)
4. Only then, add UI — and add the least that's coherent.

Adding an inspector to label one value, or a setting to expose one option, is almost always the
wrong size. The MVP has no inspector and no settings by decision; keep it that way until evidence
(not taste) demands otherwise.

## Material vs. mechanical

A change is **material** — and needs this brief plus, often, human sign-off — when it changes the
user's task, a default, scope, a consequence, navigation, the interaction surface, or which states
are reachable. Renaming a label to its canonical noun, swapping a hard-coded color for a token, or
substituting an established component is **mechanical** — just do it well and verify.
