# Patterns

Reusable interaction patterns already shipped in the drawing UX. Reach for an established pattern
before inventing one; cite the pattern and the file when you reuse it. Each is a *verified adjacent
pattern* (Decision Authority level 5) — real, in the codebase, and consistent with the ADRs.

## pattern/transient-then-canonical

**Shape:** the user builds something up locally (transient client state), and only a *completed*
unit crosses into the engine as a command; the result then renders **from the mirror**, not from the
local state.

**Where:** footprint drawing — `pendingPicks` (dashed polyline) is client-only; the closed ring fires
`DrawFootprint`; the committed footprint renders from `FootprintMirror` (`plan-view.tsx`).

**Use when:** any multi-step gesture (a future multi-segment wall, a selection lasso). Keep the
in-progress state visually distinct (dashed/ghosted) from canonical (solid). Never send partial state
as a command; never render local state as canonical.

## pattern/named-face-picking

**Shape:** to act on a face of canonical geometry, raycast to a **named** engine face, not a
surface-normal guess; confirm with a cheap sanity check.

**Where:** push/pull picks the `top-cap` mesh and dispatches against `TOP_FACE` (the kernel's named
index), confirming the world normal is vertical only as a guard (`three-view.tsx`, ADR 0008 §3).

**Use when:** any future face interaction. Make the pickable element its own named mesh; reference the
engine's named face as the source of truth.

## pattern/preserve-camera-on-nonspatial-change

**Shape:** re-frame the camera only when the thing being viewed *moved in plan*; leave it alone for a
change that doesn't (the user may be mid-orbit).

**Where:** `frameView` runs only when `footprintSig` changes; a height-only push/pull does not
re-frame (`three-view.tsx`).

**Use when:** any update that changes the model without changing its plan footprint. Track a signature
of the spatial thing and gate re-framing on it.

## pattern/precondition-disabled-tool

**Shape:** a tool/control is enabled only when its precondition holds; the status bar explains the
path to enabling it; the label stays stable.

**Where:** Push/Pull is `disabled` until a mass exists (`hasMass`, `app.tsx`).

**Use when:** any action that needs a prior object. Disable on the real precondition (not on taste),
keep `aria` state in sync, and say why in the status bar.

## pattern/status-bar-as-feedback

**Shape:** the single status line names the active state and the next action; it's the primary
feedback channel before any panel exists.

**Where:** `statusText(store)` in `app.tsx`.

**Use when:** you need to tell the user what's happening or what to do next. Add a status line before
adding chrome.

## pattern/imperative-renderer-in-react

**Shape:** a heavy imperative renderer (Three.js) lives in a ref-held handle, built once and updated
by effects; React owns no per-frame state; resources are disposed on rebuild and unmount.

**Where:** `ThreeView` / `SceneHandle` (`three-view.tsx`), per ADR 0005.

**Use when:** integrating any imperative canvas/GL surface. Don't lift per-frame state into React;
do dispose geometries/materials.

---

A new pattern earns a place here only after it has shipped and been verified — see governance in
`AGENTS.md`. Don't pre-register aspirational patterns; that's what `coverage-gaps.md` is for.
