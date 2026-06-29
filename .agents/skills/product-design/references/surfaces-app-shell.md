# Surface: app shell

The frame that organizes the UI — the toolbar/tool palette, the status area, and the two-viewport
layout. React-owned chrome; it holds **no canonical geometry**. Canonical language owner:
[`apps/web/CONTEXT.md`](../../../apps/web/CONTEXT.md) "App shell". Code: `apps/web/src/app.tsx`,
`apps/web/src/app.css`.

_Use "app shell" as the proper noun — avoid "layout", "frame", "chrome"._

## Structure

A flex column: `header.toolbar` → `main.viewports` (a `1fr 1fr` grid of two `section.viewport`) →
`footer.statusbar`. Keep this shape; extend it, don't nest new wrappers around it.

## The toolbar and tools

- Tools live in a `<nav aria-label="Drawing tools">` as `<button type="button">`. A **tool** is a
  picking state machine that turns gestures into a command (`CONTEXT.md` "Tool"); the **active tool**
  is the one receiving picks.
- The active tool carries `aria-pressed={true}`. Keep that in sync with `store.activeTool` — it's the
  only signal a screen reader gets for which tool is live.
- **Enable/disable on a real precondition only.** Push/Pull is `disabled` until a mass exists
  (`hasMass = (store.volume?.count ?? 0) > 0`) because it acts on the 3D mass. When you add a tool,
  state its precondition explicitly and disable on that — never disable as decoration.
- **Labels are stable.** A tool button reads "Footprint" / "Push/Pull" regardless of state. Convey
  progress and the reason for a disabled control in the **status bar**, not by relabeling the button.

## The status bar

The MVP's primary feedback channel. `statusText(store)` maps the current state to one line. Rules:

- Every reachable state produces a line (see the inventory in `surfaces.md`). Adding a state means
  adding its line here.
- The line says **what's active and what to do next** ("Push/Pull active — drag the top cap in 3D to
  set the mass height"), in canonical nouns.
- Display **feet**, never ticks ("mass 8.0ft tall"). See `copy.md` for the units convention.
- When a tool is disabled, the status bar is where the user learns the path to enabling it.

## Decisions settled here

- The shell is chrome only; it never reads or writes geometry. Selection/active-tool state lives in
  React; TanStack is pulled in only if that state outgrows plain React state
  ([ADR 0005](../../../docs/adr/0005-frontend-application-stack-react-vite.md)).
- The two panes are peers (`1fr 1fr`), both input surfaces (ADR 0007).

## Coverage gaps (don't claim these work)

- **No error/permission state** in the shell — a failed engine load or rejected command has no
  status treatment beyond the initial "Loading engine…".
- **No compact breakpoint** — the grid doesn't stack; narrow windows crush both panes.
- **No undo affordance** — there is nothing in the shell to reverse a draw or a push/pull.

See `coverage-gaps.md` before designing into any of these.
