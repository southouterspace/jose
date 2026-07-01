/**
 * The HUD layer's pure text helpers — the measurement labels the drawing surfaces overlay on a
 * gesture (ADR 0012 §3). Kept free of React/Three so it unit-tests without a scene and can be shared
 * by both the plan (SVG) and 3D (HTML overlay) surfaces as the HUD grows. Today it holds the
 * push/pull readout; segment length+angle and snap badges join it here.
 */

import { formatLength } from "@jose/tool-runner";

/**
 * The push/pull readout: the resulting mass height and — once the drag has moved — the signed
 * distance being applied. This is what lets you *see* the push/pull distance instead of dragging the
 * top cap blind. `startHeightTicks` is the cap's height when the drag began (0 for a flat,
 * not-yet-extruded face); `distanceTicks` is the live signed drag delta. The resulting height is
 * clamped at 0 — the height a non-positive drag would produce, where the mass vanishes (a state
 * whose full treatment is still an open product decision, `surfaces-3d-view.md`).
 */
export function pushPullReadout(
  startHeightTicks: number,
  distanceTicks: number
): string {
  const height = formatLength(Math.max(0, startHeightTicks + distanceTicks));
  if (distanceTicks === 0) {
    return height; // No drag yet — just name the current height.
  }
  const arrow = distanceTicks > 0 ? "▲" : "▼";
  return `${height}  ${arrow} ${formatLength(Math.abs(distanceTicks))}`;
}
