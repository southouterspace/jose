import { describe, expect, test } from "bun:test";
import {
  boundsOf,
  DEFAULT_CAMERA,
  fitToBounds,
  MAX_SCALE,
  MIN_SCALE,
  type PlanCamera,
  panBy,
  toScreenX,
  toScreenY,
  toWorldX,
  toWorldY,
  VIEW_H,
  VIEW_W,
  zoomAt,
} from "./plan-camera";

/** Round to the nearest pixel-hundredth to compare floating-point transforms. */
const near = (v: number): number => Math.round(v * 100) / 100;

describe("world↔screen round-trip", () => {
  test("toWorld inverts toScreen at the default camera", () => {
    for (const cam of [
      DEFAULT_CAMERA,
      { scale: 0.2, offsetX: 30, offsetY: 500 },
    ]) {
      for (const t of [-4000, -100, 0, 384, 5000]) {
        expect(near(toWorldX(cam, toScreenX(cam, t)))).toBe(near(t));
        expect(near(toWorldY(cam, toScreenY(cam, t)))).toBe(near(t));
      }
    }
  });

  test("world Y is up while screen Y is down", () => {
    // A larger world Y sits *higher* on screen, i.e. a smaller screen-Y pixel.
    expect(toScreenY(DEFAULT_CAMERA, 1000)).toBeLessThan(
      toScreenY(DEFAULT_CAMERA, 0)
    );
  });
});

describe("zoomAt", () => {
  test("keeps the world point under the cursor pinned to the same pixel", () => {
    const px = 220;
    const py = 410;
    const before = {
      x: toWorldX(DEFAULT_CAMERA, px),
      y: toWorldY(DEFAULT_CAMERA, py),
    };
    const zoomed = zoomAt(DEFAULT_CAMERA, px, py, 1.25);
    expect(near(toScreenX(zoomed, before.x))).toBe(px);
    expect(near(toScreenY(zoomed, before.y))).toBe(py);
    expect(zoomed.scale).toBeGreaterThan(DEFAULT_CAMERA.scale);
  });

  test("clamps zoom at the limits so the view can't invert or vanish", () => {
    let cam: PlanCamera = DEFAULT_CAMERA;
    for (let i = 0; i < 100; i++) {
      cam = zoomAt(cam, 320, 320, 2);
    }
    expect(cam.scale).toBe(MAX_SCALE);
    for (let i = 0; i < 100; i++) {
      cam = zoomAt(cam, 320, 320, 0.5);
    }
    expect(cam.scale).toBe(MIN_SCALE);
  });
});

describe("panBy", () => {
  test("shifts the offset by the pixel delta and leaves scale untouched", () => {
    const panned = panBy(DEFAULT_CAMERA, 40, -25);
    expect(panned.offsetX).toBe(DEFAULT_CAMERA.offsetX + 40);
    expect(panned.offsetY).toBe(DEFAULT_CAMERA.offsetY - 25);
    expect(panned.scale).toBe(DEFAULT_CAMERA.scale);
  });
});

describe("boundsOf", () => {
  test("returns null for no points", () => {
    expect(boundsOf([])).toBeNull();
  });

  test("brackets every point", () => {
    expect(
      boundsOf([
        { x: 10, y: -5 },
        { x: -3, y: 20 },
        { x: 8, y: 8 },
      ])
    ).toEqual({ minX: -3, minY: -5, maxX: 10, maxY: 20 });
  });
});

describe("fitToBounds (Zoom-Extents)", () => {
  test("empty content falls back to the default camera", () => {
    expect(fitToBounds(null)).toEqual(DEFAULT_CAMERA);
  });

  test("centers the geometry in the viewBox", () => {
    const cam = fitToBounds({ minX: 0, minY: 0, maxX: 4000, maxY: 2000 });
    // The box center must land at the viewBox center.
    expect(near(toScreenX(cam, 2000))).toBe(VIEW_W / 2);
    expect(near(toScreenY(cam, 1000))).toBe(VIEW_H / 2);
  });

  test("fits the geometry within the padded viewBox", () => {
    const bounds = { minX: -1000, minY: -1000, maxX: 5000, maxY: 3000 };
    const cam = fitToBounds(bounds);
    // Every corner stays inside the viewBox once fitted.
    for (const x of [bounds.minX, bounds.maxX]) {
      const sx = toScreenX(cam, x);
      expect(sx).toBeGreaterThanOrEqual(0);
      expect(sx).toBeLessThanOrEqual(VIEW_W);
    }
    for (const y of [bounds.minY, bounds.maxY]) {
      const sy = toScreenY(cam, y);
      expect(sy).toBeGreaterThanOrEqual(0);
      expect(sy).toBeLessThanOrEqual(VIEW_H);
    }
  });

  test("a degenerate (zero-area) box does not blow up the scale", () => {
    const cam = fitToBounds({ minX: 100, minY: 100, maxX: 100, maxY: 900 });
    expect(cam.scale).toBeGreaterThanOrEqual(MIN_SCALE);
    expect(cam.scale).toBeLessThanOrEqual(MAX_SCALE);
    // Still centers on the run.
    expect(near(toScreenX(cam, 100))).toBe(VIEW_W / 2);
    expect(near(toScreenY(cam, 500))).toBe(VIEW_H / 2);
  });
});
