import { expect, test } from "bun:test";
import {
  DEFAULT_SETTINGS,
  formatLength,
  inferAlignment,
  parseLength,
  parsePolarLength,
  parseSize,
  pointAtAngle,
  pointAtDistance,
  pushPullDistance,
  rectangleCorner,
  rectangleRing,
  TOOL_CATALOG,
  ToolRunner,
} from "./index";

test("the wall tool emits a DrawWall command on the second pick", () => {
  const runner = new ToolRunner();
  expect(runner.activeKey).toBe("wall");

  // First pick: tool not yet satisfied.
  expect(runner.pick({ x: 0, y: 0 })).toBeNull();
  expect(runner.pendingPicks).toHaveLength(1);

  // Second pick: commits and resets.
  const command = runner.pick({ x: 10 * 384, y: 0 });
  expect(command).not.toBeNull();
  expect(command).toEqual({
    kind: "drawWall",
    x0: 0,
    y0: 0,
    x1: 10 * 384,
    y1: 0,
    height: DEFAULT_SETTINGS.wallHeightTicks,
    spacingInches: DEFAULT_SETTINGS.spacingInches,
  });
  expect(runner.pendingPicks).toHaveLength(0);
});

test("the rectangle tool emits a closed 4-corner DrawFootprint on the second pick", () => {
  const runner = new ToolRunner(DEFAULT_SETTINGS, "rectangle");
  expect(runner.activeKey).toBe("rectangle");
  expect(runner.pick({ x: 0, y: 0 })).toBeNull();
  const command = runner.pick({ x: 24 * 384, y: 16 * 384 });
  expect(command).toEqual({
    kind: "drawFootprint",
    xs: [0, 24 * 384, 24 * 384, 0],
    ys: [0, 0, 16 * 384, 16 * 384],
  });
  expect(runner.pendingPicks).toHaveLength(0);
});

test("rectangleRing / rectangleCorner span an axis-aligned box from two corners", () => {
  expect(rectangleRing({ x: 0, y: 0 }, { x: 10, y: 6 })).toEqual([
    { x: 0, y: 0 },
    { x: 10, y: 0 },
    { x: 10, y: 6 },
    { x: 0, y: 6 },
  ]);
  // A typed size grows toward the cursor's quadrant (here: down-left of the anchor).
  expect(rectangleCorner({ x: 100, y: 100 }, { x: 40, y: 40 }, 24, 16)).toEqual(
    {
      x: 76,
      y: 84,
    }
  );
  // A zero cursor delta defaults to the +X/+Y quadrant.
  expect(
    rectangleCorner({ x: 0, y: 0 }, { x: 0, y: 0 }, 24 * 384, 16 * 384)
  ).toEqual({ x: 24 * 384, y: 16 * 384 });
});

test("parseSize reads a W,D pair (comma, x, or ×) into ticks, else null", () => {
  expect(parseSize("24', 16'")).toEqual({ width: 24 * 384, depth: 16 * 384 });
  expect(parseSize("24'x16'")).toEqual({ width: 24 * 384, depth: 16 * 384 });
  expect(parseSize("12 × 8")).toEqual({ width: 12 * 384, depth: 8 * 384 });
  // Both parts must name a positive length.
  expect(parseSize("24'")).toBeNull();
  expect(parseSize("24', abc")).toBeNull();
  expect(parseSize("")).toBeNull();
});

test("picks snap to the tick grid", () => {
  const runner = new ToolRunner({ ...DEFAULT_SETTINGS, gridTicks: 32 });
  // 33 ticks snaps down to 1in (32); 48 ticks is exactly 1.5in and rounds up to 2in (64).
  expect(runner.snap({ x: 33, y: 48 })).toEqual({ x: 32, y: 64 });
  runner.pick({ x: 33, y: 48 });
  expect(runner.pendingPicks[0]).toEqual({ x: 32, y: 64 });
});

test("configure changes the value grammar applied to the next commit", () => {
  const runner = new ToolRunner();
  runner.configure({ spacingInches: 19.2, wallHeightTicks: 9 * 384 });
  runner.pick({ x: 0, y: 0 });
  const command = runner.pick({ x: 3840, y: 0 });
  expect(command?.kind).toBe("drawWall");
  if (command?.kind !== "drawWall") {
    throw new Error("expected a drawWall command");
  }
  expect(command.spacingInches).toBe(19.2);
  expect(command.height).toBe(9 * 384);
});

