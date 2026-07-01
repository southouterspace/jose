/**
 * Plan-view selection: what a cursor picks in the plan, and the pure hit-test that resolves it
 * (ADR 0013). A `Selection` names a piece of the *current* footprint ring by index — the whole face,
 * a vertex, or an edge — never engine geometry (selection is presentation state, owned by the store).
 *
 * `hitTest` projects the ring through the `PlanCamera` (P0 #2) into viewBox pixels and resolves in a
 * fixed priority — vertex → edge → face — so a corner is forgiving and a click inside the ring falls
 * through to the face. Screen-space testing keeps the pick tolerance constant at every zoom. No React,
 * no DOM: unit-tested directly (`plan-selection.test.ts`), like `plan-camera.ts`.
 */

import { type PlanCamera, toScreenX, toScreenY } from "./plan-camera";

/** A picked piece of the footprint ring. `edge` i runs from vertex i to vertex (i+1) mod n. */
export type Selection =
  | { readonly kind: "footprint"; readonly spaceId: number }
  | { readonly kind: "vertex"; readonly index: number }
  | { readonly kind: "edge"; readonly index: number };

/** The discriminant alone — what the status bar and cues key off without the payload. */
export type SelectionKind = Selection["kind"];

/** A ring vertex in world ticks, tagged with the space (ring) it belongs to. */
export interface RingVertex {
  readonly spaceId: number;
  readonly x: number;
  readonly y: number;
}

/** Pixel radii within which a cursor counts as over a vertex / edge (viewBox px). */
export interface HitTolerance {
  readonly edge: number;
  readonly vertex: number;
}

/** Vertex tolerance ≥ edge tolerance so picking a corner wins over the edges meeting at it. */
export const DEFAULT_TOLERANCE: HitTolerance = { vertex: 9, edge: 6 };

interface Px {
  readonly px: number;
  readonly py: number;
}

const distance = (a: Px, b: Px): number => Math.hypot(a.px - b.px, a.py - b.py);

/** Shortest distance from point `p` to segment `a`–`b` (all in screen px). */
export function distanceToSegment(p: Px, a: Px, b: Px): number {
  const dx = b.px - a.px;
  const dy = b.py - a.py;
  const lenSq = dx * dx + dy * dy;
  if (lenSq === 0) {
    return distance(p, a); // Degenerate segment: a and b coincide.
  }
  // Project p onto the line, clamped to the segment.
  const t = Math.max(
    0,
    Math.min(1, ((p.px - a.px) * dx + (p.py - a.py) * dy) / lenSq)
  );
  return distance(p, { px: a.px + t * dx, py: a.py + t * dy });
}

/** Even-odd ray cast: is screen point `p` inside the polygon `ring` (screen px)? */
export function pointInPolygon(p: Px, ring: readonly Px[]): boolean {
  let inside = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const a = ring[i];
    const b = ring[j];
    if (!(a && b)) {
      continue;
    }
    const straddles = a.py > p.py !== b.py > p.py;
    if (
      straddles &&
      p.px < ((b.px - a.px) * (p.py - a.py)) / (b.py - a.py) + a.px
    ) {
      inside = !inside;
    }
  }
  return inside;
}

/**
 * Resolve a cursor at screen point `screen` to the ring piece it picks, or `null` for empty space.
 * Priority: nearest vertex within tolerance → nearest edge within tolerance → enclosing face.
 */
export function hitTest(
  camera: PlanCamera,
  vertices: readonly RingVertex[],
  screen: Px,
  tolerance: HitTolerance = DEFAULT_TOLERANCE
): Selection | null {
  const n = vertices.length;
  if (n === 0) {
    return null;
  }
  const pts: Px[] = vertices.map((v) => ({
    px: toScreenX(camera, v.x),
    py: toScreenY(camera, v.y),
  }));

  // 1) Nearest vertex within tolerance.
  let bestVertex = -1;
  let bestVertexDist = tolerance.vertex;
  for (let i = 0; i < n; i++) {
    const d = distance(pts[i] as Px, screen);
    if (d <= bestVertexDist) {
      bestVertexDist = d;
      bestVertex = i;
    }
  }
  if (bestVertex >= 0) {
    return { kind: "vertex", index: bestVertex };
  }

  // 2) Nearest edge within tolerance. A ≥3-vertex ring is closed (edge n-1 → 0); a 2-vertex run has
  //    a single open edge.
  if (n >= 2) {
    const edgeCount = n >= 3 ? n : 1;
    let bestEdge = -1;
    let bestEdgeDist = tolerance.edge;
    for (let i = 0; i < edgeCount; i++) {
      const d = distanceToSegment(screen, pts[i] as Px, pts[(i + 1) % n] as Px);
      if (d <= bestEdgeDist) {
        bestEdgeDist = d;
        bestEdge = i;
      }
    }
    if (bestEdge >= 0) {
      return { kind: "edge", index: bestEdge };
    }
  }

  // 3) Inside the closed face.
  if (n >= 3 && pointInPolygon(screen, pts)) {
    return { kind: "footprint", spaceId: vertices[0]?.spaceId ?? 0 };
  }
  return null;
}

/** Whether two selections refer to the same piece (for cheap equality in the view/store). */
export function sameSelection(
  a: Selection | null,
  b: Selection | null
): boolean {
  if (a === null || b === null) {
    return a === b;
  }
  if (a.kind !== b.kind) {
    return false;
  }
  if (a.kind === "footprint" && b.kind === "footprint") {
    return a.spaceId === b.spaceId;
  }
  if (a.kind === "vertex" && b.kind === "vertex") {
    return a.index === b.index;
  }
  if (a.kind === "edge" && b.kind === "edge") {
    return a.index === b.index;
  }
  return false;
}
