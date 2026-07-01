import { describe, expect, test } from "bun:test";
import { DEFAULT_CAMERA, toScreenX, toScreenY } from "./plan-camera";
import type { Px } from "./plan-selection";
import { resolveSnap, SNAP_LABEL } from "./plan-snap";

const FT = 384;
/** A 24'×16' rectangle ring (world ticks), origin at a corner. */
const RING = [
  { x: 0, y: 0 },
  { x: 24 * FT, y: 0 },
  { x: 24 * FT, y: 16 * FT },
  { x: 0, y: 16 * FT },
];

/** The screen point (viewBox px) a world point lands at under the default camera. */
const at = (x: number, y: number): Px => ({
  px: toScreenX(DEFAULT_CAMERA, x),
  py: toScreenY(DEFAULT_CAMERA, y),
});

describe("resolveSnap — point snaps", () => {
  test("snaps to an endpoint (a footprint vertex) and returns its exact world point", () => {
    const v = { x: 24 * FT, y: 16 * FT };
    // Cursor a few px off the corner → endpoint snap onto the exact vertex.
    const screen = { px: at(v.x, v.y).px + 3, py: at(v.x, v.y).py - 2 };
    expect(resolveSnap(DEFAULT_CAMERA, RING, [], screen)).toEqual({
      kind: "endpoint",
      world: v,
    });
  });

  test("snaps to an edge midpoint", () => {
    // Midpoint of the bottom edge (0,0)-(24',0) is (12', 0).
    const mid = { x: 12 * FT, y: 0 };
    const screen = { px: at(mid.x, mid.y).px + 2, py: at(mid.x, mid.y).py + 2 };
    expect(resolveSnap(DEFAULT_CAMERA, RING, [], screen)).toEqual({
      kind: "midpoint",
      world: mid,
    });
  });

  test("snaps onto an edge (nearest point) when past the endpoint/midpoint radius", () => {
    // A point ~6' along the bottom edge — not near a vertex or the midpoint, but on the edge.
    const onEdge = { x: 6 * FT, y: 0 };
    const screen = {
      px: at(onEdge.x, onEdge.y).px,
      py: at(onEdge.x, onEdge.y).py + 3,
    };
    const snap = resolveSnap(DEFAULT_CAMERA, RING, [], screen);
    expect(snap?.kind).toBe("on-edge");
    expect(snap?.world.y).toBe(0);
    // Landed on the edge near where the cursor was (~6ft along).
    expect(Math.abs((snap?.world.x ?? 0) - 6 * FT)).toBeLessThan(FT);
  });

  test("endpoint beats midpoint/on-edge when both are within tolerance", () => {
    // Right at a vertex, which is also the meeting point of two edges: endpoint must win.
    const v = { x: 0, y: 0 };
    expect(resolveSnap(DEFAULT_CAMERA, RING, [], at(v.x, v.y))?.kind).toBe(
      "endpoint"
    );
  });

  test("snaps to an in-progress pending vertex (before the ring is committed)", () => {
    const pending = [
      { x: 4 * FT, y: 4 * FT },
      { x: 8 * FT, y: 4 * FT },
    ];
    const v = pending[1] as { x: number; y: number };
    expect(resolveSnap(DEFAULT_CAMERA, [], pending, at(v.x, v.y))).toEqual({
      kind: "endpoint",
      world: v,
    });
  });

  test("returns null when the cursor is far from all geometry", () => {
    expect(resolveSnap(DEFAULT_CAMERA, RING, [], { px: 5, py: 5 })).toBeNull();
  });

  test("every snap kind has a badge label", () => {
    expect(SNAP_LABEL.endpoint).toBe("Endpoint");
    expect(SNAP_LABEL.midpoint).toBe("Midpoint");
    expect(SNAP_LABEL["on-edge"]).toBe("On Edge");
  });
});
