/**
 * @jose/render-mirror — the JS-side read-only mirror over the Rust worker's SoA buffers.
 *
 * The Rust worker owns the canonical Structure-of-Arrays tick columns in linear memory and *writes*
 * them; this package cuts zero-copy typed-array views over the very same bytes and *reads* them to
 * render. Both sides interpret the bytes through the **identical generated `BufferLayout` table**
 * (`@jose/model-types`), so the writer and reader provably cannot drift — the keystone contract.
 *
 * One-way, model → pixels: nothing here mutates canonical geometry.
 */
import {
  BUFFER_LAYOUTS,
  LAYOUT_HASH,
  type BufferColumn,
  type BufferLayout,
} from "@jose/model-types";

export { LAYOUT_HASH };
export type { BufferColumn, BufferLayout };

/** The generated layout for the `MemberPlacement` SoA buffer (the first end-to-end slice). */
export const MEMBER_PLACEMENT_LAYOUT: BufferLayout = BUFFER_LAYOUTS.MemberPlacement;

/** Framing-role vocabulary, indexed by the `roleId` column. */
export const MEMBER_ROLES: readonly string[] = MEMBER_PLACEMENT_LAYOUT.roles;

/**
 * Guard the keystone at startup: the Rust engine reports its generated `LAYOUT_HASH`; if it differs
 * from the one this build was generated against, the byte offsets disagree and every read would be
 * silently wrong. Fail loudly instead.
 */
export function assertLayout(engineLayoutHash: string): void {
  if (engineLayoutHash !== LAYOUT_HASH) {
    throw new Error(
      `BufferLayout mismatch: engine reports "${engineLayoutHash}" but render-mirror was generated ` +
        `against "${LAYOUT_HASH}". Rebuild the wasm engine and run \`bun run codegen\`.`,
    );
  }
}

/** One decoded member: a wall-local elevation segment (ticks), a draw width, and its role. */
export interface MemberRow {
  readonly x0: number;
  readonly y0: number;
  readonly z0: number;
  readonly x1: number;
  readonly y1: number;
  readonly z1: number;
  readonly width: number;
  readonly roleId: number;
  readonly role: string;
}

type Int32Field = "x0" | "y0" | "z0" | "x1" | "y1" | "z1" | "width";

function column(layout: BufferLayout, field: string): BufferColumn {
  const col = layout.columns.find((c) => c.field === field);
  if (!col) throw new Error(`render-mirror: no column "${field}" in ${layout.domainType} layout`);
  return col;
}

/**
 * A read-only mirror over a `MemberPlacement` SoA byte block. Holds zero-copy typed-array column
 * views cut at the *generated* byte offsets; rows are bounded by the live `count` the worker
 * reported, so stale tail bytes are never read.
 */
export class MemberMirror {
  readonly layout: BufferLayout = MEMBER_PLACEMENT_LAYOUT;
  readonly count: number;
  private readonly i32: Record<Int32Field, Int32Array>;
  private readonly roleIds: Uint32Array;

  /**
   * @param buffer     the SoA bytes from the engine snapshot (or shared linear memory)
   * @param count      live member count the worker reported
   * @param byteOffset where this buffer starts inside `buffer` (0 for a standalone snapshot)
   */
  constructor(buffer: ArrayBuffer, count: number, byteOffset = 0) {
    const need = byteOffset + this.layout.bufferBytes;
    if (buffer.byteLength < need) {
      throw new Error(
        `render-mirror: buffer too small (${buffer.byteLength} bytes, need ${need} for ${this.layout.domainType})`,
      );
    }
    if (count < 0 || count > this.layout.capacity) {
      throw new Error(`render-mirror: count ${count} out of range [0, ${this.layout.capacity}]`);
    }
    this.count = count;
    const cap = this.layout.capacity;
    const i32 = (field: Int32Field): Int32Array =>
      new Int32Array(buffer, byteOffset + column(this.layout, field).byteOffset, cap);
    this.i32 = {
      x0: i32("x0"),
      y0: i32("y0"),
      z0: i32("z0"),
      x1: i32("x1"),
      y1: i32("y1"),
      z1: i32("z1"),
      width: i32("width"),
    };
    this.roleIds = new Uint32Array(buffer, byteOffset + column(this.layout, "roleId").byteOffset, cap);
  }

  /** Resolve a roleId to its role string (empty string if out of vocabulary). */
  roleOf(roleId: number): string {
    return this.layout.roles[roleId] ?? "";
  }

  /** Decode row `i` (0 ≤ i < count). */
  row(i: number): MemberRow {
    if (i < 0 || i >= this.count) throw new RangeError(`render-mirror: row ${i} out of [0, ${this.count})`);
    const roleId = this.roleIds[i] ?? 0;
    return {
      x0: this.i32.x0[i] ?? 0,
      y0: this.i32.y0[i] ?? 0,
      z0: this.i32.z0[i] ?? 0,
      x1: this.i32.x1[i] ?? 0,
      y1: this.i32.y1[i] ?? 0,
      z1: this.i32.z1[i] ?? 0,
      width: this.i32.width[i] ?? 0,
      roleId,
      role: this.roleOf(roleId),
    };
  }

  /** All live rows, in buffer order. */
  rows(): MemberRow[] {
    const out: MemberRow[] = [];
    for (let i = 0; i < this.count; i++) out.push(this.row(i));
    return out;
  }
}
