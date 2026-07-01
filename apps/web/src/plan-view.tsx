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
 */

import type { FootprintMirror } from "@jose/render-mirror";
import type { DraftPoint } from "@jose/tool-runner";
import { formatLength, parseLength, pointAtDistance } from "@jose/tool-runner";
import { type PointerEvent, useRef, useState } from "react";
import type { EngineStore } from "./engine-store";
import { ValueBox } from "./value-box";

/** Screen pixels per world tick. 1 tick = 1/32in; ~0.05 px/tick ≈ a 10ft wall is ~192px. */
const PX_PER_TICK = 0.05;
/** World-space tick offset placed at the viewport origin, so the drawing area sits in view. */
const ORIGIN_TICKS = { x: 1920, y: 1920 };
/** Grid line spacing in world ticks (384 = 1ft). */
const GRID_TICKS = 384;
/** Half-extent of the world the grid spans, in ticks. */
const GRID_HALF_TICKS = 7680;

const VIEW_W = 640;
const VIEW_H = 640;

/** World tick X → screen px. */
function sx(xTicks: number): number {
  return (xTicks + ORIGIN_TICKS.x) * PX_PER_TICK;
}
/** World tick Y → screen px (world Y is up, screen Y is down). */
function sy(yTicks: number): number {
  return VIEW_H - (yTicks + ORIGIN_TICKS.y) * PX_PER_TICK;
}
/** Screen px → world tick X. */
function wx(px: number): number {
  return px / PX_PER_TICK - ORIGIN_TICKS.x;
}
/** Screen px → world tick Y (inverse of `sy`). */
function wy(px: number): number {
  return (VIEW_H - px) / PX_PER_TICK - ORIGIN_TICKS.y;
}

function gridLines(): {
  key: string;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}[] {
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
      x1: sx(t),
      y1: sy(-GRID_HALF_TICKS),
      x2: sx(t),
      y2: sy(GRID_HALF_TICKS),
    });
    lines.push({
      key: `h${t}`,
      x1: sx(-GRID_HALF_TICKS),
      y1: sy(t),
      x2: sx(GRID_HALF_TICKS),
      y2: sy(t),
    });
  }
  return lines;
}

/** Grid lines span a fixed world extent — compute once at module load, not per render. */
const GRID_LINES = gridLines();

/** The committed footprint ring as an SVG points string, read from the engine's mirror. */
function ringPoints(footprint: FootprintMirror): string {
  return footprint
    .vertices()
    .map((v) => `${sx(v.x)},${sy(v.y)}`)
    .join(" ");
}

export interface PlanViewProps {
  readonly store: EngineStore;
}

export function PlanView({ store }: PlanViewProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [draft, setDraft] = useState<DraftPoint | null>(null);
  const [hovering, setHovering] = useState(false);
  const [lengthInput, setLengthInput] = useState("");

  const isFootprint = store.activeTool === "footprint";

  /** World point (ticks) under a pointer event, via the screen↔world transforms. */
  const worldFromEvent = (
    event: PointerEvent<SVGSVGElement>
  ): { x: number; y: number } | null => {
    const svg = svgRef.current;
    if (!svg) {
      return null;
    }
    const rect = svg.getBoundingClientRect();
    const px = ((event.clientX - rect.left) / rect.width) * VIEW_W;
    const py = ((event.clientY - rect.top) / rect.height) * VIEW_H;
    return { x: wx(px), y: wy(py) };
  };

  const onPointerMove = (event: PointerEvent<SVGSVGElement>): void => {
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

  const onPointerDown = (event: PointerEvent<SVGSVGElement>): void => {
    if (!isFootprint) {
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
        onPointerDown={onPointerDown}
        onPointerLeave={onPointerLeave}
        onPointerMove={onPointerMove}
        ref={svgRef}
        viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
      >
        <title>Plan drawing surface</title>
        <g className="plan__grid">
          {GRID_LINES.map((l) => (
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
