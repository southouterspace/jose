import { describe, expect, test } from "bun:test";
import { pushPullReadout } from "./hud";

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
