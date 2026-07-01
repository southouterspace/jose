import { describe, expect, test } from "bun:test";
import { DEFAULT_CAMERA, toScreenX, toScreenY } from "./plan-camera";
import {
  distanceToSegment,
  hitTest,
  pointInPolygon,
  type RingVertex,
  sameSelection,
} from "./plan-selection";

/** A 10ft × 10ft square footprint (world ticks; 384 = 1ft), one ring (spaceId 0). */
const SQUARE: RingVertex[] = [
  { x: 0, y: 0, spaceId: 0 },
  { x: 3840, y: 0, spaceId: 0 },
  { x: 3840, y: 3840, spaceId: 0 },
  { x: 0, y: 3840, spaceId: 0 },
];

/** The screen pixel a world point projects to under the default camera. */
const screenOf = (x: number, y: number) => ({
  px: toScreenX(DEFAULT_CAMERA, x),
  py: toScreenY(DEFAULT_CAMERA, y),
});

describe("distanceToSegment", () => {
  test("0 on the segment, perpendicular distance off it", () => {
    const a = { px: 0, py: 0 };
    const b = { px: 10, py: 0 };
    expect(distanceToSegment({ px: 5, py: 0 }, a, b)).toBe(0);
    expect(distanceToSegment({ px: 5, py: 4 }, a, b)).toBe(4);
  });

  test("clamps past the endpoints", () => {
    const a = { px: 0, py: 0 };
    const b = { px: 10, py: 0 };
    expect(distanceToSegment({ px: -3, py: 0 }, a, b)).toBe(3);
  });

  test("degenerate segment falls back to point distance", () => {
    const a = { px: 2, py: 2 };
    expect(distanceToSegment({ px: 5, py: 6 }, a, a)).toBe(5);
  });
});

describe("pointInPolygon", () => {
  const ring = SQUARE.map((v) => screenOf(v.x, v.y));

  test("center is inside", () => {
    expect(pointInPolygon(screenOf(1920, 1920), ring)).toBe(true);
  });

  test("far outside is outside", () => {
    expect(pointInPolygon(screenOf(20_000, 20_000), ring)).toBe(false);
  });
});

describe("hitTest priority", () => {
  test("empty ring never hits", () => {
    expect(hitTest(DEFAULT_CAMERA, [], screenOf(0, 0))).toBeNull();
  });

  test("a cursor over a corner picks that vertex", () => {
    expect(hitTest(DEFAULT_CAMERA, SQUARE, screenOf(3840, 0))).toEqual({
      kind: "vertex",
      index: 1,
    });
  });

  test("a cursor on an edge (not a corner) picks that edge", () => {
    // Midpoint of edge 0 (v0 → v1), the bottom edge.
    expect(hitTest(DEFAULT_CAMERA, SQUARE, screenOf(1920, 0))).toEqual({
      kind: "edge",
      index: 0,
    });
  });

  test("the closing edge (last → first) is pickable", () => {
    // Midpoint of edge 3 (v3 → v0), the left edge.
    expect(hitTest(DEFAULT_CAMERA, SQUARE, screenOf(0, 1920))).toEqual({
      kind: "edge",
      index: 3,
    });
  });

  test("a cursor inside the ring picks the whole footprint", () => {
    expect(hitTest(DEFAULT_CAMERA, SQUARE, screenOf(1920, 1920))).toEqual({
      kind: "footprint",
      spaceId: 0,
    });
  });

  test("a cursor in empty space hits nothing", () => {
    const far = screenOf(1920, 1920);
    expect(
      hitTest(DEFAULT_CAMERA, SQUARE, { px: far.px + 300, py: far.py })
    ).toBeNull();
  });

  test("vertex wins over the edges meeting at it", () => {
    // Exactly on corner v2 — both edge 1 and edge 2 touch it, but the vertex takes priority.
    expect(hitTest(DEFAULT_CAMERA, SQUARE, screenOf(3840, 3840))).toEqual({
      kind: "vertex",
      index: 2,
    });
  });
});

describe("sameSelection", () => {
  test("nulls, matches, and mismatches", () => {
    expect(sameSelection(null, null)).toBe(true);
    expect(sameSelection(null, { kind: "vertex", index: 0 })).toBe(false);
    expect(
      sameSelection({ kind: "vertex", index: 2 }, { kind: "vertex", index: 2 })
    ).toBe(true);
    expect(
      sameSelection({ kind: "edge", index: 1 }, { kind: "vertex", index: 1 })
    ).toBe(false);
    expect(
      sameSelection(
        { kind: "footprint", spaceId: 0 },
        { kind: "footprint", spaceId: 1 }
      )
    ).toBe(false);
  });
});
