import { describe, expect, test } from "bun:test";
import {
  edgeLabels,
  footprintExtents,
  formatAngle,
  formatExtents,
  pushPullReadout,
  segmentAngleDegrees,
  segmentReadout,
} from "./hud";

/** 1ft = 384 ticks (1/32in canonical). */
const FT = 384;

describe("pushPullReadout", () => {
  test("flat face: the resulting height equals the upward drag", () => {
    expect(pushPullReadout(0, 8 * FT)).toBe(`8' 0"  ▲ 8' 0"`);
  });

  test("adds a positive drag to the starting height, marked ▲", () => {
    expect(pushPullReadout(8 * FT, FT)).toBe(`9' 0"  ▲ 1' 0"`);
  });

  test("a downward drag lowers the height and reads ▼", () => {
    expect(pushPullReadout(8 * FT, -2 * FT)).toBe(`6' 0"  ▼ 2' 0"`);
  });

  test("clamps the resulting height at zero when the mass would vanish", () => {
    expect(pushPullReadout(FT, -3 * FT)).toBe(`0' 0"  ▼ 3' 0"`);
  });

  test("no drag yet: shows only the current height", () => {
    expect(pushPullReadout(8 * FT, 0)).toBe(`8' 0"`);
  });
});

describe("segmentAngleDegrees", () => {
  test("measures CCW from world +X, Y up (east 0, north 90)", () => {
    expect(segmentAngleDegrees({ x: 0, y: 0 }, { x: 10, y: 0 })).toBe(0);
    expect(segmentAngleDegrees({ x: 0, y: 0 }, { x: 0, y: 10 })).toBe(90);
    expect(segmentAngleDegrees({ x: 0, y: 0 }, { x: 10, y: 10 })).toBe(45);
  });

  test("normalizes a southward bearing into [0, 360)", () => {
    // Due south is -90° raw → 270°.
    expect(segmentAngleDegrees({ x: 0, y: 0 }, { x: 0, y: -10 })).toBe(270);
  });
});

describe("formatAngle / segmentReadout", () => {
  test("formatAngle rounds to whole degrees with a sign", () => {
    expect(formatAngle(44.6)).toBe("45°");
    expect(formatAngle(0)).toBe("0°");
  });

  test("segmentReadout pairs length and bearing", () => {
    expect(segmentReadout(12 * FT, 45)).toBe(`12' 0"  45°`);
  });
});

describe("edgeLabels", () => {
  test("labels each edge of a closed ring, including the closing edge", () => {
    // A 24'×16' rectangle (ticks): 4 edges, last closes back to vertex 0.
    const ring = [
      { x: 0, y: 0 },
      { x: 24 * FT, y: 0 },
      { x: 24 * FT, y: 16 * FT },
      { x: 0, y: 16 * FT },
    ];
    const labels = edgeLabels(ring);
    expect(labels).toHaveLength(4);
    expect(labels[0]).toEqual({ lengthTicks: 24 * FT, midX: 12 * FT, midY: 0 });
    // The closing edge (vertex 3 → 0) is the left side, 16ft, midpoint on x=0.
    expect(labels[3]).toEqual({
      lengthTicks: 16 * FT,
      midX: 0,
      midY: 8 * FT,
    });
  });

  test("is empty for a degenerate ring (< 2 vertices)", () => {
    expect(edgeLabels([])).toEqual([]);
    expect(edgeLabels([{ x: 0, y: 0 }])).toEqual([]);
  });
});

describe("footprintExtents / formatExtents", () => {
  test("reports the bounding width (X span) and depth (Y span)", () => {
    const pts = [
      { x: 0, y: 0 },
      { x: 24 * FT, y: 0 },
      { x: 24 * FT, y: 16 * FT },
    ];
    expect(footprintExtents(pts)).toEqual({ width: 24 * FT, depth: 16 * FT });
  });

  test("is null with fewer than two points", () => {
    expect(footprintExtents([])).toBeNull();
    expect(footprintExtents([{ x: 5, y: 5 }])).toBeNull();
  });

  test("formatExtents reads width × depth in feet/inches", () => {
    expect(formatExtents(24 * FT, 16 * FT)).toBe(`24' 0" × 16' 0"`);
  });
});
