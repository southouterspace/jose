/**
 * Pure presentation tessellation: a framed member's **world** segment (ticks) → the oriented box the
 * 3D view renders. Display-only — the engine owns canonical geometry (ADR 0006/0012); a box stroked
 * along a segment is the 3D analogue of stroking a line in plan. No Three.js types here so it stays a
 * unit (the component turns each `MemberBox` into a `BoxGeometry` + quaternion).
 *
 * Axis mapping matches `mass-tessellation` exactly so members sit inside the mass: world-X → Three X,
 * world-Y → Three Z, world-Z (up) → Three +Y. Coordinates scale by `TICKS_PER_UNIT`.
 */

import { type ThreePoint, TICKS_PER_UNIT } from "./mass-tessellation";

/** A decoded member's world segment + cross-section, as the render mirror hands it over (ticks). */
export interface MemberSegment {
  readonly role: string;
  readonly roleId: number;
  readonly width: number;
  readonly x0: number;
  readonly x1: number;
  readonly y0: number;
  readonly y1: number;
  readonly z0: number;
  readonly z1: number;
}

/** An oriented box: its center, the unit direction of its long axis, and its dimensions (Three units). */
export interface MemberBox {
  readonly center: ThreePoint;
  /** Unit direction of the member's length axis, in Three space. */
  readonly dir: ThreePoint;
  readonly length: number;
  readonly role: string;
  readonly roleId: number;
  /** Square cross-section side (the member's draw width). */
  readonly width: number;
}

/** Map a world tick point (z = up) into Three scene space (y = up), scaled to units. */
function worldToThree(x: number, y: number, z: number): ThreePoint {
  return {
    x: x / TICKS_PER_UNIT,
    y: z / TICKS_PER_UNIT,
    z: y / TICKS_PER_UNIT,
  };
}

/**
 * Tessellate one member segment into an oriented box. Returns `null` for a degenerate (zero-length)
 * member, which has no orientation to render.
 */
export function memberBox(member: MemberSegment): MemberBox | null {
  const s = worldToThree(member.x0, member.y0, member.z0);
  const e = worldToThree(member.x1, member.y1, member.z1);
  const dx = e.x - s.x;
  const dy = e.y - s.y;
  const dz = e.z - s.z;
  const length = Math.hypot(dx, dy, dz);
  if (length === 0) {
    return null;
  }
  return {
    center: { x: (s.x + e.x) / 2, y: (s.y + e.y) / 2, z: (s.z + e.z) / 2 },
    dir: { x: dx / length, y: dy / length, z: dz / length },
    length,
    width: member.width / TICKS_PER_UNIT,
    roleId: member.roleId,
    role: member.role,
  };
}

/** Tessellate a set of members, dropping any degenerate ones. */
export function memberBoxes(members: readonly MemberSegment[]): MemberBox[] {
  const out: MemberBox[] = [];
  for (const m of members) {
    const box = memberBox(m);
    if (box) {
      out.push(box);
    }
  }
  return out;
}