test("cancel clears an in-progress operation", () => {
  const runner = new ToolRunner();
  runner.pick({ x: 100, y: 100 });
  expect(runner.pendingPicks).toHaveLength(1);
  runner.cancel();
  expect(runner.pendingPicks).toHaveLength(0);
});

test("activating an unknown tool throws", () => {
  const runner = new ToolRunner();
  expect(() => runner.activate("nope")).toThrow(/unknown tool/);
  expect(TOOL_CATALOG.wall?.picks).toBe(2);
});

test("the footprint tool emits no command until the ring closes", () => {
  const runner = new ToolRunner();
  runner.activate("footprint");
  expect(runner.activeKey).toBe("footprint");

  // A 10ft x 8ft rectangle (ticks: 1ft = 384).
  expect(runner.pick({ x: 0, y: 0 })).toBeNull();
  expect(runner.pick({ x: 3840, y: 0 })).toBeNull();
  expect(runner.pick({ x: 3840, y: 3072 })).toBeNull();
  expect(runner.pick({ x: 0, y: 3072 })).toBeNull();
  expect(runner.pendingPicks).toHaveLength(4);

  // Clicking back near the first vertex (≥3 points down) closes the ring.
  const command = runner.pick({ x: 0, y: 0 });
  expect(command).toEqual({
    kind: "drawFootprint",
    xs: [0, 3840, 3840, 0],
    ys: [0, 0, 3072, 3072],
  });
  // Resets for the next operation; the closing click is not re-committed as a vertex.
  expect(runner.pendingPicks).toHaveLength(0);
});

test("the footprint closes on a click within the snap threshold of the first vertex", () => {
  const runner = new ToolRunner();
  runner.activate("footprint");
  runner.pick({ x: 0, y: 0 });
  runner.pick({ x: 3840, y: 0 });
  runner.pick({ x: 3840, y: 3072 });
  // A near-but-not-exact closing click snaps onto the first vertex.
  const command = runner.pick({ x: 20, y: -16 });
  expect(command).not.toBeNull();
  expect(command).toEqual({
    kind: "drawFootprint",
    xs: [0, 3840, 3840],
    ys: [0, 0, 3072],
  });
});

test("a near-first click with fewer than 3 points does not close (it is a new vertex)", () => {
  const runner = new ToolRunner();
  runner.activate("footprint");
  runner.pick({ x: 0, y: 0 });
  // Only 2 points so far: clicking near the first is just the 3rd vertex, no command.
  expect(runner.pick({ x: 16, y: 0 })).toBeNull();
  expect(runner.pendingPicks).toHaveLength(2);
});

test("footprint picks snap to the tick grid", () => {
  const runner = new ToolRunner();
  runner.activate("footprint");
  runner.pick({ x: 33, y: 48 });
  expect(runner.pendingPicks[0]).toEqual({ x: 32, y: 64 });
});

test("activating the footprint tool clears any in-progress wall operation", () => {
  const runner = new ToolRunner();
  runner.pick({ x: 100, y: 100 });
  runner.activate("footprint");
  expect(runner.pendingPicks).toHaveLength(0);
  expect(TOOL_CATALOG.footprint?.key).toBe("footprint");
});

test("pushpull is a gesture tool, not a runner pick-tool, so the runner must not be asked to activate it", () => {
  // Regression: the store's `activate` branches on `key in TOOL_CATALOG`. `pushpull` is the 3D
  // drag gesture, NOT a runner tool — forwarding it to the runner throws "unknown tool".
  expect("footprint" in TOOL_CATALOG).toBe(true);
  expect("pushpull" in TOOL_CATALOG).toBe(false);

  const runner = new ToolRunner(undefined, "footprint");
  // Mid-draw, then "switch to pushpull" the store's way: cancel (not activate) — must not throw.
  runner.pick({ x: 0, y: 0 });
  expect(runner.pendingPicks).toHaveLength(1);
  expect(() => runner.cancel()).not.toThrow();
  expect(runner.pendingPicks).toHaveLength(0);

  // Switching back to footprint re-activates the runner tool and plan drawing still works.
  runner.activate("footprint");
  expect(runner.activeKey).toBe("footprint");
  expect(runner.pick({ x: 100, y: 100 })).toBeNull();
  expect(runner.pendingPicks).toHaveLength(1);
});

test("dragging the top cap up (negative pixel delta) raises the mass", () => {
  // Screen Y is down-positive, so an upward drag is a negative pixel delta.
  // scale = 4 ticks/px → 100px up = +400 ticks (raise).
  expect(pushPullDistance(-100, 4)).toBe(400);
});

