/**
 * Footprint editing — the pure ring transforms behind P2 #9 (ADR 0015). A drawn footprint is a ring
 * of world-XY tick vertices; the three edit verbs each produce a *new* ring, which the plan view
 * commits through `store.editFootprint` (the engine validates it and re-extrudes at the current mass
 * height). These are computed client-side against the index-keyed render mirror (ADR 0013), applied
 * transiently during a drag, and only the committed ring crosses the boundary.
 *
 * No React, no DOM, no engine types — unit-tested directly, like `plan-selection.ts` / `plan-snap.ts`.
 */

import type { Point } from "@jose/tool-runner";

/** A footprint ring needs at least a triangle; below this a delete is refused. */
export const MIN_RING_VERTICES = 3;

/** Move vertex `index` to `point`, returning the new ring (the others unchanged). Out-of-range
 *  indices return the ring untouched — the caller resolved the index against the same mirror. */
export function moveVertex(
  ring: readonly Point[],
  index: number,
  point: Point
): Point[] {
  if (index < 0 || index >= ring.length) {
    return [...ring];
  }
  return ring.map((v, i) => (i === index ? point : v));
}

/** Insert `point` as a new vertex splitting `edgeIndex` (the edge from vertex `edgeIndex` to
 *  `edgeIndex + 1`, closing at the last), returning the new, one-longer ring. */
export function insertOnEdge(
  ring: readonly Point[],
  edgeIndex: number,
  point: Point
): Point[] {
  if (edgeIndex < 0 || edgeIndex >= ring.length) {
    return [...ring];
  }
  const next = [...ring];
  next.splice(edgeIndex + 1, 0, point);
  return next;
}

/** Delete vertex `index`, returning the shorter ring — or `null` when that would drop the ring below
 *  [`MIN_RING_VERTICES`] (a footprint can't be fewer than three corners), so the caller can explain
 *  why nothing happened instead of sending a doomed edit. */
export function deleteVertex(
  ring: readonly Point[],
  index: number
): Point[] | null {
  if (index < 0 || index >= ring.length) {
    return null;
  }
  if (ring.length <= MIN_RING_VERTICES) {
    return null;
  }
  return ring.filter((_, i) => i !== index);
}
