/**
 * Pure presentation tessellation: footprint (world-XY ticks) + extrusion height (ticks) → the box
 * mass the 3D view renders. This is display-only — the engine owns the canonical geometry (ADR
 * 0006/0008 §2); the 2D analogue is stroking a polyline. No Three.js types here so it stays a unit
 * (the component feeds these corners into a `Shape`/`ExtrudeGeometry`).
 *
 * Axis mapping (world → Three): world-X → Three X, world-Y → Three Z, height → Three +Y (up).
 * World ticks scale to Three world units by `TICKS_PER_UNIT` so the camera/grid stay in a sane
 * range; the mapping is linear, so push/pull pixel math is unaffected.
 */

/** A 2D footprint vertex in world ticks (matches `FootprintVertex` shape, dependency-free here). */
export interface PlanPoint {
  readonly x: number;
  readonly y: number;
}

/** A point in Three.js scene space (world units). */
export interface ThreePoint {
  readonly x: number;
  readonly y: number;
  readonly z: number;
}

/** Three world units per world tick. 384 ticks = 1ft, so 1 unit = 1ft keeps the scene compact. */
export const TICKS_PER_UNIT = 384;

/** Project a plan footprint vertex (world ticks) onto the Three ground plane (XZ), at a given Y. */
export function planToThree(point: PlanPoint, heightTicks: number): ThreePoint {
  return {
    x: point.x / TICKS_PER_UNIT,
    y: heightTicks / TICKS_PER_UNIT,
    z: point.y / TICKS_PER_UNIT,
  };
}

/** The tessellated mass: the footprint's base ring (Y=0) and top ring (Y=height), in Three space. */
export interface MassCorners {
  readonly base: readonly ThreePoint[];
  readonly top: readonly ThreePoint[];
}

/**
 * Tessellate a footprint + height into base + top rings in Three space. The base sits on the ground
 * (Y=0); the top cap is lifted to `+height` along +Y — the face push/pull drags. Rings preserve
 * footprint order so the side quads connect 1:1.
 */
export function tessellateMass(
  footprint: readonly PlanPoint[],
  heightTicks: number
): MassCorners {
  const base = footprint.map((p) => planToThree(p, 0));
  const top = footprint.map((p) => planToThree(p, heightTicks));
  return { base, top };
}

/** How the 3D camera should frame the mass: where it pivots and where it sits (Three units). */
export interface MassFraming {
  /** The camera position: an angled offset from `target`, far enough to fit the whole mass. */
  readonly camera: ThreePoint;
  /** The orbit pivot — the footprint centroid in XZ, lifted to ~half the mass height in Y. */
  readonly target: ThreePoint;
}

/** Unit vector of the angled orbit direction (looking down from the +X/+Y/+Z octant). */
const VIEW_DIR = (() => {
  const x = 0.66;
  const y = 0.62;
  const z = 0.77;
  const len = Math.hypot(x, y, z);
  return { x: x / len, y: y / len, z: z / len };
})();

/** Extra room beyond the mass's bounding radius, so it sits comfortably (not edge-to-edge). */
const FRAME_MARGIN = 2.2;
/** A floor on the framing distance so a tiny footprint isn't framed uselessly close. */
const MIN_FRAME_DISTANCE = 12;

/**
 * Compute where to put the orbit pivot and camera to frame a mass wherever it is drawn. The pivot is
 * the footprint centroid (mean of the Three X/Z vertices) at half-height; the camera sits along a
 * fixed angled direction at a distance derived from the footprint's bounding size × margin — so the
 * whole mass is in frame regardless of world position. Pure (no Three.js), so it is unit-testable.
 */
export function frameMass(
  footprint: readonly PlanPoint[],
  heightTicks: number
): MassFraming {
  const corners = footprint.map((p) => planToThree(p, 0));
  const n = corners.length || 1;
  let sx = 0;
  let sz = 0;
  let minX = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let minZ = Number.POSITIVE_INFINITY;
  let maxZ = Number.NEGATIVE_INFINITY;
  for (const c of corners) {
    sx += c.x;
    sz += c.z;
    minX = Math.min(minX, c.x);
    maxX = Math.max(maxX, c.x);
    minZ = Math.min(minZ, c.z);
    maxZ = Math.max(maxZ, c.z);
  }
  const heightUnits = heightTicks / TICKS_PER_UNIT;
  const centerY = heightUnits / 2;
  const target: ThreePoint = { x: sx / n, y: centerY, z: sz / n };

  // Bounding radius spans the footprint diagonal and the full height; margin gives breathing room.
  const spanX = corners.length > 0 ? maxX - minX : 0;
  const spanZ = corners.length > 0 ? maxZ - minZ : 0;
  const radius = Math.hypot(spanX, spanZ, heightUnits) / 2;
  const distance = Math.max(radius * FRAME_MARGIN, MIN_FRAME_DISTANCE);

  const camera: ThreePoint = {
    x: target.x + VIEW_DIR.x * distance,
    y: target.y + VIEW_DIR.y * distance,
    z: target.z + VIEW_DIR.z * distance,
  };
  return { target, camera };
}
