# Interface quality

Load for implementation, a material visual change, or a full review. `product-judgment.md` decides
*what* to build; this decides whether it's built *well*. Findings here map to the P0–P3 scale in
`SKILL.md`.

## Hierarchy and layout

- Lead with the primary task. In the drawing UX the two viewports are the work; the toolbar and
  status bar are rails. Don't let chrome compete with the viewports for weight.
- Use spacing, alignment, and the existing CSS classes (`app.css`) before adding a container or a
  border. The shell is a flex column (`toolbar` / `viewports` grid / `statusbar`); extend that
  structure rather than nesting new wrappers.
- Keep the two-pane grid balanced (`grid-template-columns: 1fr 1fr`). If a surface needs more room,
  that's a layout *decision* (record it), not an ad-hoc width override.

## Semantics first

- **Navigation elements navigate; action elements act.** The tools are `<button type="button">`
  inside a `<nav aria-label="Drawing tools">` — they activate a tool (an action), so they must stay
  buttons, not links. The active one carries `aria-pressed`.
- Use a real `<button>` for anything clickable; never a clickable `<div>`. Icon-only or
  ambiguous controls need an accessible name (see `copy.md` and `rules.md`
  `rule/control-accessible-name`).
- Each viewport `<section>` carries an `aria-label` (`Plan viewport`, `3D viewport`); the drawing
  surfaces carry their own (`Plan drawing surface`). Keep these accurate when you rename a surface.

## State and feedback

- The **status bar is the primary feedback channel** in the MVP. Every reachable state should
  produce status text that tells the user what's active and what to do next (`statusText` in
  `app.tsx`). When you add a state, add its status line.
- Keep control labels **stable across state**. A tool button reads "Footprint" whether or not a
  footprint exists; convey progress in the status bar and the viewport, not by relabeling the
  control. (Disabling is fine when there's a real precondition; see below.)
- Disable a control only for a real precondition, and make the reason discoverable. Push/Pull is
  `disabled` until a mass exists (`app.tsx`); the status bar should explain the path to enabling it.

## Visual finish

- Honor the dark theme (`color-scheme: dark`, the `#1b1b1f`/`#202024` surfaces). New color must read
  against it and shouldn't be introduced ad hoc — Jose has no design-token system yet (coverage
  gap); reuse the existing palette in `app.css` and flag if you need more.
- Distinguish **transient** from **canonical** rendering visually and in code: the mid-draw polyline
  is dashed (`plan__pending`), the committed footprint is solid (`plan__footprint`). Don't blur that
  line — it mirrors the one-direction rule.
- The 3D mass uses a translucent wall + a distinct, named top cap so the push/pull target reads as
  grabbable. If you restyle it, keep the top cap visually identifiable as the interactive face.

## Interaction

- Match the gesture to the surface: clicks place footprint vertices in plan; a vertical drag on the
  top cap sets height in 3D; orbit otherwise. Don't overload one gesture with two meanings in the
  same tool state.
- Freeze conflicting interactions during a drag (orbit is disabled while pushing/pulling —
  `three-view.tsx`). A new drag interaction must do the same.
- `touch-action: none` is set on both surfaces so the browser doesn't hijack the gesture; preserve
  it on any new input surface.

## The bar for "done"

A change is done when it passes lint/typecheck/tests, every reachable state it touches has been
exercised on a rendered surface, the canonical nouns are used, and you can state the user problem it
solves. "It compiles and looks right in the diff" is not done — only a rendered viewport establishes
visual quality.
