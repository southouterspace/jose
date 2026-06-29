import { expect, test } from "bun:test";
import { memberBox, memberBoxes } from "./member-tessellation";

const ft = 384;

test("a vertical stud maps world-Z (up) to Three +Y and centers on its midpoint", () => {
  // A stud on the y=0 wall, from z=0 to z=8ft, at x=2ft.
  const box = memberBox({
    x0: 2 * ft,
    y0: 0,
    z0: 0,
    x1: 2 * ft,
    y1: 0,
    z1: 8 * ft,
    width: 48,
    roleId: 1,
    role: "stud",
  });
  if (!box) {
    throw new Error("expected a box");
  }
  expect(box.length).toBeCloseTo(8, 5); // 8ft tall in Three units
  // Long axis points straight up (+Y) in Three space.
  expect(box.dir.y).toBeCloseTo(1, 5);
  expect(box.dir.x).toBeCloseTo(0, 5);
  expect(box.dir.z).toBeCloseTo(0, 5);
  // Centered halfway up, at the member's world X (→ Three X), world Y=0 (→ Three Z).
  expect(box.center.x).toBeCloseTo(2, 5);
  expect(box.center.y).toBeCloseTo(4, 5);
  expect(box.center.z).toBeCloseTo(0, 5);
  expect(box.width).toBeCloseTo(48 / 384, 5);
});

test("a plate on a wall running along world-Y points along Three +Z", () => {
  // A bottom plate from (10ft,0) to (10ft,12ft) — the wall runs north (world +Y → Three +Z).
  const box = memberBox({
    x0: 10 * ft,
    y0: 0,
    z0: 0,
    x1: 10 * ft,
    y1: 12 * ft,
    z1: 0,
    width: 48,
    roleId: 0,
    role: "plate",
  });
  if (!box) {
    throw new Error("expected a box");
  }
  expect(box.length).toBeCloseTo(12, 5);
  expect(box.dir.z).toBeCloseTo(1, 5); // runs along Three +Z
  expect(box.dir.y).toBeCloseTo(0, 5);
  expect(box.center.x).toBeCloseTo(10, 5);
  expect(box.center.z).toBeCloseTo(6, 5);
});

test("a degenerate (zero-length) member yields no box", () => {
  expect(
    memberBox({
      x0: 0,
      y0: 0,
      z0: 0,
      x1: 0,
      y1: 0,
      z1: 0,
      width: 48,
      roleId: 1,
      role: "stud",
    })
  ).toBeNull();
});

test("memberBoxes drops degenerate members and keeps the rest", () => {
  const boxes = memberBoxes([
    {
      x0: 0,
      y0: 0,
      z0: 0,
      x1: 0,
      y1: 0,
      z1: 8 * ft,
      width: 48,
      roleId: 1,
      role: "stud",
    },
    {
      x0: 0,
      y0: 0,
      z0: 0,
      x1: 0,
      y1: 0,
      z1: 0,
      width: 48,
      roleId: 1,
      role: "stud",
    },
  ]);
  expect(boxes).toHaveLength(1);
});
