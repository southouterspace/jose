/**
 * The plan view — the top-down (world XY), orthographic 2D drawing surface (ADR 0005 / CONTEXT.md).
 *
 * While the footprint tool is active, clicks add ring vertices and a click near the first closes the
 * ring, sending a `DrawFootprint` command into the worker. As the cursor moves it shows a live preview:
 * a rubber-band segment, a live **length + angle** readout (`hud.ts`), and — from `plan-snap.ts` — the
 * inference cues: **point snaps** (endpoint/midpoint/on-edge), **on-axis** inference, and **locks**
 * (Shift → the dominant axis; `→`/`↑` arrow keys → the X/Y axis). The value-entry box accepts an exact
 * length (feet/inches), optionally with a `< angle` clause for an absolute bearing (`10' 6" < 45`).
 *
 * When the snapshot returns, this renders the footprint polygon **from the `FootprintMirror`** — the
 * engine's canonical ring — never from the raw clicks — and labels each committed edge with its length
 * plus the overall width×depth. The mid-draw polyline (`pendingPicks`), the preview, and the guides are
 * transient UI only.
 *
 * The view is navigable (P0 #2): scroll zooms toward the cursor, middle-drag pans, and Fit / Shift+Z
 * runs Zoom-Extents. All of it flows through one `PlanCamera` (`plan-camera.ts`) held in state — the
 * pure world↔screen transform every coordinate below is derived from.
 *
 * With the Select tool active (P0 #3, ADR 0013) a click picks the ring piece under the cursor — a
 * vertex, an edge, or the whole footprint — via the pure screen-space `hitTest`; the cursor's hover
 * target is previewed, and the committed selection (store state) is highlighted on top.
 */

