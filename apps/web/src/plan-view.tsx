/**
 * The plan view — the top-down (world XY), orthographic 2D drawing surface (ADR 0005 / CONTEXT.md).
 *
 * While the footprint tool is active, clicks add ring vertices and a click near the first closes the
 * ring, sending a `DrawFootprint` command into the worker. Holding Shift while clicking locks the new
 * edge to the X or Y axis (orthogonal drawing). As the cursor moves it shows a live preview: a
 * rubber-band segment, dashed alignment guides to existing vertices, and a highlighted close target
 * when a click would close the ring. The value-entry box accepts an exact length (feet/inches) to
 * place the next vertex precisely along the current direction.
 *
 * When the snapshot returns, this renders the footprint polygon **from the `FootprintMirror`** — the
 * engine's canonical ring — never from the raw clicks. The mid-draw polyline (`pendingPicks`), the
 * preview, and the guides are transient UI only.
 *
 * The view is navigable (P0 #2): scroll zooms toward the cursor, middle-drag pans, and Fit / Shift+Z
 * runs Zoom-Extents. All of it flows through one `PlanCamera` (`plan-camera.ts`) held in state — the
 * pure world↔screen transform every coordinate below is derived from.
 */

import type { FootprintMirror } from "@jose/render-mirror";
import type { DraftPoint } from "@jose/tool-runner";
import { formatLength, parseLength, pointAtDistance } from "@jose/tool-runner";
import { type PointerEvent, useEffect, useMemo, useRef, useState } from "react";
import type { EngineStore } from "./engine-store";
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
import { ValueBox } from "./value-box";

