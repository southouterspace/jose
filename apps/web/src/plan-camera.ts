/**
 * The plan view's 2D camera: a pan/zoom transform between world ticks and the SVG's fixed viewBox
 * pixel space. Pure math — no React, no DOM — so pan, zoom-to-cursor, and Zoom-Extents are unit-tested
 * directly (`plan-camera.test.ts`). `plan-view.tsx` holds one `PlanCamera` in React state and derives
 * every rendered coordinate from it.
 *
 * Convention: world Y is up, screen Y is down; `scale` is screen px per world tick (1 tick = 1/32in);
 * `offsetX`/`offsetY` are the viewBox pixel at which world (0, 0) lands. So:
 *   screenX = worldX * scale + offsetX
 *   screenY = offsetY - worldY * scale
 */

/** The SVG viewBox is a fixed 640×640 pixel space; the `<svg>` element scales it to its container. */
export const VIEW_W = 640;
export const VIEW_H = 640;

/** Default zoom: screen px per world tick. ~0.05 px/tick ≈ a 10ft wall spans ~192px. */
const DEFAULT_SCALE = 0.05;
/** World-tick offset that seats the origin comfortably in view at the default zoom. */
const ORIGIN_TICKS = { x: 1920, y: 1920 };

/** Zoom limits (px/tick): out to ~1/10 the default, in to 40× it — the view can't invert or vanish. */
export const MIN_SCALE = 0.005;
export const MAX_SCALE = 2;

export interface PlanCamera {
  /** ViewBox pixel where world x = 0 lands. */
  readonly offsetX: number;
  /** ViewBox pixel where world y = 0 lands. */
  readonly offsetY: number;
  /** Screen pixels per world tick. */
  readonly scale: number;
}

/** The opening view: origin seated in the lower-left quadrant at the default zoom (the prior fixed frame). */
export const DEFAULT_CAMERA: PlanCamera = {
  scale: DEFAULT_SCALE,
  offsetX: ORIGIN_TICKS.x * DEFAULT_SCALE,
  offsetY: VIEW_H - ORIGIN_TICKS.y * DEFAULT_SCALE,
};

const clampScale = (scale: number): number =>
  Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));

export const toScreenX = (cam: PlanCamera, xTicks: number): number =>
  xTicks * cam.scale + cam.offsetX;
export const toScreenY = (cam: PlanCamera, yTicks: number): number =>
  cam.offsetY - yTicks * cam.scale;
export const toWorldX = (cam: PlanCamera, px: number): number =>
  (px - cam.offsetX) / cam.scale;
export const toWorldY = (cam: PlanCamera, py: number): number =>
  (cam.offsetY - py) / cam.scale;

/** Zoom by `factor` about a viewBox pixel, keeping the world point under that pixel fixed (zoom-to-cursor). */
export function zoomAt(
  cam: PlanCamera,
  px: number,
  py: number,
  factor: number
): PlanCamera {
  const scale = clampScale(cam.scale * factor);
  // The world point under the cursor must map back to the same pixel after the scale change.
  const wx = toWorldX(cam, px);
  const wy = toWorldY(cam, py);
  return { scale, offsetX: px - wx * scale, offsetY: py + wy * scale };
}

/** Pan by a viewBox-pixel delta (drag). Scale is unchanged; the offset shifts with the cursor. */
export function panBy(cam: PlanCamera, dpx: number, dpy: number): PlanCamera {
  return {
    scale: cam.scale,
    offsetX: cam.offsetX + dpx,
    offsetY: cam.offsetY + dpy,
  };
}

export interface Bounds {
  readonly maxX: number;
  readonly maxY: number;
  readonly minX: number;
  readonly minY: number;
}

/** World-space bounds of a set of points, or `null` when there are none. */
export function boundsOf(
  points: readonly { readonly x: number; readonly y: number }[]
): Bounds | null {
  if (points.length === 0) {
    return null;
  }
  let minX = Number.POSITIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;
  for (const p of points) {
    minX = Math.min(minX, p.x);
    minY = Math.min(minY, p.y);
    maxX = Math.max(maxX, p.x);
    maxY = Math.max(maxY, p.y);
  }
  return { minX, minY, maxX, maxY };
}

/** Padding (viewBox px) left around geometry when fitting it to the view. */
const FIT_PADDING = 48;

/**
 * Zoom-Extents: frame `bounds` in the viewBox with padding. A degenerate box — no geometry, a single
 * point, or a perfectly straight run — falls back to the default zoom centered on the geometry rather
 * than dividing by a zero span.
 */
export function fitToBounds(bounds: Bounds | null): PlanCamera {
  if (!bounds) {
    return DEFAULT_CAMERA;
  }
  const w = bounds.maxX - bounds.minX;
  const h = bounds.maxY - bounds.minY;
  const cx = (bounds.minX + bounds.maxX) / 2;
  const cy = (bounds.minY + bounds.maxY) / 2;
  const usableW = VIEW_W - 2 * FIT_PADDING;
  const usableH = VIEW_H - 2 * FIT_PADDING;
  // Fit to the tighter axis; a zero span on one axis doesn't constrain the scale.
  const scale =
    w > 0 || h > 0
      ? clampScale(
          Math.min(
            w > 0 ? usableW / w : MAX_SCALE,
            h > 0 ? usableH / h : MAX_SCALE
          )
        )
      : DEFAULT_SCALE;
  return {
    scale,
    offsetX: VIEW_W / 2 - cx * scale,
    offsetY: VIEW_H / 2 + cy * scale,
  };
}
