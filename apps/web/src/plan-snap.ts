/**
 * Plan-view snapping (P1 #5, ADR 0014). Resolves the raw cursor to an exact world point on existing
 * geometry — a footprint/pending **endpoint**, an edge **midpoint**, or the nearest point **on an
 * edge** — at a constant *screen-space* tolerance (so a snap feels the same at every zoom, like
 * `plan-selection.ts` and `plan-camera.ts`). Pure: no React, no DOM — unit-tested directly.
 *
 * Beyond point snaps, `resolveDraw` adds **on-axis** inference and **locks** (Shift → the dominant
 * axis; arrow keys → an explicit axis) through the anchor. Still deferred: parallel/perpendicular to
 * arbitrary edges (low value in an orthogonal framing tool — most edges are axis-aligned, so parallel
 * already coincides with on-axis) and intersection. The view renders the returned cue and commits the
 * snapped point through `pick({ exact: true })`, so the runner stays pixel-free and the engine remains
 * the source of truth ([ADR 0008](../../docs/adr/0008-mvp-geometry-and-command-contract.md)).
 */

import type { Point } from "@jose/tool-runner";
import {
  type PlanCamera,
  toScreenX,
  toScreenY,
  toWorldX,
  toWorldY,
} from "./plan-camera";
import { type Px, projectToSegment } from "./plan-selection";

/** The kinds a draw snap resolves. Point snaps (endpoint → midpoint → on-edge) pin an exact point;
 *  `on-axis` constrains the segment to a world axis through the anchor (inferred, or locked). */
export type SnapKind = "endpoint" | "midpoint" | "on-axis" | "on-edge";

/** The X (horizontal, red) or Y (vertical, green) world axis a draw is constrained to. */
export type LockAxis = "x" | "y" | null;

/** The axis line to draw for an `on-axis` snap: a full-extent line at `atTicks`, `locked` when a hard
 *  Shift/arrow lock (bold) rather than a soft inference. `horizontal` = the X axis (constant Y). */
export interface AxisGuide {
  readonly atTicks: number;
  readonly locked: boolean;
  readonly orientation: "horizontal" | "vertical";
}

/** The active draw constraint: an explicit `axis` (arrow-key lock), and/or `shift` held (lock the
 *  dominant axis relative to the anchor — SketchUp's Shift). */
export interface DrawLock {
  readonly axis: LockAxis;
  readonly shift: boolean;
}

/** A resolved snap: the kind, the exact world point (ticks) the view commits, and — for `on-axis` —
 *  the guide line to draw. */
export interface Snap {
  readonly guide?: AxisGuide;
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
  "on-axis": "On Axis",
  "on-edge": "On Edge",
};

/** Within this angle of a world axis, the segment infers on-axis (SketchUp's ~few-degrees band). */
const AXIS_TAN = Math.tan((6 * Math.PI) / 180);

/** The world axis the anchor→cursor segment runs most along (the axis a Shift-lock constrains to). */
function dominantAxis(anchor: Point, cursor: Point): LockAxis {
  return Math.abs(cursor.x - anchor.x) >= Math.abs(cursor.y - anchor.y)
    ? "x"
    : "y";
}

/** The segment's axis when it runs within `AXIS_TAN` of a world axis, else `null` (free direction). */
function inferAxis(anchor: Point, cursor: Point): LockAxis {
  const dx = Math.abs(cursor.x - anchor.x);
  const dy = Math.abs(cursor.y - anchor.y);
  if (dx === 0 && dy === 0) {
    return null;
  }
  if (dy <= AXIS_TAN * dx) {
    return "x"; // near-horizontal
  }
  if (dx <= AXIS_TAN * dy) {
    return "y"; // near-vertical
  }
  return null;
}

/** The point on the axis through `anchor`: locking X keeps the cursor's X and the anchor's Y (a
 *  horizontal run), locking Y keeps the anchor's X and the cursor's Y (a vertical run). */
function projectAxis(anchor: Point, cursor: Point, axis: "x" | "y"): Point {
  return axis === "x"
    ? { x: cursor.x, y: anchor.y }
    : { x: anchor.x, y: cursor.y };
}

/** The guide line for an axis constraint through `anchor`. */
function axisGuide(anchor: Point, axis: "x" | "y", locked: boolean): AxisGuide {
  return axis === "x"
    ? { orientation: "horizontal", atTicks: anchor.y, locked }
    : { orientation: "vertical", atTicks: anchor.x, locked };
}

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

/**
 * Resolve a draw pick to a snap, with the full priority: a **hard lock** (arrow-key `axis`, or Shift
 * on the dominant axis) → a **point snap** (endpoint/midpoint/on-edge, exact) → **on-axis inference**
 * (the segment running within a few degrees of a world axis) → `null` (free — the view falls back to
 * the grid). Lock and on-axis need an `anchor` (the previous pick); point snaps do not, so they work
 * for the rectangle tool too (which passes `anchor = null`).
 */
export function resolveDraw(
  camera: PlanCamera,
  ring: readonly Point[],
  pending: readonly Point[],
  screen: Px,
  anchor: Point | null,
  lock: DrawLock,
  tol: SnapTolerance = DEFAULT_SNAP_TOLERANCE
): Snap | null {
  const cursor: Point = {
    x: Math.round(toWorldX(camera, screen.px)),
    y: Math.round(toWorldY(camera, screen.py)),
  };

  // 1) Hard lock: constrain to the arrow-locked axis, or (Shift) the dominant axis.
  if (anchor) {
    const locked =
      lock.axis ?? (lock.shift ? dominantAxis(anchor, cursor) : null);
    if (locked) {
      return {
        kind: "on-axis",
        world: projectAxis(anchor, cursor, locked),
        guide: axisGuide(anchor, locked, true),
      };
    }
  }

  // 2) Point snaps win over inference — they pin an exact point.
  const point = resolveSnap(camera, ring, pending, screen, tol);
  if (point) {
    return point;
  }

  // 3) On-axis inference (soft) when the segment runs near a world axis.
  if (anchor) {
    const axis = inferAxis(anchor, cursor);
    if (axis) {
      return {
        kind: "on-axis",
        world: projectAxis(anchor, cursor, axis),
        guide: axisGuide(anchor, axis, false),
      };
    }
  }

  return null;
}
