/**
 * The plan view — the top-down (world XY), orthographic 2D drawing surface (ADR 0005 / CONTEXT.md).
 *
 * While the footprint tool is active, clicks add ring vertices and a click near the first closes the
 * ring, sending a `DrawFootprint` command into the worker. Holding Shift while clicking locks the new
 * edge to the X or Y axis (orthogonal drawing). When the snapshot returns, this renders
 * the footprint polygon **from the `FootprintMirror`** — the engine's canonical ring — never from
 * the raw clicks. The mid-draw polyline (`pendingPicks`) is transient UI only.
 */

import type { FootprintMirror } from "@jose/render-mirror";
import { type PointerEvent, useRef } from "react";
import type { EngineStore } from "./engine-store";

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

  const onPointerDown = (event: PointerEvent<SVGSVGElement>): void => {
    if (store.activeTool !== "footprint") {
      return;
    }
    const svg = svgRef.current;
    if (!svg) {
      return;
    }
    const rect = svg.getBoundingClientRect();
    const px = ((event.clientX - rect.left) / rect.width) * VIEW_W;
    const py = ((event.clientY - rect.top) / rect.height) * VIEW_H;
    // Hold Shift to lock the new edge to the X or Y axis relative to the previous vertex.
    store.pick({ x: wx(px), y: wy(py) }, { axisLock: event.shiftKey });
  };

  const footprint = store.footprint;
  const hasRing = footprint !== null && footprint.count >= 3;

  return (
    <svg
      aria-label="Plan drawing surface"
      className="plan"
      onPointerDown={onPointerDown}
      ref={svgRef}
      viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
    >
      <title>Plan drawing surface</title>
      <g className="plan__grid">
        {GRID_LINES.map((l) => (
          <line key={l.key} x1={l.x1} x2={l.x2} y1={l.y1} y2={l.y2} />
        ))}
      </g>

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

      {/* The engine's canonical footprint, read from the mirror. */}
      {hasRing && footprint && (
        <polygon className="plan__footprint" points={ringPoints(footprint)} />
      )}
    </svg>
  );
}
