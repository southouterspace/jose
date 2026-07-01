# Plan — Selection model (P0 #3)

The precondition for every editing verb (P2) and any future properties panel: **click to select a
footprint, a vertex, or an edge in the plan view; hover shows what a click would pick; Esc (or an
empty click) clears.** This is the third P0 "table stakes" item from
[`docs/analysis/sketchup-tools-analysis.md`](../analysis/sketchup-tools-analysis.md) §3; the load-bearing
call (where selection lives, how it's keyed) is recorded in
[ADR 0013](../adr/0013-selection-model.md).

## What we're building (and not)

**In:** a **Select** tool (the framework's third, per ADR 0012), plan-view hit-testing that resolves a
cursor to a vertex → edge → footprint (in that priority), a persistent **selection** in the store, a
**hover** affordance while the tool is active, and `Esc` / empty-click to clear. Selection cues render
over the canonical footprint.

**Out (deferred):** 3D selection (the mass / faces — selection is store-level so it extends there next),
window/crossing box-select, multi-select, and any *edit* the selection enables (vertex drag, move/copy,
delete — those are P2 and depend on this).

## Decisions this plan rests on

- **Selection is presentation state, not domain state** — it lives in the store (React), never in the
  engine, exactly as `activeTool` does. The engine holds no selection. ([ADR 0013](../adr/0013-selection-model.md).)
- **Referenced by ring index + kind**, resolved against the current `FootprintMirror`, and **cleared on
  every recompute** so a stale index can never dangle. ([ADR 0013](../adr/0013-selection-model.md).)
- **Hit-testing is screen-space** (viewBox px, over the `PlanCamera`), so tolerances are constant
  regardless of zoom — reusing the camera from P0 #2.
- **Select is a non-runner tool-chrome entry** (like `pushpull`): it emits no `Command`, so it never
  reaches `ToolRunner`; the store's `activate` already routes non-catalog keys straight to UI state.

## Phases (each keeps `main` green)

1. **Pure hit-test module (`plan-selection.ts`).** The `Selection` union + `hitTest(camera, vertices,
   screenPoint)` with priority vertex → edge → face, plus `pointInPolygon` / `distToSegment`. No React,
   no DOM — unit-tested directly (mirrors `plan-camera.ts`).
2. **Store: the selection state.** `selection`, `select(sel)`, `clearSelection()`; cleared whenever a
   `space` snapshot arrives (geometry changed).
3. **Tool-chrome: the Select tool.** A registry row (`key: "select"`, shortcut `s`, plan surface,
   `value: "none"`); `ChromeState` gains a `selectedKind` so the status bar reads the selection. Parity
   test unaffected (Select isn't runner-backed).
4. **Plan view: interactions + cues.** In select mode a primary click hit-tests → `store.select`; pointer
   move sets a local `hover`; selection and hover render as highlighted vertex / edge / face over the
   canonical ring. Middle-drag pan (P0 #2) still wins first.
5. **App: Esc clears.** The global key handler clears selection on `Escape` when not typing (the value
   box keeps its own Escape-cancels-entry grammar).

## Smaller engineering calls (mine to make)

- **Priority vertex > edge > face**, with a larger vertex tolerance than edge, so picking a corner is
  forgiving and a click inside the ring falls through to the face.
- **Hover is view-local, selection is store-level.** Hover is ephemeral per-pointer and only meaningful
  on the plan surface; selection is shared (status bar today, properties panel / 3D later).
- **Selection persists across tool switches** (SketchUp: select, then pick an edit tool) but **not across
  a recompute** — clearing on snapshot is the safe MVP rule until edits re-derive it.
- **`Esc` clears selection globally; the value box still owns `Esc` while focused** (cancel entry), so the
  two never fight — the global handler skips typing targets.
