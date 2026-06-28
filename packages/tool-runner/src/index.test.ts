import { expect, test } from "bun:test";
import { DEFAULT_SETTINGS, TOOL_CATALOG, ToolRunner } from "./index";

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
