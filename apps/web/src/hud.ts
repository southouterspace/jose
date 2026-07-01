/**
 * The HUD layer's pure text helpers — the measurement labels the drawing surfaces overlay on a
 * gesture (ADR 0012 §3). Kept free of React/Three so it unit-tests without a scene and can be shared
 * by both the plan (SVG) and 3D (HTML overlay) surfaces as the HUD grows. Today it holds the
 * push/pull readout, the plan segment's length+angle readout, per-edge dimension labels, and the
 * running width×depth extents; snap/inference badges join it here next.
 */

import type { Point } from "@jose/tool-runner";
import { formatLength } from "@jose/tool-runner";

/**
 * The push/pull readout: the resulting mass height and — once the drag has moved — the signed
 * distance being applied. This is what lets you *see* the push/pull distance instead of dragging the
 * top cap blind. `startHeightTicks` is the cap's height when the drag began (0 for a flat,
 * not-yet-extruded face); `distanceTicks` is the live signed drag delta. The resulting height is
 * clamped at 0 — the height a non-positive drag would produce, where the mass vanishes (a state
 * whose full treatment is still an open product decision, `surfaces-3d-view.md`).
 */
export function pushPullReadout(
  startHeightTicks: number,
  distanceTicks: number
): string {
  const height = formatLength(Math.max(0, startHeightTicks + distanceTicks));
  if (distanceTicks === 0) {
    return height; // No drag yet — just name the current height.
  }
  const arrow = distanceTicks > 0 ? "▲" : "▼";
  return `${height}  ${arrow} ${formatLength(Math.abs(distanceTicks))}`;
}

/**
 * The plan bearing of the segment `from → to`, in degrees, measured counter-clockwise from world +X
 * and normalized to `[0, 360)`. World Y is up (the plan camera's convention), so due-east is `0`,
 * due-north `90`. This is the angle the live segment readout shows and the same convention the value
 * box's polar entry (`parsePolarLength`) types back — so what you read is what you can type.
 */
export function segmentAngleDegrees(from: Point, to: Point): number {
  const deg = (Math.atan2(to.y - from.y, to.x - from.x) * 180) / Math.PI;
  return ((deg % 360) + 360) % 360;
}

/** A plan bearing formatted for display: whole degrees with a degree sign (e.g. `45°`). */
export function formatAngle(degrees: number): string {
  return `${Math.round(degrees) % 360}°`;
}

/** The live draw readout: the segment's length and its bearing, e.g. `12' 0"  45°`. One string so the
 *  plan surface renders a single label trailing the cursor (matching `pushPullReadout`'s shape). */
export function segmentReadout(lengthTicks: number, degrees: number): string {
  return `${formatLength(lengthTicks)}  ${formatAngle(degrees)}`;
}

/** A per-edge measurement label: the edge's length (ticks) and the world-space midpoint to anchor it,
 *  for the persistent dimension labels on a committed footprint. */
export interface EdgeLabel {
  readonly lengthTicks: number;
  readonly midX: number;
  readonly midY: number;
}

/** The dimension labels for a closed ring: one per edge (vertex _i_ → _i_+1, the last edge closing
 *  back to vertex 0), each carrying its length and midpoint. Empty for a degenerate ring (< 2
 *  vertices). World ticks throughout — the surface maps midpoints to screen and formats the length. */
export function edgeLabels(vertices: readonly Point[]): EdgeLabel[] {
  if (vertices.length < 2) {
    return [];
  }
  return vertices.map((a, i) => {
    const b = vertices[(i + 1) % vertices.length] ?? a;
    return {
      lengthTicks: Math.round(Math.hypot(b.x - a.x, b.y - a.y)),
      midX: (a.x + b.x) / 2,
      midY: (a.y + b.y) / 2,
    };
  });
}

/** The overall bounding extents of a set of plan points, in ticks — the running width×depth readout.
 *  `null` for fewer than two points (no meaningful extent yet). Width is the world-X span, depth the
 *  world-Y span. */
export function footprintExtents(
  points: readonly Point[]
): { width: number; depth: number } | null {
  if (points.length < 2) {
    return null;
  }
  let minX = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;
  for (const p of points) {
    minX = Math.min(minX, p.x);
    maxX = Math.max(maxX, p.x);
    minY = Math.min(minY, p.y);
    maxY = Math.max(maxY, p.y);
  }
  return { width: Math.round(maxX - minX), depth: Math.round(maxY - minY) };
}

/** The width×depth readout, feet/inches: e.g. `24' 0" × 16' 0"`. */
export function formatExtents(width: number, depth: number): string {
  return `${formatLength(width)} × ${formatLength(depth)}`;
}
