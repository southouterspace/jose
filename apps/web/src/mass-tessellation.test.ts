import { expect, test } from "bun:test";
import {
  frameMass,
  planToThree,
  TICKS_PER_UNIT,
  tessellateMass,
} from "./mass-tessellation";

test("a plan vertex maps world-X→Three X and world-Y→Three Z, height→+Y", () => {
  // 2ft east, 3ft north, lifted 8ft.
  const p = planToThree({ x: 2 * 384, y: 3 * 384 }, 8 * 384);
  expect(p.x).toBe(2);
  expect(p.z).toBe(3);
  expect(p.y).toBe(8); // height runs up +Y
});

test("the base ring sits on the ground (Y=0) and the top ring is lifted to +height", () => {
  const footprint = [
    { x: 0, y: 0 },
    { x: 10 * 384, y: 0 },
    { x: 10 * 384, y: 8 * 384 },
    { x: 0, y: 8 * 384 },
  ];
  const { base, top } = tessellateMass(footprint, 8 * 384);

  expect(base).toHaveLength(4);
  expect(top).toHaveLength(4);

  // Every base corner is on the ground.
  for (const c of base) {
    expect(c.y).toBe(0);
  }
  // Every top corner is lifted to +8 (8ft), preserving footprint order in XZ.
  for (let i = 0; i < top.length; i++) {
    const ti = top[i];
    const bi = base[i];
    if (!(ti && bi)) {
      throw new Error("missing ring corner");
    }
    expect(ti.y).toBe(8);
    expect(ti.x).toBe(bi.x);
    expect(ti.z).toBe(bi.z);
  }

  // Footprint X/Z preserved: second corner is 10ft east, third is 8ft north too.
  expect(base[1]?.x).toBe(10);
  expect(base[2]?.z).toBe(8);
});

test("TICKS_PER_UNIT is one foot so the scene stays compact", () => {
  expect(TICKS_PER_UNIT).toBe(384);
});

test("frameMass pivots on the footprint centroid, even far from the world origin", () => {
  // A 4ft × 4ft square placed 100ft east / 50ft north — nowhere near origin.
  const ox = 100 * 384;
  const oy = 50 * 384;
  const footprint = [
    { x: ox, y: oy },
    { x: ox + 4 * 384, y: oy },
    { x: ox + 4 * 384, y: oy + 4 * 384 },
    { x: ox, y: oy + 4 * 384 },
  ];
  const { target, camera } = frameMass(footprint, 8 * 384);

  // Centroid is 2ft into the square: (102ft, 52ft) → Three (X=102, Z=52).
  expect(target.x).toBeCloseTo(102, 5);
  expect(target.z).toBeCloseTo(52, 5);
  // Pivot Y is ~half the 8ft height.
  expect(target.y).toBeCloseTo(4, 5);

  // The camera sits offset from the centroid (not at origin), above the ground, and within
  // a sane distance of the pivot — so the mass is framed wherever it was drawn.
  expect(camera.y).toBeGreaterThan(target.y);
  const dist = Math.hypot(
    camera.x - target.x,
    camera.y - target.y,
    camera.z - target.z
  );
  expect(dist).toBeGreaterThan(0);
  // Far from origin: camera X/Z are near the centroid, not near 0.
  expect(camera.x).toBeGreaterThan(90);
  expect(camera.z).toBeGreaterThan(40);
});

test("frameMass distance grows with footprint size so a bigger mass still fits", () => {
  const small = frameMass(
    [
      { x: 0, y: 0 },
      { x: 4 * 384, y: 0 },
      { x: 4 * 384, y: 4 * 384 },
      { x: 0, y: 4 * 384 },
    ],
    8 * 384
  );
  const big = frameMass(
    [
      { x: 0, y: 0 },
      { x: 40 * 384, y: 0 },
      { x: 40 * 384, y: 40 * 384 },
      { x: 0, y: 40 * 384 },
    ],
    8 * 384
  );
  const distOf = (f: typeof small): number =>
    Math.hypot(
      f.camera.x - f.target.x,
      f.camera.y - f.target.y,
      f.camera.z - f.target.z
    );
  expect(distOf(big)).toBeGreaterThan(distOf(small));
});
