/**
 * The plan view — the top-down (world XY), orthographic 2D drawing surface (ADR 0005 / CONTEXT.md).
 *
 * While the footprint tool is active, clicks add ring vertices and a click near the first closes the
 * ring, sending a `DrawFootprint` command into the worker. Holding Shift while clicking locks the new
 * edge to the X or Y axis (orthogonal drawing). As the cursor moves it shows a live preview: a
 * rubber-band segment, dashed alignment guides to existing vertices, a live **length + angle** readout
 * (`hud.ts`), and a highlighted close target when a click would close the ring. The value-entry box
 * accepts an exact length (feet/inches), optionally with a `< angle` clause for an absolute bearing
 * (`10' 6" < 45`), to place the next vertex precisely.
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
  type RingVertex,
  type Selection,
  sameSelection,
} from "./plan-selection";
import { resolveSnap, SNAP_LABEL, type Snap, type SnapKind } from "./plan-snap";
import { type SubmitModifiers, ValueBox } from "./value-box";

/** Grid line spacing in world ticks (384 = 1ft). */
const GRID_TICKS = 384;
/** Half-extent of the world the grid spans, in ticks. */
const GRID_HALF_TICKS = 7680;
/** Per-notch wheel zoom factor (in on scroll-up, out on scroll-down). */
const ZOOM_STEP = 1.1;

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

/** Resolve a client pointer into the point a draw pick would land on: a snapped point on existing
 *  geometry (committed `exact`) when the cursor is within a snap's screen tolerance, else the raw
 *  world point under the cursor. `null` only when the pointer can't be projected. */
function resolveDrawPoint(
  camera: PlanCamera,
  svg: SVGSVGElement | null,
  ring: readonly Point[],
  pending: readonly Point[],
  clientX: number,
  clientY: number
): { target: Point; exact: boolean; snap: Snap | null } | null {
  const vb = viewBoxOf(svg, clientX, clientY);
  if (!vb) {
    return null;
  }
  const snap = resolveSnap(camera, ring, pending, vb);
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
 *  (not just the color) carries the kind, so the cue reads without relying on hue. */
function snapMarker(kind: SnapKind, x: number, y: number): ReactElement {
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
  return (
    <g className="plan__snap plan__snap--edge">
      <line x1={x - 5} x2={x + 5} y1={y - 5} y2={y + 5} />
      <line x1={x - 5} x2={x + 5} y1={y + 5} y2={y - 5} />
    </g>
  );
}

/** The live snap cue: a colored marker at the snapped point plus a badge naming the inference. */
function snapCue(camera: PlanCamera, snap: Snap | null): ReactElement | null {
  if (!snap) {
    return null;
  }
  const x = toScreenX(camera, snap.world.x);
  const y = toScreenY(camera, snap.world.y);
  return (
    <g>
      {snapMarker(snap.kind, x, y)}
      <text className="plan__snapbadge" x={x + 11} y={y - 22}>
        {SNAP_LABEL[snap.kind]}
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

/** The cursor for the plan surface: grabbing while panning, an arrow/pointer in select mode, else the
 *  CSS crosshair (`undefined` defers to the stylesheet). */
function surfaceCursor(
  panning: boolean,
  isSelect: boolean,
  hasHover: boolean
): string | undefined {
  if (panning) {
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
  /** The live snap the cursor resolved to while drawing (endpoint/midpoint/on-edge), or `null`. */
  const [snap, setSnap] = useState<Snap | null>(null);
  /** Active middle-drag pan, tracked in a ref so pointer-move doesn't re-render per frame. */
  const panRef = useRef<PanState | null>(null);

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

  // A ref keeps the latest Zoom-Extents in reach so the hotkey attaches once.
  const fitRef = useRef(fitToContent);
  fitRef.current = fitToContent;
  useZoomExtentsHotkey(fitRef);

  const onPointerMove = (event: PointerEvent<SVGSVGElement>): void => {
    if (continuePan(panRef.current, svgRef.current, event, setCamera)) {
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
    const r = resolveDrawPoint(
      camera,
      svgRef.current,
      ringWorld,
      store.pendingPicks,
      event.clientX,
      event.clientY
    );
    if (!r) {
      return;
    }
    setHovering(true);
    setSnap(r.snap);
    // A snapped point commits exactly (overriding grid + axis lock). Axis lock is a footprint gesture;
    // a rectangle is already axis-aligned.
    setDraft(
      store.draft(r.target, {
        axisLock: isFootprint && event.shiftKey && !r.exact,
        exact: r.exact,
      })
    );
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
    // Select mode: a click picks whatever is under the cursor, or clears on empty space.
    if (isSelect) {
      store.select(
        pickAt(
          camera,
          svgRef.current,
          store.footprint,
          event.clientX,
          event.clientY
        )
      );
      return;
    }
    if (!isPlanDraw) {
      return;
    }
    const r = resolveDrawPoint(
      camera,
      svgRef.current,
      ringWorld,
      store.pendingPicks,
      event.clientX,
      event.clientY
    );
    if (!r) {
      return;
    }
    // Hold Shift to lock a footprint edge to the X or Y axis (a rectangle is already axis-aligned); a
    // snapped point commits exactly and overrides both.
    const axisLock = isFootprint && event.shiftKey && !r.exact;
    store.pick(r.target, { axisLock, exact: r.exact });
    setLengthInput("");
    setSnap(r.snap);
    setDraft(store.draft(r.target, { axisLock, exact: r.exact }));
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

  /** The committed footprint ring as an SVG points string, read from the engine's mirror. */
  const ringPoints = (footprint: FootprintMirror): string =>
    footprint
      .vertices()
      .map((v) => `${sx(v.x)},${sy(v.y)}`)
      .join(" ");

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
  const cursor = surfaceCursor(panning, isSelect, hoverCue !== null);

  // Running width×depth: the in-progress picks + cursor while drawing, else the committed ring.
  const extentsPoints: readonly Point[] =
    showPreview && draft ? [...store.pendingPicks, draft.point] : ringVertices;
  const extents = footprintExtents(extentsPoints);

  return (
    <div className="plan__wrap">
      <svg
        aria-label="Plan drawing surface"
        className="plan"
        onPointerCancel={endPan}
        onPointerDown={onPointerDown}
        onPointerLeave={onPointerLeave}
        onPointerMove={onPointerMove}
        onPointerUp={endPan}
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

        {/* The engine's canonical footprint, read from the mirror. */}
        {hasRing && footprint && (
          <polygon className="plan__footprint" points={ringPoints(footprint)} />
        )}

        {/* Persistent dimension labels: each committed edge's length at its midpoint. */}
        {hasRing && edgeLengthCues(camera, ringVertices)}

        {/* Selection cues: hover under, the committed selection on top. */}
        {selectionCue(camera, ringVertices, hoverCue, "hover")}
        {selectionCue(camera, ringVertices, store.selection, "selected")}

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
