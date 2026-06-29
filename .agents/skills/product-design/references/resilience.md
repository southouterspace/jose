# Resilience

Load when work touches overflow, large or degenerate values, units, or network/error paths. The
happy path (draw a tidy square, pull it up) is the easy 80%; this is the reality the MVP mostly
hasn't faced yet. Several items here are **coverage gaps** — flagged so you design defensively and
don't claim a guarantee the code doesn't make.

## Units and large values

- Canonical geometry is in **ticks** (1 tick = 1/32in; 1ft = 384 ticks). The UI must display
  **feet/inches** — never raw ticks (`heightFeet` in `app.tsx` divides by 384 to one decimal).
- Design status text and any future readout for a **wide range**: a closet (a few feet) and a
  40-foot wall must both read cleanly. `toFixed(1)` is fine for height today; check that a long
  footprint vertex count or a large dimension doesn't overflow the status bar.
- The plan view maps `~0.05 px/tick` over a fixed `640×640` viewBox with a `±7680`-tick grid
  (`plan-view.tsx`). A footprint drawn outside that span renders off-grid or off-view — there is no
  pan/zoom yet (**coverage gap**). Don't assume the user can always reach or see what they drew.

## Degenerate and edge geometry

- A footprint needs **≥ 3 vertices** to be a ring (`hasRing = count >= 3`). Fewer is transient, not
  canonical — keep it that way.
- Push/pull distance can be **negative** (drag down). The mesh only builds when `height > 0`
  (`rebuildMass` guards `heightUnits <= 0`); a non-positive height renders nothing. If you add a
  numeric height path, reject or clamp non-positive values *with a message*, not silently.
- A self-intersecting or near-zero-area footprint is not validated in the client today (**coverage
  gap**). The engine is the authority; if it rejects a command, the UI must surface that — see error
  handling below.

## Network, worker, and error paths

- The engine runs in a **Web Worker** and loads asynchronously. The app shows `"Loading engine…"`
  until `store.ready` (`app.tsx`). Any surface that can render before the engine is ready must have a
  loading state — don't render an interactive viewport that silently no-ops on click.
- **There is no user-visible error state yet** (**coverage gap**). If the worker fails to load, a
  command is rejected, or the `LAYOUT_HASH` assertion fails (`assertLayout` in render-mirror), the
  user currently gets nothing actionable. When you touch this path, design the error surface:
  what the user sees, whether their input is preserved, and how they recover. Don't ship a new
  command path without deciding what its failure looks like.
- **Preserve user input through a recoverable failure.** A rejected `DrawFootprint` should not
  silently discard the picks the user just made without a way to retry.

## Responsive and input

- The shell is a full-height flex column with a 2-column viewport grid (`app.css`). It has **no
  compact/stacked breakpoint** (**coverage gap**): on a narrow viewport the two panes crush to
  unusable widths. Don't claim responsive support; if you add a breakpoint, decide whether the panes
  stack or one becomes primary.
- Both drawing surfaces set `touch-action: none` for pointer gestures. Touch *works* for orbit and
  clicks, but touch ergonomics (target sizes, the close-the-ring tap tolerance, pinch-zoom) are
  unverified (**coverage gap**).

## The rule of this file

When you hit one of these gaps, do the honest thing: handle it if it's in scope, and **say so in the
review or the PR** if it isn't. Silent truncation, a swallowed error, or an unhandled degenerate
case reads as "covered" when it isn't. Add the gap to `coverage-gaps.md` if it's new.