test("dragging the top cap down (positive pixel delta) lowers the mass", () => {
  expect(pushPullDistance(50, 4)).toBe(-200);
});

test("a zero pointer delta yields exactly zero distance (no recompute)", () => {
  expect(pushPullDistance(0, 4)).toBe(0);
});

test("push/pull distance honors the ticks-per-pixel scale and rounds to whole ticks", () => {
  expect(pushPullDistance(-10, 0.5)).toBe(5);
  // -7px * 1.3 = 9.1 → rounds to 9 whole ticks.
  expect(pushPullDistance(-7, 1.3)).toBe(9);
});

test("axisLock snaps the new edge to the dominant axis relative to the previous pick", () => {
  const runner = new ToolRunner(undefined, "footprint");
  runner.pick({ x: 0, y: 0 });

  // Mostly-horizontal move (|dx| > |dy|) with axis lock → Y collapses back to the anchor's Y.
  expect(
    runner.pick({ x: 10 * 384, y: 2 * 384 }, { axisLock: true })
  ).toBeNull();
  expect(runner.pendingPicks[1]).toEqual({ x: 10 * 384, y: 0 });

  // Mostly-vertical move (|dy| > |dx|) with axis lock → X collapses back to the prior vertex's X.
  runner.pick({ x: 9 * 384, y: 12 * 384 }, { axisLock: true });
  expect(runner.pendingPicks[2]).toEqual({ x: 10 * 384, y: 12 * 384 });
});

test("axisLock is a no-op for the very first pick (no anchor to lock against)", () => {
  const runner = new ToolRunner(undefined, "footprint");
  runner.pick({ x: 3 * 384, y: 7 * 384 }, { axisLock: true });
  expect(runner.pendingPicks[0]).toEqual({ x: 3 * 384, y: 7 * 384 });
});

test("the wall tool axis-locks its second pick to the first", () => {
  const runner = new ToolRunner(); // wall tool by default
  runner.pick({ x: 0, y: 0 });
  // A near-horizontal second pick snaps onto the X axis of the first.
  const command = runner.pick({ x: 8 * 384, y: 384 }, { axisLock: true });
  expect(command).toMatchObject({
    kind: "drawWall",
    x0: 0,
    y0: 0,
    x1: 8 * 384,
    y1: 0,
  });
});

test("parseLength reads feet/inches grammar into ticks (1ft=384, 1in=32)", () => {
  expect(parseLength("10")).toBe(10 * 384); // bare number = feet
  expect(parseLength("10.5")).toBe(Math.round(10.5 * 384));
  expect(parseLength("10'")).toBe(10 * 384);
  expect(parseLength("10' 6\"")).toBe(10 * 384 + 6 * 32);
  expect(parseLength("10'6")).toBe(10 * 384 + 6 * 32); // trailing number = inches
  expect(parseLength('6"')).toBe(6 * 32);
  expect(parseLength("6in")).toBe(6 * 32);
  expect(parseLength("3ft 7in")).toBe(3 * 384 + 7 * 32);
});

test("parseLength rejects empty, non-numeric, and non-positive input", () => {
  expect(parseLength("")).toBeNull();
  expect(parseLength("   ")).toBeNull();
  expect(parseLength("abc")).toBeNull();
  expect(parseLength("0")).toBeNull();
  expect(parseLength("-5")).toBeNull();
});

test("formatLength renders ticks as feet and inches", () => {
  expect(formatLength(12 * 384)).toBe(`12' 0"`);
  expect(formatLength(0)).toBe(`0' 0"`);
  expect(formatLength(384 + 6 * 32)).toBe(`1' 6"`); // 1ft 6in
});

test("pointAtDistance places a point an exact distance along the cursor direction", () => {
  // Cursor 100 ticks east, length 5ft (1920 ticks) → exactly 1920 east of the anchor.
  expect(pointAtDistance({ x: 0, y: 0 }, { x: 100, y: 0 }, 1920)).toEqual({
    x: 1920,
    y: 0,
  });
  // A diagonal direction scales to the requested length (3-4-5: dir (3,4), len 5 → (3,4)).
  expect(pointAtDistance({ x: 0, y: 0 }, { x: 30, y: 40 }, 5)).toEqual({
    x: 3,
    y: 4,
  });
  // axisLock collapses the off-axis component before scaling.
  expect(
    pointAtDistance({ x: 0, y: 0 }, { x: 100, y: 20 }, 1920, true)
  ).toEqual({ x: 1920, y: 0 });
  // A zero-length direction still yields a segment (defaults to +X).
  expect(pointAtDistance({ x: 5, y: 5 }, { x: 5, y: 5 }, 384)).toEqual({
    x: 389,
    y: 5,
  });
});