import type { FootprintMirror } from "@jose/render-mirror";
import type { AlignmentGuide, DraftPoint, Point } from "@jose/tool-runner";
import {
  formatLength,
  parsePolarLength,
  parseSize,
  pointAtAngle,
  pointAtDistance,
  rectangleCorner,
  rectangleRing,
} from "@jose/tool-runner";
import {
  type Dispatch,
  type PointerEvent,
  type ReactElement,
  type RefObject,
  type SetStateAction,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { EngineStore } from "./engine-store";
import { insertOnEdge, moveVertex } from "./footprint-edit";
import {
  edgeLabels,
  footprintExtents,
  formatExtents,
  segmentAngleDegrees,
  segmentReadout,
} from "./hud";
import {
  boundsOf,
  DEFAULT_CAMERA,
  fitToBounds,
  type PlanCamera,
  panBy,
  toScreenX,
  toScreenY,
  toWorldX,
  toWorldY,
  VIEW_H,
  VIEW_W,
  zoomAt,
} from "./plan-camera";
import {
  hitTest,
  type Px,
  type RingVertex,
  type Selection,
  sameSelection,
} from "./plan-selection";
import {
  type AxisGuide,
  type DrawLock,
  type LockAxis,
  resolveDraw,
  SNAP_LABEL,
  type Snap,
  type SnapKind,
} from "./plan-snap";
import { type SubmitModifiers, ValueBox } from "./value-box";

/** Grid line spacing in world ticks (384 = 1ft). */
const GRID_TICKS = 384;
/** Half-extent of the world the grid spans, in ticks. */
const GRID_HALF_TICKS = 7680;
/** Per-notch wheel zoom factor (in on scroll-up, out on scroll-down). */
const ZOOM_STEP = 1.1;
/** How far (viewBox px) a Select-tool press must travel before it counts as an edit *drag* rather
 *  than a *click* — the one gesture disambiguation behind "a click selects, a drag edits" (ADR 0015). */
const EDIT_DRAG_THRESHOLD_PX = 4;
/** Grid a dragged vertex snaps to when no inference snap applies, ticks (1in) — matches the draw grid. */
const EDIT_GRID_TICKS = 32;
/** No axis lock: the edit drag resolves point snaps only (endpoint / midpoint / on-edge). Shift/arrow
 *  axis-lock during an edit is a deferred refinement (ADR 0015). */
const EDIT_NO_LOCK: DrawLock = { axis: null, shift: false };

/** An in-progress footprint edit (Select tool): dragging a ring vertex (`move`) or a new vertex split
 *  onto an edge (`insert`). Tracked in a ref so pointer-move doesn't re-render per frame; `preview`
 *  holds the latest edited ring so the pointer-up commit reads it without a stale closure. */
interface EditDrag {
  readonly index: number;
  readonly kind: "insert" | "move";
  moved: boolean;
  readonly pointerId: number;
  preview: readonly Point[] | null;
  readonly startPx: Px;
}

/** Snap a dragged vertex onto the tick grid when no inference snap caught it (the draw path's fallback). */
const snapToEditGrid = (p: Point): Point => ({
  x: Math.round(p.x / EDIT_GRID_TICKS) * EDIT_GRID_TICKS,
  y: Math.round(p.y / EDIT_GRID_TICKS) * EDIT_GRID_TICKS,
});

/** Resolve a client pointer into the world point an edit drag would place: a `plan-snap` point snap
 *  (endpoint / midpoint / on-edge against `candidateRing` — the ring *without* the vertex being
 *  dragged, so it never snaps to itself) when one applies, else the grid-snapped point under the
 *  cursor. `null` only when the pointer can't be projected. */
function resolveEditPoint(
  camera: PlanCamera,
  svg: SVGSVGElement | null,
  candidateRing: readonly Point[],
  clientX: number,
  clientY: number
): { world: Point; snap: Snap | null } | null {
  const vb = viewBoxOf(svg, clientX, clientY);
  if (!vb) {
    return null;
  }
  const snap = resolveDraw(camera, candidateRing, [], vb, null, EDIT_NO_LOCK);
  if (snap) {
    return { world: snap.world, snap };
  }
  return {
    world: snapToEditGrid({
      x: toWorldX(camera, vb.px),
      y: toWorldY(camera, vb.py),
    }),
    snap: null,
  };
}

/** Continue an armed/active edit drag on a pointer-move: promote it to a real drag once it crosses
 *  the threshold, then resolve the moved/inserted vertex (snapping through plan-snap) and publish the
 *  transient preview ring + snap cue. Returns whether it consumed the event (so the caller skips hover
 *  / draw handling); `false` only when the pointer isn't the one that started the drag. Mirrors
 *  {@link continuePan}. */
function continueEditDrag(
  drag: EditDrag,
  camera: PlanCamera,
  ring: readonly Point[],
  svg: SVGSVGElement | null,
  event: PointerEvent<SVGSVGElement>,
  setPreview: Dispatch<SetStateAction<readonly Point[] | null>>,
  setSnap: Dispatch<SetStateAction<Snap | null>>
): boolean {
  if (event.pointerId !== drag.pointerId) {
    return false;
  }
  const vb = viewBoxOf(svg, event.clientX, event.clientY);
  if (!vb) {
    return true;
  }
  const travelled = Math.hypot(
    vb.px - drag.startPx.px,
    vb.py - drag.startPx.py
  );
  if (!drag.moved && travelled < EDIT_DRAG_THRESHOLD_PX) {
    return true; // still within click tolerance — not a drag yet.
  }
  drag.moved = true;
  // Move: snap against the ring *without* the dragged vertex. Insert: the new vertex isn't in the ring
  // yet, so the whole ring is a valid snap target.
  const candidate =
    drag.kind === "move" ? ring.filter((_, i) => i !== drag.index) : ring;
  const r = resolveEditPoint(
    camera,
    svg,
    candidate,
    event.clientX,
    event.clientY
  );
  if (!r) {
    return true;
  }
  const preview =
    drag.kind === "move"
      ? moveVertex(ring, drag.index, r.world)
      : insertOnEdge(ring, drag.index, r.world);
  drag.preview = preview;
  setPreview(preview);
  setSnap(r.snap);
  return true;
}

/** Arm an edit drag from a Select-tool press: a vertex/edge under the cursor returns the (not-yet-
 *  moved) [`EditDrag`] to track; the face/empty space returns `null` and the caller selects instead. */
function armEditDrag(
  camera: PlanCamera,
  svg: SVGSVGElement | null,
  footprint: FootprintMirror | null,
  event: PointerEvent<SVGSVGElement>
): EditDrag | null {
  const hit = pickAt(camera, svg, footprint, event.clientX, event.clientY);
  const vb = viewBoxOf(svg, event.clientX, event.clientY);
  if (vb && hit && (hit.kind === "vertex" || hit.kind === "edge")) {
    return {
      pointerId: event.pointerId,
      kind: hit.kind === "vertex" ? "move" : "insert",
      index: hit.index,
      startPx: vb,
      moved: false,
      preview: null,
    };
  }
  return null;
}

/** A highlight over a selected/hovered ring piece — a vertex dot, an edge line, or the whole face. */
function selectionCue(
  camera: PlanCamera,
  ringVertices: readonly RingVertex[],
  sel: Selection | null,
  variant: "hover" | "selected"
): ReactElement | null {
  if (!sel || ringVertices.length === 0) {
    return null;
  }
  const cls = variant === "selected" ? "plan__sel" : "plan__hover";
  const px = (v: RingVertex): number => toScreenX(camera, v.x);
  const py = (v: RingVertex): number => toScreenY(camera, v.y);
  if (sel.kind === "vertex") {
    const v = ringVertices[sel.index];
    return v ? (
      <circle
        className={`${cls} ${cls}--vertex`}
        cx={px(v)}
        cy={py(v)}
        r={variant === "selected" ? 7 : 6}
      />
    ) : null;
  }
  if (sel.kind === "edge") {
    const a = ringVertices[sel.index];
    const b = ringVertices[(sel.index + 1) % ringVertices.length];
    return a && b ? (
      <line
        className={`${cls} ${cls}--edge`}
        x1={px(a)}
        x2={px(b)}
        y1={py(a)}
        y2={py(b)}
      />
    ) : null;
  }
  return (
    <polygon
      className={`${cls} ${cls}--face`}
      points={ringVertices.map((v) => `${px(v)},${py(v)}`).join(" ")}
    />
  );
}

/** Resolve the plan value box's typed entry into the next vertex — a length along the cursor
 *  direction, or (with a `< angle` clause) a length at that absolute bearing. Returns the exact world
 *  point, or an `error` string to surface when the entry names no length. */
function resolveTypedVertex(
  anchor: Point,
  draft: DraftPoint,
  input: string,
  axisLock: boolean
): { point: Point } | { error: string } {
  const entry = parsePolarLength(input);
  if (entry === null) {
    return { error: "Enter a length like 10' 6\", or 10' 6\" < 45." };
  }
  const point =
    entry.angleDegrees === null
      ? pointAtDistance(anchor, draft.point, entry.lengthTicks, axisLock)
      : pointAtAngle(anchor, entry.lengthTicks, entry.angleDegrees);
  return { point };
}

/** Resolve the rectangle value box's typed `W,D` into the opposite corner (grown toward the cursor's
 *  quadrant), or an `error` string when the entry doesn't name a size. */
function resolveTypedRectangle(
  anchor: Point,
  draft: DraftPoint,
  input: string
): { point: Point } | { error: string } {
  const size = parseSize(input);
  if (size === null) {
    return { error: "Enter a size like 24', 16'." };
  }
  return {
    point: rectangleCorner(anchor, draft.point, size.width, size.depth),
  };
}

/** Resolve a client pointer into the point a draw pick would land on: a snapped point (committed
 *  `exact`) — a point snap, or an axis lock/inference through the `anchor` — when one applies, else the
 *  raw world point under the cursor. `null` only when the pointer can't be projected. */
function resolveDrawPoint(
  camera: PlanCamera,
  svg: SVGSVGElement | null,
  ring: readonly Point[],
  pending: readonly Point[],
  anchor: Point | null,
  lock: DrawLock,
  clientX: number,
  clientY: number
): { target: Point; exact: boolean; snap: Snap | null } | null {
  const vb = viewBoxOf(svg, clientX, clientY);
  if (!vb) {
    return null;
  }
  const snap = resolveDraw(camera, ring, pending, vb, anchor, lock);
  if (snap) {
    return { target: snap.world, exact: true, snap };
  }
  return {
    target: { x: toWorldX(camera, vb.px), y: toWorldY(camera, vb.py) },
    exact: false,
    snap: null,
  };
}

/** The snap marker glyph per kind: an endpoint square, a midpoint diamond, an on-edge ✕ — the shape
 *  (not just the color) carries the kind. `on-axis` has no point marker (its guide line + the cursor
 *  dot carry it), so it returns `null`. */
function snapMarker(kind: SnapKind, x: number, y: number): ReactElement | null {
  if (kind === "endpoint") {
    return (
      <rect
        className="plan__snap plan__snap--endpoint"
        height={10}
        width={10}
        x={x - 5}
        y={y - 5}
      />
    );
  }
  if (kind === "midpoint") {
    return (
      <rect
        className="plan__snap plan__snap--midpoint"
        height={10}
        transform={`rotate(45 ${x} ${y})`}
        width={10}
        x={x - 5}
        y={y - 5}
      />
    );
  }
  if (kind === "on-edge") {
    return (
      <g className="plan__snap plan__snap--edge">
        <line x1={x - 5} x2={x + 5} y1={y - 5} y2={y + 5} />
        <line x1={x - 5} x2={x + 5} y1={y + 5} y2={y - 5} />
      </g>
    );
  }
  return null;
}

/** The full-extent axis guide line for an `on-axis` snap: red for the X axis (horizontal), green for
 *  the Y axis (vertical), bold when hard-locked (Shift/arrow). */
function axisGuideCue(
  camera: PlanCamera,
  guide: AxisGuide | undefined
): ReactElement | null {
  if (!guide) {
    return null;
  }
  const cls = `plan__axis plan__axis--${guide.orientation === "horizontal" ? "x" : "y"}${guide.locked ? " plan__axis--locked" : ""}`;
  return guide.orientation === "horizontal" ? (
    <line
      className={cls}
      x1={0}
      x2={VIEW_W}
      y1={toScreenY(camera, guide.atTicks)}
      y2={toScreenY(camera, guide.atTicks)}
    />
  ) : (
    <line
      className={cls}
      x1={toScreenX(camera, guide.atTicks)}
      x2={toScreenX(camera, guide.atTicks)}
      y1={0}
      y2={VIEW_H}
    />
  );
}

/** The live snap cue: a colored marker (point kinds) plus a badge naming the inference; a hard lock
 *  reads "… — locked". */
function snapCue(camera: PlanCamera, snap: Snap | null): ReactElement | null {
  if (!snap) {
    return null;
  }
  const x = toScreenX(camera, snap.world.x);
  const y = toScreenY(camera, snap.world.y);
  const label = snap.guide?.locked
    ? `${SNAP_LABEL[snap.kind]} — locked`
    : SNAP_LABEL[snap.kind];
  return (
    <g>
      {snapMarker(snap.kind, x, y)}
      <text className="plan__snapbadge" x={x + 11} y={y - 22}>
        {label}
      </text>
    </g>
  );
}

/** The dashed alignment guides in play: a full-height/width line at each row/column the cursor shares
 *  with an existing vertex (the from-point inference). */
function guideCues(
  camera: PlanCamera,
  guides: readonly AlignmentGuide[]
): ReactElement[] {
  return guides.map((g) =>
    g.orientation === "vertical" ? (
      <line
        className="plan__guide"
        key={`gv${g.sourceIndex}`}
        x1={toScreenX(camera, g.atTicks)}
        x2={toScreenX(camera, g.atTicks)}
        y1={0}
        y2={VIEW_H}
      />
    ) : (
      <line
        className="plan__guide"
        key={`gh${g.sourceIndex}`}
        x1={0}
        x2={VIEW_W}
        y1={toScreenY(camera, g.atTicks)}
        y2={toScreenY(camera, g.atTicks)}
      />
    )
  );
}

/** The rectangle rubber-band: the axis-aligned box from the first corner (`anchor`) to the cursor. */
function rectanglePreviewCue(
  camera: PlanCamera,
  anchor: Point | null,
  cursor: Point | null
): ReactElement | null {
  if (!(anchor && cursor)) {
    return null;
  }
  const points = rectangleRing(anchor, cursor)
    .map((c) => `${toScreenX(camera, c.x)},${toScreenY(camera, c.y)}`)
    .join(" ");
  return <polygon className="plan__rubber-rect" fill="none" points={points} />;
}

/** The transient draw preview under the cursor (rendered only while a plan draw is hovering): the
 *  active tool's rubber-band (footprint segment or rectangle box), the ring-close target, the cursor
 *  dot, the length+angle readout, and the live snap cue. Kept a component so its per-element
 *  conditionals live here, not in `PlanView`'s body. */
function DrawPreview(props: {
  readonly anchor: Point | null;
  readonly camera: PlanCamera;
  readonly draft: DraftPoint;
  readonly firstPick: Point | null;
  readonly isFootprint: boolean;
  readonly isRectangle: boolean;
  readonly liveSegment: LiveSegment | null;
  readonly snap: Snap | null;
}) {
  const { anchor, camera, draft, firstPick, liveSegment } = props;
  const sx = (t: number): number => toScreenX(camera, t);
  const sy = (t: number): number => toScreenY(camera, t);
  const showDim =
    props.isFootprint &&
    !draft.closing &&
    liveSegment !== null &&
    liveSegment.lengthTicks > 0;
  return (
    <>
      {/* Axis guide line (on-axis inference / lock), under the geometry preview. */}
      {axisGuideCue(camera, props.snap?.guide)}

      {/* Footprint rubber-band from the last vertex to the cursor. */}
      {props.isFootprint && anchor && (
        <line
          className="plan__rubber"
          x1={sx(anchor.x)}
          x2={sx(draft.point.x)}
          y1={sy(anchor.y)}
          y2={sy(draft.point.y)}
        />
      )}

      {/* Rectangle rubber-band: the axis-aligned box from the first corner to the cursor. */}
      {props.isRectangle && rectanglePreviewCue(camera, anchor, draft.point)}

      {/* Close target: a click here closes the ring. */}
      {draft.closing && firstPick && (
        <g className="plan__close">
          <circle cx={sx(firstPick.x)} cy={sy(firstPick.y)} r={9} />
          <text x={sx(firstPick.x) + 13} y={sy(firstPick.y) - 9}>
            Close
          </text>
        </g>
      )}

      {/* Live cursor dot + the length+angle readout trailing it (footprint tool). */}
      {!draft.closing && (
        <circle
          className="plan__cursor"
          cx={sx(draft.point.x)}
          cy={sy(draft.point.y)}
          r={3.5}
        />
      )}
      {showDim && liveSegment && (
        <text
          className="plan__dim"
          x={sx(draft.point.x) + 10}
          y={sy(draft.point.y) - 10}
        >
          {segmentReadout(liveSegment.lengthTicks, liveSegment.angleDeg)}
        </text>
      )}

      {/* Live snap cue (endpoint / midpoint / on-edge): a colored marker + badge at the snap. */}
      {snapCue(camera, props.snap)}
    </>
  );
}

/** Persistent length labels on a committed footprint — one per edge, centered on the edge midpoint. */
function edgeLengthCues(
  camera: PlanCamera,
  ringVertices: readonly RingVertex[]
): ReactElement[] {
  return edgeLabels(ringVertices).map((label) => (
    <text
      className="plan__edgelen"
      key={`e${label.midX},${label.midY}`}
      x={toScreenX(camera, label.midX)}
      y={toScreenY(camera, label.midY)}
    >
      {formatLength(label.lengthTicks)}
    </text>
  ));
}

/** The transient edit preview: the edited ring (a vertex moved / one inserted), rendered in place of
 *  the canonical footprint while a Select-tool drag is live, with live edge-length labels and the snap
 *  cue. Client-only until release — only the committed ring crosses into the engine (ADR 0015). */
function EditPreview(props: {
  readonly camera: PlanCamera;
  readonly ring: readonly Point[];
  readonly snap: Snap | null;
}): ReactElement {
  const { camera, ring } = props;
  const verts: RingVertex[] = ring.map((p) => ({ x: p.x, y: p.y, spaceId: 0 }));
  const points = verts
    .map((v) => `${toScreenX(camera, v.x)},${toScreenY(camera, v.y)}`)
    .join(" ");
  return (
    <g>
      <polygon
        className="plan__footprint plan__footprint--editing"
        points={points}
      />
      {edgeLengthCues(camera, verts)}
      {snapCue(camera, props.snap)}
    </g>
  );
}

/** The canonical footprint layer: the committed ring polygon, its edge-length labels, and the
 *  selection/hover cues — *or*, while a Select-tool edit drag is live, the transient [`EditPreview`]
 *  in its place (so the canonical ring and its edit never double up). Keeps PlanView's render body
 *  free of the editing branch. */
function FootprintLayer(props: {
  readonly camera: PlanCamera;
  readonly editPreview: readonly Point[] | null;
  readonly footprint: FootprintMirror | null;
  readonly hasRing: boolean;
  readonly hoverCue: Selection | null;
  readonly ringVertices: readonly RingVertex[];
  readonly selection: Selection | null;
  readonly snap: Snap | null;
}): ReactElement {
  const { camera, editPreview, footprint, hasRing, ringVertices } = props;
  if (editPreview) {
    return <EditPreview camera={camera} ring={editPreview} snap={props.snap} />;
  }
  const points =
    hasRing && footprint
      ? footprint
          .vertices()
          .map((v) => `${toScreenX(camera, v.x)},${toScreenY(camera, v.y)}`)
          .join(" ")
      : "";
  return (
    <g>
      {hasRing && footprint && (
        <polygon className="plan__footprint" points={points} />
      )}
      {hasRing && edgeLengthCues(camera, ringVertices)}
      {selectionCue(camera, ringVertices, props.hoverCue, "hover")}
      {selectionCue(camera, ringVertices, props.selection, "selected")}
    </g>
  );
}

/** A client pointer position in the fixed viewBox pixel space (independent of the element's size). */
function viewBoxOf(
  svg: SVGSVGElement | null,
  clientX: number,
  clientY: number
): { px: number; py: number } | null {
  if (!svg) {
    return null;
  }
  const rect = svg.getBoundingClientRect();
  return {
    px: ((clientX - rect.left) / rect.width) * VIEW_W,
    py: ((clientY - rect.top) / rect.height) * VIEW_H,
  };
}

/** Scroll-to-zoom toward the cursor. A native, non-passive `wheel` listener is required to
 *  `preventDefault` the page scroll (React's synthetic wheel handler is passive); `setCamera`'s
 *  functional form reads the latest camera, so this attaches once. */
function useWheelZoom(
  svgRef: RefObject<SVGSVGElement | null>,
  setCamera: Dispatch<SetStateAction<PlanCamera>>
): void {
  useEffect(() => {
    const svg = svgRef.current;
    if (!svg) {
      return;
    }
    const onWheel = (event: WheelEvent): void => {
      event.preventDefault();
      const rect = svg.getBoundingClientRect();
      const px = ((event.clientX - rect.left) / rect.width) * VIEW_W;
      const py = ((event.clientY - rect.top) / rect.height) * VIEW_H;
      const factor = event.deltaY < 0 ? ZOOM_STEP : 1 / ZOOM_STEP;
      setCamera((cam) => zoomAt(cam, px, py, factor));
    };
    svg.addEventListener("wheel", onWheel, { passive: false });
    return () => svg.removeEventListener("wheel", onWheel);
  }, [svgRef, setCamera]);
}

/** `Shift+Z` runs Zoom-Extents (SketchUp's), reading the latest fit via a ref so it attaches once;
 *  skipped while typing so it never hijacks the value box. */
function useZoomExtentsHotkey(fitRef: RefObject<() => void>): void {
  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if (event.ctrlKey || event.metaKey || event.altKey) {
        return;
      }
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.isContentEditable)) {
        return;
      }
      if (event.shiftKey && event.key.toLowerCase() === "z") {
        event.preventDefault();
        fitRef.current?.();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [fitRef]);
}