/** Grid line spacing in world ticks (384 = 1ft). */
const GRID_TICKS = 384;
/** Half-extent of the world the grid spans, in ticks. */
const GRID_HALF_TICKS = 7680;
/** Per-notch wheel zoom factor (in on scroll-up, out on scroll-down). */
const ZOOM_STEP = 1.1;

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
  /** Active middle-drag pan, tracked in a ref so pointer-move doesn't re-render per frame. */
  const panRef = useRef<{
    pointerId: number;
    lastPx: number;
    lastPy: number;
  } | null>(null);

  const isFootprint = store.activeTool === "footprint";

  /** Local screen↔world binds over the current camera, so the JSX below reads plainly. */
  const sx = (xTicks: number): number => toScreenX(camera, xTicks);
  const sy = (yTicks: number): number => toScreenY(camera, yTicks);

  /** A pointer's position in the fixed viewBox pixel space (independent of the element's size). */
  const viewBoxPoint = (
    clientX: number,
    clientY: number
  ): { px: number; py: number } | null => {
    const svg = svgRef.current;
    if (!svg) {
      return null;
    }
    const rect = svg.getBoundingClientRect();
    return {
      px: ((clientX - rect.left) / rect.width) * VIEW_W,
      py: ((clientY - rect.top) / rect.height) * VIEW_H,
    };
  };

  /** World point (ticks) under a pointer event, via the current camera. */
  const worldFromEvent = (
    event: PointerEvent<SVGSVGElement>
  ): { x: number; y: number } | null => {
    const vb = viewBoxPoint(event.clientX, event.clientY);
    if (!vb) {
      return null;
    }
    return { x: toWorldX(camera, vb.px), y: toWorldY(camera, vb.py) };
  };

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

  // Scroll to zoom toward the cursor. A native, non-passive listener is required to preventDefault
  // the page scroll; React's synthetic wheel handler is passive and can't. `setCamera`'s functional
  // form reads the latest camera, so this attaches once.
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
  }, []);

  // Shift+Z zooms to fit (SketchUp's Zoom-Extents). A ref keeps the latest content in reach without
  // re-subscribing; skipped while typing so it doesn't hijack the value box.
  const fitRef = useRef(fitToContent);
  fitRef.current = fitToContent;
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
        fitRef.current();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const onPointerMove = (event: PointerEvent<SVGSVGElement>): void => {
    const pan = panRef.current;
    if (pan && event.pointerId === pan.pointerId) {
      const vb = viewBoxPoint(event.clientX, event.clientY);
      if (!vb) {
        return;
      }
      setCamera((cam) => panBy(cam, vb.px - pan.lastPx, vb.py - pan.lastPy));
      pan.lastPx = vb.px;
      pan.lastPy = vb.py;
      return;
    }
    if (!isFootprint) {
      return;
    }
    const world = worldFromEvent(event);
    if (!world) {
      return;
    }
    setHovering(true);
    setDraft(store.draft(world, { axisLock: event.shiftKey }));
  };

  const onPointerLeave = (): void => {
    setHovering(false);
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
      const vb = viewBoxPoint(event.clientX, event.clientY);
      if (!vb) {
        return;
      }
      panRef.current = {
        pointerId: event.pointerId,
        lastPx: vb.px,
        lastPy: vb.py,
      };
      svgRef.current?.setPointerCapture(event.pointerId);
      setPanning(true);
      return;
    }
    // Only a primary click on the footprint tool places a vertex.
    if (event.button !== 0 || !isFootprint) {
      return;
    }
    const world = worldFromEvent(event);
    if (!world) {
      return;
    }
    // Hold Shift to lock the new edge to the X or Y axis relative to the previous vertex.
    store.pick(world, { axisLock: event.shiftKey });
    setLengthInput("");
    setDraft(store.draft(world, { axisLock: event.shiftKey }));
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
  /** Live segment length (anchor → cursor), ticks, while drawing. */
  const liveLength =
    anchor && draft
      ? Math.hypot(draft.point.x - anchor.x, draft.point.y - anchor.y)
      : null;

  /** Place the next vertex at the typed length along the current cursor direction. */
  const commitLength = (axisLock: boolean): void => {
    if (!(anchor && draft)) {
      return;
    }
    const ticks = parseLength(lengthInput);
    if (ticks === null) {
      // Don't silently swallow an unparseable entry — tell the user how to phrase it.
      store.flagRejection("Enter a length like 10' 6\" or 126in.");
      return;
    }
    const target = pointAtDistance(anchor, draft.point, ticks, axisLock);
    store.pick(target, { exact: true });
    setLengthInput("");
    setDraft(null);
  };

  const footprint = store.footprint;
  const hasRing = footprint !== null && footprint.count >= 3;
  const showPreview = isFootprint && hovering && draft !== null;
  // The value box is live only once a segment has somewhere to grow from (≥1 vertex down).
  const canEnterLength = isFootprint && anchor !== null;

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
        style={panning ? { cursor: "grabbing" } : undefined}
        viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
      >
        <title>Plan drawing surface</title>
        <g className="plan__grid">
          {gridLines.map((l) => (
            <line key={l.key} x1={l.x1} x2={l.x2} y1={l.y1} y2={l.y2} />
          ))}
        </g>

        {/* Dashed alignment guides: the cursor shares an existing vertex's row/column. */}
        {showPreview &&
          draft.guides.map((g) =>
            g.orientation === "vertical" ? (
              <line
                className="plan__guide"
                key={`gv${g.sourceIndex}`}
                x1={sx(g.atTicks)}
                x2={sx(g.atTicks)}
                y1={0}
                y2={VIEW_H}
              />
            ) : (
              <line
                className="plan__guide"
                key={`gh${g.sourceIndex}`}
                x1={0}
                x2={VIEW_W}
                y1={sy(g.atTicks)}
                y2={sy(g.atTicks)}
              />
            )
          )}

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

        {/* Rubber-band segment from the last vertex to the live cursor. */}
        {showPreview && anchor && (
          <line
            className="plan__rubber"
            x1={sx(anchor.x)}
            x2={sx(draft.point.x)}
            y1={sy(anchor.y)}
            y2={sy(draft.point.y)}
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

        {/* Close target: a click here closes the ring — make it unmistakable. */}
        {showPreview && draft.closing && store.pendingPicks[0] && (
          <g className="plan__close">
            <circle
              cx={sx(store.pendingPicks[0].x)}
              cy={sy(store.pendingPicks[0].y)}
              r={9}
            />
            <text
              x={sx(store.pendingPicks[0].x) + 13}
              y={sy(store.pendingPicks[0].y) - 9}
            >
              Close
            </text>
          </g>
        )}

        {/* Live cursor + dimension readout while drawing. */}
        {showPreview && !draft.closing && (
          <circle
            className="plan__cursor"
            cx={sx(draft.point.x)}
            cy={sy(draft.point.y)}
            r={3.5}
          />
        )}
        {showPreview &&
          liveLength !== null &&
          liveLength > 0 &&
          !draft.closing && (
            <text
              className="plan__dim"
              x={sx(draft.point.x) + 10}
              y={sy(draft.point.y) - 10}
            >
              {formatLength(Math.round(liveLength))}
            </text>
          )}

        {/* The engine's canonical footprint, read from the mirror. */}
        {hasRing && footprint && (
          <polygon className="plan__footprint" points={ringPoints(footprint)} />
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

      {/* Value-entry box (shared VCB): type an exact length, Enter to place the vertex along the
          cursor direction; Shift+Enter locks it to an axis (ADR 0012 §4). */}
      <ValueBox
        ariaLabel="Segment length in feet and inches"
        disabled={!canEnterLength}
        label="Length"
        onCancel={() => {
          setLengthInput("");
          store.cancelDraw();
          setDraft(null);
        }}
        onChange={setLengthInput}
        onSubmit={({ shiftKey }) => commitLength(shiftKey)}
        placeholder={
          liveLength !== null && liveLength > 0
            ? formatLength(Math.round(liveLength))
            : `e.g. 10' 6"`
        }
        value={lengthInput}
      />
    </div>
  );
}