test("pointAtAngle places a point at an absolute bearing (CCW from +X, world Y up)", () => {
  // Due east: cos0=1, sin0=0.
  expect(pointAtAngle({ x: 0, y: 0 }, 1920, 0)).toEqual({ x: 1920, y: 0 });
  // Due north (world Y up): 90° → +Y.
  expect(pointAtAngle({ x: 0, y: 0 }, 1920, 90)).toEqual({ x: 0, y: 1920 });
  // A negative bearing drops below the anchor.
  expect(pointAtAngle({ x: 10, y: 10 }, 384, -90)).toEqual({ x: 10, y: -374 });
});

test("parsePolarLength reads a length with an optional < angle clause", () => {
  // Bare length: no angle → cursor direction (null).
  expect(parsePolarLength("10' 6\"")).toEqual({
    lengthTicks: 10 * 384 + 6 * 32,
    angleDegrees: null,
  });
  // Length < angle (both `<` and `∠` separate them).
  expect(parsePolarLength("12<90")).toEqual({
    lengthTicks: 12 * 384,
    angleDegrees: 90,
  });
  expect(parsePolarLength("8' ∠ -30")).toEqual({
    lengthTicks: 8 * 384,
    angleDegrees: -30,
  });
  // A blank or unparseable angle clause falls back to the cursor direction, not a rejection.
  expect(parsePolarLength("8'<")).toEqual({
    lengthTicks: 8 * 384,
    angleDegrees: null,
  });
  expect(parsePolarLength("8'<abc")).toEqual({
    lengthTicks: 8 * 384,
    angleDegrees: null,
  });
  // No positive length → null (the length is the required part).
  expect(parsePolarLength("<45")).toBeNull();
  expect(parsePolarLength("")).toBeNull();
});

test("inferAlignment snaps a point onto an existing vertex's row/column within tolerance", () => {
  const vertices = [
    { x: 0, y: 0 },
    { x: 3840, y: 0 },
  ];
  // Cursor near the column of vertex 0 (x≈0) and the row of vertex 1 (y≈0): snaps both, two guides.
  const result = inferAlignment({ x: 40, y: 30 }, vertices, 120);
  expect(result.point).toEqual({ x: 0, y: 0 });
  expect(result.guides).toContainEqual({
    orientation: "vertical",
    atTicks: 0,
    sourceIndex: 0,
  });
  expect(result.guides).toContainEqual({
    orientation: "horizontal",
    atTicks: 0,
    sourceIndex: 0,
  });
});

test("inferAlignment leaves a point alone when no vertex is within tolerance", () => {
  const result = inferAlignment({ x: 5000, y: 5000 }, [{ x: 0, y: 0 }], 120);
  expect(result.point).toEqual({ x: 5000, y: 5000 });
  expect(result.guides).toHaveLength(0);
});

test("draft previews the resolved point and close target without committing", () => {
  const runner = new ToolRunner(undefined, "footprint");
  runner.pick({ x: 0, y: 0 });
  runner.pick({ x: 3840, y: 0 });
  runner.pick({ x: 3840, y: 3072 });

  // Hovering near the first vertex (≥3 down) previews a closing draft snapped onto it — no commit.
  const closing = runner.draft({ x: 40, y: 40 });
  expect(closing.closing).toBe(true);
  expect(closing.point).toEqual({ x: 0, y: 0 });
  expect(runner.pendingPicks).toHaveLength(3); // draft must not mutate the in-progress picks
});

test("draft surfaces alignment guides toward existing vertices", () => {
  const runner = new ToolRunner(undefined, "footprint");
  runner.pick({ x: 0, y: 0 });
  // Cursor roughly above the first vertex's column → a vertical guide snapping x back to 0.
  const aligned = runner.draft({ x: 24, y: 3000 });
  expect(aligned.point.x).toBe(0);
  expect(aligned.guides.some((g) => g.orientation === "vertical")).toBe(true);
});

test("an exact pick bypasses grid snap so a typed length is honored", () => {
  const runner = new ToolRunner(undefined, "footprint");
  runner.pick({ x: 0, y: 0 });
  // 5ft (1920 ticks) is on-grid; nudge by 7 ticks to prove exact entry is not re-snapped to 1in.
  runner.pick({ x: 1927, y: 0 }, { exact: true });
  expect(runner.pendingPicks[1]).toEqual({ x: 1927, y: 0 });
});