/** Arrow-key axis lock while the footprint tool is active: → toggles the X (red) axis, ↑ the Y (green)
 *  axis, ← / ↓ release. Skipped while typing so it never fights the value box. */
function useAxisLockHotkeys(
  enabled: boolean,
  setAxisLock: Dispatch<SetStateAction<LockAxis>>
): void {
  useEffect(() => {
    if (!enabled) {
      return;
    }
    const onKey = (event: KeyboardEvent): void => {
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.isContentEditable)) {
        return;
      }
      if (event.key === "ArrowRight") {
        event.preventDefault();
        setAxisLock((a) => (a === "x" ? null : "x"));
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        setAxisLock((a) => (a === "y" ? null : "y"));
      } else if (event.key === "ArrowLeft" || event.key === "ArrowDown") {
        event.preventDefault();
        setAxisLock(null);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [enabled, setAxisLock]);
}

/** An active middle-drag pan, tracked in a ref so pointer-move doesn't re-render per frame. */
interface PanState {
  lastPx: number;
  lastPy: number;
  pointerId: number;
}

/** Continue an active middle-drag pan on a pointer-move; returns whether the pan consumed the event
 *  (so the caller skips tool handling). */
function continuePan(
  pan: PanState | null,
  svg: SVGSVGElement | null,
  event: PointerEvent<SVGSVGElement>,
  setCamera: Dispatch<SetStateAction<PlanCamera>>
): boolean {
  if (!(pan && event.pointerId === pan.pointerId)) {
    return false;
  }
  const vb = viewBoxOf(svg, event.clientX, event.clientY);
  if (vb) {
    setCamera((cam) => panBy(cam, vb.px - pan.lastPx, vb.py - pan.lastPy));
    pan.lastPx = vb.px;
    pan.lastPy = vb.py;
  }
  return true;
}

/** Start a middle-drag pan on a pointer-down: capture the pointer and return the new pan state, or
 *  `null` when the pointer can't be projected. */
function startPan(
  svg: SVGSVGElement | null,
  event: PointerEvent<SVGSVGElement>
): PanState | null {
  const vb = viewBoxOf(svg, event.clientX, event.clientY);
  if (!vb) {
    return null;
  }
  svg?.setPointerCapture(event.pointerId);
  return { pointerId: event.pointerId, lastPx: vb.px, lastPy: vb.py };
}

/** The ring piece a client pointer position would select (select tool), or `null`. */
function pickAt(
  camera: PlanCamera,
  svg: SVGSVGElement | null,
  footprint: FootprintMirror | null,
  clientX: number,
  clientY: number
): Selection | null {
  const vb = viewBoxOf(svg, clientX, clientY);
  return vb ? hitTest(camera, footprint?.vertices() ?? [], vb) : null;
}

/** The committed ring's vertices for drawing cues, or empty until a closed footprint exists. */
function ringOf(footprint: FootprintMirror | null): readonly RingVertex[] {
  return footprint && footprint.count >= 3 ? footprint.vertices() : [];
}

/** The hover cue to paint: only in select mode, and only when it isn't just echoing the selection. */
function visibleHover(
  isSelect: boolean,
  hover: Selection | null,
  selection: Selection | null
): Selection | null {
  return isSelect && !sameSelection(hover, selection) ? hover : null;
}

/** The cursor for the plan surface: grabbing while panning or dragging an edit, a pointer over a
 *  draggable ring piece in select mode, else the CSS crosshair (`undefined` defers to the stylesheet). */
function surfaceCursor(
  panning: boolean,
  isSelect: boolean,
  hasHover: boolean,
  editing: boolean
): string | undefined {
  if (panning || editing) {
    return "grabbing";
  }
  if (isSelect) {
    return hasHover ? "pointer" : "default";
  }
  return;
}

/** The live segment (anchor → cursor) while drawing a footprint: length (ticks) and bearing (degrees). */
interface LiveSegment {
  readonly angleDeg: number;
  readonly lengthTicks: number;
}

/** The value box's placeholder text for the active grammar: the live W×D for a rectangle, the live
 *  length+angle for a footprint segment, or a typed-format example before there's a live value. */
function valuePlaceholder(
  isRectangle: boolean,
  extents: { width: number; depth: number } | null,
  liveSegment: LiveSegment | null
): string {
  if (isRectangle) {
    return extents
      ? formatExtents(extents.width, extents.depth)
      : `e.g. 24', 16'`;
  }
  return liveSegment && liveSegment.lengthTicks > 0
    ? segmentReadout(liveSegment.lengthTicks, liveSegment.angleDeg)
    : `e.g. 10' 6" < 45`;
}

/** The plan value box (SketchUp's VCB) with grammar-aware chrome: **Size** (`W,D`) for the rectangle
 *  tool, **Length** (with optional `< angle`) for the footprint tool. Keeps the grammar branching out
 *  of `PlanView`'s body. */
function PlanValueBox(props: {
  readonly disabled: boolean;
  readonly extents: { width: number; depth: number } | null;
  readonly isRectangle: boolean;
  readonly liveSegment: LiveSegment | null;
  readonly onCancel: () => void;
  readonly onChange: (value: string) => void;
  readonly onSubmit: (modifiers: SubmitModifiers) => void;
  readonly value: string;
}) {
  const { isRectangle } = props;
  return (
    <ValueBox
      ariaLabel={
        isRectangle
          ? "Rectangle width and depth in feet and inches, e.g. 24', 16'"
          : 'Segment length in feet and inches — add "< angle" for a bearing'
      }
      disabled={props.disabled}
      label={isRectangle ? "Size" : "Length"}
      onCancel={props.onCancel}
      onChange={props.onChange}
      onSubmit={props.onSubmit}
      placeholder={valuePlaceholder(
        isRectangle,
        props.extents,
        props.liveSegment
      )}
      value={props.value}
    />
  );
}

export interface PlanViewProps {
  readonly store: EngineStore;
}

export function PlanView({ store }: PlanViewProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [draft, setDraft] = useState<DraftPoint | null>(null);
  const [hovering, setHovering] = useState(false);
  const [lengthInput, setLengthInput] = useState("");
  const [camera, setCamera] = useState<PlanCamera>(DEFAULT_CAMERA);
  const [panning, setPanning] = useState(false);
  /** What a click would select right now (select tool only) — ephemeral, view-local hover cue. */
  const [hover, setHover] = useState<Selection | null>(null);
  /** The live snap the cursor resolved to while drawing (point / on-axis), or `null`. */
  const [snap, setSnap] = useState<Snap | null>(null);
  /** The arrow-key axis lock (footprint tool): constrains every pick to the world X or Y axis. */
  const [axisLock, setAxisLock] = useState<LockAxis>(null);
  /** Active middle-drag pan, tracked in a ref so pointer-move doesn't re-render per frame. */
  const panRef = useRef<PanState | null>(null);
  /** An armed/active footprint edit drag (Select tool), tracked in a ref like the pan. */
  const dragRef = useRef<EditDrag | null>(null);
  /** The transient edited ring while a Select-tool drag is live (null when not editing) — rendered in
   *  place of the canonical footprint; only committed on release. */
  const [editPreview, setEditPreview] = useState<readonly Point[] | null>(null);

  const isFootprint = store.activeTool === "footprint";
  const isRectangle = store.activeTool === "rectangle";
  const isSelect = store.activeTool === "select";
  /** Both plan draw tools collect world picks the same way; only the preview + value grammar differ. */
  const isPlanDraw = isFootprint || isRectangle;
  /** Committed ring vertices (world ticks) the snap engine tests against; empty until the first close. */
  const ringWorld: readonly Point[] = store.footprint?.vertices() ?? [];

  /** Local screen↔world binds over the current camera, so the JSX below reads plainly. */
  const sx = (xTicks: number): number => toScreenX(camera, xTicks);
  const sy = (yTicks: number): number => toScreenY(camera, yTicks);

  /** Zoom-Extents: frame the committed footprint and any in-progress picks (Shift+Z / the Fit button). */
  const fitToContent = (): void => {
    const points: { x: number; y: number }[] = store.pendingPicks.map((p) => ({
      x: p.x,
      y: p.y,
    }));
    if (store.footprint) {
      for (const v of store.footprint.vertices()) {
        points.push({ x: v.x, y: v.y });
      }
    }
    setCamera(fitToBounds(boundsOf(points)));
  };

  useWheelZoom(svgRef, setCamera);
  useAxisLockHotkeys(isFootprint, setAxisLock);

  // Release the axis lock whenever there's no in-progress draw (a fresh start owns its own lock).
  const drawInProgress = store.pendingPicks.length > 0;
  useEffect(() => {
    if (!drawInProgress) {
      setAxisLock(null);
    }
  }, [drawInProgress]);

  // A ref keeps the latest Zoom-Extents in reach so the hotkey attaches once.
  const fitRef = useRef(fitToContent);
  fitRef.current = fitToContent;
  useZoomExtentsHotkey(fitRef);

  const onPointerMove = (event: PointerEvent<SVGSVGElement>): void => {
    if (continuePan(panRef.current, svgRef.current, event, setCamera)) {
      return;
    }
    // An armed edit drag (Select tool) owns the pointer until release — move the vertex / place the
    // inserted one, never falling through to hover.
    const drag = dragRef.current;
    if (
      drag &&
      continueEditDrag(
        drag,
        camera,
        ringWorld,
        svgRef.current,
        event,
        setEditPreview,
        setSnap
      )
    ) {
      return;
    }
    // Select mode: hover-highlight whatever a click would pick (vertex/edge/footprint).
    if (isSelect) {
      const next = pickAt(
        camera,
        svgRef.current,
        store.footprint,
        event.clientX,
        event.clientY
      );
      setHover((prev) => (sameSelection(prev, next) ? prev : next));
      return;
    }
    if (!isPlanDraw) {
      return;
    }
    // Axis lock / on-axis inference are footprint gestures (a rectangle is already axis-aligned), so
    // only the footprint tool passes an anchor + lock; both tools still get point snaps.
    const drawAnchor = isFootprint ? (store.pendingPicks.at(-1) ?? null) : null;
    const lock: DrawLock = {
      axis: axisLock,
      shift: isFootprint && event.shiftKey,
    };
    const r = resolveDrawPoint(
      camera,
      svgRef.current,
      ringWorld,
      store.pendingPicks,
      drawAnchor,
      lock,
      event.clientX,
      event.clientY
    );
    if (!r) {
      return;
    }
    setHovering(true);
    setSnap(r.snap);
    // A resolved snap commits exactly; the runner's own grid/axis handling is bypassed here.
    setDraft(store.draft(r.target, { exact: r.exact }));
  };

  const onPointerLeave = (): void => {
    setHovering(false);
    setHover(null);
    setSnap(null);
  };

  const endPan = (event: PointerEvent<SVGSVGElement>): void => {
    const pan = panRef.current;
    if (pan && event.pointerId === pan.pointerId) {
      svgRef.current?.releasePointerCapture(event.pointerId);
      panRef.current = null;
      setPanning(false);
    }
  };

  /** Finish an active edit gesture on pointer-up: a real drag commits the edited ring; a press that
   *  never moved was a *click*, so it selects the piece under it (the ADR 0013 behavior). Returns
   *  whether it consumed the event. */
  const endDrag = (event: PointerEvent<SVGSVGElement>): boolean => {
    const drag = dragRef.current;
    if (!(drag && event.pointerId === drag.pointerId)) {
      return false;
    }
    svgRef.current?.releasePointerCapture(event.pointerId);
    dragRef.current = null;
    if (drag.moved && drag.preview) {
      store.editFootprint(drag.preview);
    } else {
      store.select(
        drag.kind === "move"
          ? { kind: "vertex", index: drag.index }
          : { kind: "edge", index: drag.index }
      );
    }
    setEditPreview(null);
    setSnap(null);
    return true;
  };

  /** Abort an active edit gesture (pointer cancel) without committing. */
  const cancelDrag = (event: PointerEvent<SVGSVGElement>): boolean => {
    const drag = dragRef.current;
    if (!(drag && event.pointerId === drag.pointerId)) {
      return false;
    }
    svgRef.current?.releasePointerCapture(event.pointerId);
    dragRef.current = null;
    setEditPreview(null);
    setSnap(null);
    return true;
  };

  const onPointerUp = (event: PointerEvent<SVGSVGElement>): void => {
    if (endDrag(event)) {
      return;
    }
    endPan(event);
  };

  const onPointerCancel = (event: PointerEvent<SVGSVGElement>): void => {
    if (cancelDrag(event)) {
      return;
    }
    endPan(event);
  };

  const onPointerDown = (event: PointerEvent<SVGSVGElement>): void => {
    // Middle-button drag pans, in any tool (SketchUp parity) — it never places geometry.
    if (event.button === 1) {
      event.preventDefault();
      const pan = startPan(svgRef.current, event);
      if (pan) {
        panRef.current = pan;
        setPanning(true);
      }
      return;
    }
    if (event.button !== 0) {
      return; // Only the primary button acts from here.
    }
    // Select mode: a press on a vertex/edge *arms* an edit drag — a click (no drag) still selects it,
    // an actual drag moves the vertex or inserts one on the edge (ADR 0015). A press on the face/empty
    // space selects (or clears) immediately; there is no whole-footprint drag here (that's P2 #10).
    if (isSelect) {
      const armed = armEditDrag(camera, svgRef.current, store.footprint, event);
      if (armed) {
        svgRef.current?.setPointerCapture(event.pointerId);
        dragRef.current = armed;
      } else {
        // Face / empty space: select (or clear) immediately — no edit gesture there (whole-footprint
        // move is P2 #10).
        store.select(
          pickAt(
            camera,
            svgRef.current,
            store.footprint,
            event.clientX,
            event.clientY
          )
        );
      }
      return;
    }
    if (!isPlanDraw) {
      return;
    }
    const drawAnchor = isFootprint ? (store.pendingPicks.at(-1) ?? null) : null;
    const lock: DrawLock = {
      axis: axisLock,
      shift: isFootprint && event.shiftKey,
    };
    const r = resolveDrawPoint(
      camera,
      svgRef.current,
      ringWorld,
      store.pendingPicks,
      drawAnchor,
      lock,
      event.clientX,
      event.clientY
    );
    if (!r) {
      return;
    }
    store.pick(r.target, { exact: r.exact });
    setLengthInput("");
    setSnap(r.snap);
    setDraft(store.draft(r.target, { exact: r.exact }));
  };

  /** Grid lines follow the camera; recomputed only when it pans or zooms. */
  const gridLines = useMemo(() => {
    const lines: {
      key: string;
      x1: number;
      y1: number;
      x2: number;
      y2: number;
    }[] = [];
    for (let t = -GRID_HALF_TICKS; t <= GRID_HALF_TICKS; t += GRID_TICKS) {
      lines.push({
        key: `v${t}`,
        x1: toScreenX(camera, t),
        y1: toScreenY(camera, -GRID_HALF_TICKS),
        x2: toScreenX(camera, t),
        y2: toScreenY(camera, GRID_HALF_TICKS),
      });
      lines.push({
        key: `h${t}`,
        x1: toScreenX(camera, -GRID_HALF_TICKS),
        y1: toScreenY(camera, t),
        x2: toScreenX(camera, GRID_HALF_TICKS),
        y2: toScreenY(camera, t),
      });
    }
    return lines;
  }, [camera]);

  /** The previous vertex the next segment grows from — the anchor for value entry and the rubber band. */
  const anchor = store.pendingPicks.at(-1) ?? null;
  /** The live segment (anchor → cursor) while drawing: its length (ticks) and bearing (degrees CCW
   *  from +X) — what the readout shows and the value box's polar placeholder echoes. */
  const liveSegment =
    anchor && draft
      ? {
          lengthTicks: Math.round(
            Math.hypot(draft.point.x - anchor.x, draft.point.y - anchor.y)
          ),
          angleDeg: segmentAngleDegrees(anchor, draft.point),
        }
      : null;

  /** Commit the typed value box for the active grammar — a footprint vertex (length / `length < angle`)
   *  or a rectangle's opposite corner (`W,D`); flags an unparseable entry rather than swallowing it. */
  const commitValue = (axisLock: boolean): void => {
    if (!(anchor && draft)) {
      return;
    }
    const result = isRectangle
      ? resolveTypedRectangle(anchor, draft, lengthInput)
      : resolveTypedVertex(anchor, draft, lengthInput, axisLock);
    if ("error" in result) {
      store.flagRejection(result.error);
      return;
    }
    store.pick(result.point, { exact: true });
    setLengthInput("");
    setDraft(null);
  };

  const footprint = store.footprint;
  const hasRing = footprint !== null && footprint.count >= 3;
  const showPreview = isPlanDraw && hovering && draft !== null;
  // The value box is live only once a draw has an anchor (a first corner / vertex down).
  const canEnterValue = isPlanDraw && anchor !== null;

  const ringVertices = ringOf(footprint);
  const hoverCue = visibleHover(isSelect, hover, store.selection);
  /** A Select-tool edit drag is live: render the transient edited ring in place of the canonical one. */
  const isEditing = editPreview !== null;
  const cursor = surfaceCursor(panning, isSelect, hoverCue !== null, isEditing);

  // Running width×depth: the in-progress picks + cursor while drawing, else the committed ring.
  const extentsPoints: readonly Point[] =
    showPreview && draft ? [...store.pendingPicks, draft.point] : ringVertices;
  const extents = footprintExtents(extentsPoints);

  return (
    <div className="plan__wrap">
      <svg
        aria-label="Plan drawing surface"
        className="plan"
        onPointerCancel={onPointerCancel}
        onPointerDown={onPointerDown}
        onPointerLeave={onPointerLeave}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        ref={svgRef}
        style={cursor ? { cursor } : undefined}
        viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
      >
        <title>Plan drawing surface</title>
        <g className="plan__grid">
          {gridLines.map((l) => (
            <line key={l.key} x1={l.x1} x2={l.x2} y1={l.y1} y2={l.y2} />
          ))}
        </g>

        {/* Dashed alignment guides: the cursor shares an existing vertex's row/column. */}
        {showPreview && guideCues(camera, draft.guides)}

        {/* Mid-draw polyline: transient UI, not canonical geometry. */}
        {store.pendingPicks.length > 0 && (
          <polyline
            className="plan__pending"
            fill="none"
            points={store.pendingPicks
              .map((p) => `${sx(p.x)},${sy(p.y)}`)
              .join(" ")}
          />
        )}

        {store.pendingPicks.map((p) => (
          <circle
            className="plan__vertex"
            cx={sx(p.x)}
            cy={sy(p.y)}
            key={`${p.x},${p.y}`}
            r={4}
          />
        ))}

        {/* The transient draw preview under the cursor (rubber-band, close target, readout, snap). */}
        {showPreview && (
          <DrawPreview
            anchor={anchor}
            camera={camera}
            draft={draft}
            firstPick={store.pendingPicks[0] ?? null}
            isFootprint={isFootprint}
            isRectangle={isRectangle}
            liveSegment={liveSegment}
            snap={snap}
          />
        )}

        {/* The canonical footprint + selection cues — or, mid-edit, the transient preview ring. */}
        <FootprintLayer
          camera={camera}
          editPreview={editPreview}
          footprint={footprint}
          hasRing={hasRing}
          hoverCue={hoverCue}
          ringVertices={ringVertices}
          selection={store.selection}
          snap={snap}
        />

        {/* Running overall size (width × depth), pinned to the corner so it never trails the cursor. */}
        {extents && (
          <text className="plan__extents" x={12} y={24}>
            {formatExtents(extents.width, extents.depth)}
          </text>
        )}
      </svg>

      {/* Plan navigation: scroll zooms, middle-drag pans, this fits the drawing to the view. */}
      <div className="plan__nav">
        <button
          aria-keyshortcuts="Shift+Z"
          className="plan__navbtn"
          onClick={fitToContent}
          title="Zoom to fit (Shift+Z)"
          type="button"
        >
          Fit
        </button>
      </div>

      {/* Value-entry box (shared VCB, ADR 0012 §4): grammar-aware — a footprint **Length** (with an
          optional `< angle` bearing; Shift+Enter axis-locks) or a rectangle **Size** (`W,D`). */}
      <PlanValueBox
        disabled={!canEnterValue}
        extents={extents}
        isRectangle={isRectangle}
        liveSegment={liveSegment}
        onCancel={() => {
          setLengthInput("");
          store.cancelDraw();
          setDraft(null);
        }}
        onChange={setLengthInput}
        onSubmit={({ shiftKey }) => commitValue(shiftKey)}
        value={lengthInput}
      />
    </div>
  );
}
