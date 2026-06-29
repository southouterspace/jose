# Exemplar — the space-first drawing-UX MVP

Decisions worth repeating (and the traps avoided) from the drawing-UX MVP that established the
surface: PR #7 (the space-first MVP + architecture docs) and the phases recorded in
[`docs/plans/drawing-ux-mvp.md`](../../../docs/plans/drawing-ux-mvp.md), built on
[ADR 0007](../../../docs/adr/0007-space-first-modeling-footprint-push-pull.md) and
[ADR 0008](../../../docs/adr/0008-mvp-geometry-and-command-contract.md).

**Exemplar, not law.** This shows decisions that were made and why; verify each still matches the
current code before leaning on it. A flaw an exemplar reproduced is a thing to *improve*, not copy.

## Decisions worth repeating

- **Validated the feel before the architecture.** A throwaway prototype (Vite + React + vanilla
  Three, geometry mocked) proved the draw→push/pull loop and the two-input-surface model *before* the
  plan was written. Its only lasting output is the confidence behind ADR 0008's contract; it was not
  committed and deliberately broke the one-direction rule for speed. **Repeat:** spike the feel
  cheaply, then throw the spike away — don't promote a rule-breaking prototype to architecture.
- **Both panes as input surfaces, captured as an ADR.** Making the 3D view interactive *revised* an
  earlier "3D is read-only" decision; that reversal was recorded in ADR 0007 §2 with its rationale,
  not done silently. **Repeat:** when a design decision overturns a prior one, record the reversal and
  why.
- **Transient vs. canonical, drawn and coded distinctly.** The in-progress footprint is client-only
  (dashed); only the closed ring becomes a command; the result renders from the mirror (solid). This
  is `pattern/transient-then-canonical` and it kept the one-direction rule intact through an
  interactive draw loop. **Repeat:** for any multi-step gesture.
- **Named-face picking over a normal guess.** Push/pull targets the kernel's `TOP_FACE` via a
  dedicated `top-cap` mesh, with the vertical-normal check only as a guard (ADR 0008 §3). **Repeat:**
  `pattern/named-face-picking`.
- **Preserve the user's camera on a non-spatial change.** A height-only push/pull doesn't re-frame
  the camera (the user may be mid-orbit); re-framing is gated on a footprint signature. **Repeat:**
  `pattern/preserve-camera-on-nonspatial-change`.
- **Status bar carries the state; controls stay stable.** Feedback lives in one status line; tool
  labels don't mutate. **Repeat:** `pattern/status-bar-as-feedback`, `rule/stable-control-label`.
- **Display feet, compute in ticks.** `heightFeet` converts at the boundary. **Repeat:**
  `rule/display-feet-not-ticks`.

## Mistakes to avoid (gaps this slice shipped with)

These are *not* precedent — they're the known debt of a first slice. Don't reproduce them as if
intended:

- **No error/empty-of-failure state.** The slice handles only "loading" and the happy path; a worker
  or command failure has no treatment. Don't model a new command path on this silence — design its
  failure (`resilience.md`).
- **No responsive breakpoint.** The two-pane grid crushes when narrow. Don't copy the fixed `1fr 1fr`
  into a context that must go compact.
- **No pan/zoom, no snapping in plan.** Picks land raw; off-view geometry is unreachable. Don't assume
  the user can always see or precisely place what they drew.
- **No undo.** A misdraw costs a redraw. Don't add a destructive action on top of this without
  reading `coverage-gaps.md`.

## How to use this exemplar

When you extend the drawing UX, mine the **patterns** above for the shape of a good solution, and
check your change against the **mistakes** so you improve the debt rather than spread it. Cite the
exemplar and the specific pattern/rule when it informs a decision.
