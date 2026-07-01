/**
 * Plan-view snapping (P1 #5, ADR 0014). Resolves the raw cursor to an exact world point on existing
 * geometry — a footprint/pending **endpoint**, an edge **midpoint**, or the nearest point **on an
 * edge** — at a constant *screen-space* tolerance (so a snap feels the same at every zoom, like
 * `plan-selection.ts` and `plan-camera.ts`). Pure: no React, no DOM — unit-tested directly.
 *
 * This is the first cut of the SketchUp inference engine (endpoint/midpoint/on-edge point snaps);
 * linear inference (on-axis, parallel/perpendicular) and locks land next. The view renders the
 * returned cue and commits the snapped point through `pick({ exact: true })`, so the runner stays
 * pixel-free and the engine remains the source of truth ([ADR 0008](../../docs/adr/0008-mvp-geometry-and-command-contract.md)).
 */

import type { Point } from "@jose/tool-runner";
import { type PlanCamera, toScreenX, toScreenY } from "./plan-camera";
import { type Px, projectToSegment } from "./plan-selection";

/** The kinds of point snap this cut resolves. Priority is endpoint → midpoint → on-edge. */
export type SnapKind = "endpoint" | "midpoint" | "on-edge";

/** A resolved snap: which kind fired and the exact world point (ticks) the view commits. */
export interface Snap {
  readonly kind: SnapKind;
  readonly world: Point;
}

/** Screen-px radii. A cursor within `point` of an endpoint/midpoint snaps to it; within `edge` of an
 *  edge snaps onto it. `point ≥ edge` so a corner/midpoint beats the edge running through it (the same
 *  ordering `plan-selection.ts` uses for picking). */
export interface SnapTolerance {
  readonly edge: number;
  readonly point: number;
}

export const DEFAULT_SNAP_TOLERANCE: SnapTolerance = { point: 10, edge: 6 };

/** The badge wording per snap kind (user-facing copy; owned by `product-design/.../copy.md`). */
export const SNAP_LABEL: Record<SnapKind, string> = {
  endpoint: "Endpoint",
  midpoint: "Midpoint",
  "on-edge": "On Edge",
};

const midpointOf = (a: Point, b: Point): Point => ({
  x: Math.round((a.x + b.x) / 2),
  y: Math.round((a.y + b.y) / 2),
});

/** The edges of a vertex list as endpoint pairs: a closed ring (the closing edge included) when
 *  `closed` and ≥3 vertices, else the open chain. */
function edgesOf(verts: readonly Point[], closed: boolean): [Point, Point][] {
  const n = verts.length;
  if (n < 2) {
    return [];
  }
  const out: [Point, Point][] = [];
  const count = closed && n >= 3 ? n : n - 1;
  for (let i = 0; i < count; i++) {
    out.push([verts[i] as Point, verts[(i + 1) % n] as Point]);
  }
  return out;
}

/**
 * Resolve the cursor at screen point `screen` to a snap on the committed `ring` (a closed footprint)
 * and the in-progress `pending` chain (an open polyline), or `null` for no snap. Priority endpoint →
 * midpoint → on-edge; within a tier the nearest candidate inside tolerance wins.
 */
export function resolveSnap(
  camera: PlanCamera,
  ring: readonly Point[],
  pending: readonly Point[],
  screen: Px,
  tol: SnapTolerance = DEFAULT_SNAP_TOLERANCE
): Snap | null {
  const toPx = (p: Point): Px => ({
    px: toScreenX(camera, p.x),
    py: toScreenY(camera, p.y),
  });
  const dist = (a: Px, b: Px): number => Math.hypot(a.px - b.px, a.py - b.py);
  const edges = [...edgesOf(ring, true), ...edgesOf(pending, false)];

  // Track the nearest candidate found *in the current tier*; a hit ends the search (tiers are
  // strictly ordered, so any endpoint beats any midpoint beats any on-edge).
  const nearest = (
    candidates: { world: Point; at: Px }[],
    kind: SnapKind,
    limit: number
  ): Snap | null => {
    let best: { d: number; world: Point } | null = null;
    for (const c of candidates) {
      const d = dist(c.at, screen);
      if (d <= limit && (best === null || d < best.d)) {
        best = { d, world: c.world };
      }
    }
    return best ? { kind, world: best.world } : null;
  };

  // 1) Endpoints — every committed + pending vertex.
  const endpoints = [...ring, ...pending].map((v) => ({
    world: v,
    at: toPx(v),
  }));
  const endpoint = nearest(endpoints, "endpoint", tol.point);
  if (endpoint) {
    return endpoint;
  }

  // 2) Midpoints — the midpoint of every edge.
  const mids = edges.map(([a, b]) => {
    const world = midpointOf(a, b);
    return { world, at: toPx(world) };
  });
  const midpoint = nearest(mids, "midpoint", tol.point);
  if (midpoint) {
    return midpoint;
  }

  // 3) On-edge — the nearest point on any edge (lerp the *world* endpoints by the screen projection).
  const onEdge = edges.map(([a, b]) => {
    const proj = projectToSegment(screen, toPx(a), toPx(b));
    const world = {
      x: Math.round(a.x + (b.x - a.x) * proj.t),
      y: Math.round(a.y + (b.y - a.y) * proj.t),
    };
    return { world, at: { px: proj.px, py: proj.py } };
  });
  return nearest(onEdge, "on-edge", tol.edge);
}
