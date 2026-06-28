import { expect, test } from "bun:test";
import { ToolRunner, DEFAULT_SETTINGS, TOOL_CATALOG } from "./index";

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
  expect(command?.spacingInches).toBe(19.2);
  expect(command?.height).toBe(9 * 384);
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
