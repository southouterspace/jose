import { expect, test } from "bun:test";
import { BUFFER_LAYOUTS, LAYOUT_HASH } from "@jose/model-types";
import { MemberMirror, MEMBER_ROLES, assertLayout } from "./index";

const L = BUFFER_LAYOUTS.MemberPlacement;

/** Encode rows into a SoA block exactly as the Rust writer does: pure column-major, little-endian,
 *  at the *generated* byte offsets. If the reader and this writer agree, the keystone holds. */
function encode(rows: Array<Record<string, number>>): ArrayBuffer {
  const buffer = new ArrayBuffer(L.bufferBytes);
  const view = new DataView(buffer);
  rows.forEach((row, i) => {
    for (const col of L.columns) {
      const at = col.byteOffset + i * col.stride;
      const value = row[col.field] ?? 0;
      if (col.arrayKind === "Uint32Array") view.setUint32(at, value, true);
      else view.setInt32(at, value, true);
    }
  });
  return buffer;
}

test("decodes rows written at the generated column offsets", () => {
  const studId = MEMBER_ROLES.indexOf("stud");
  const plateId = MEMBER_ROLES.indexOf("plate");
  const buffer = encode([
    { x0: 0, y0: 0, z0: 48, x1: 0, y1: 0, z1: 2976, width: 48, roleId: studId },
    { x0: 0, y0: 0, z0: 0, x1: 3840, y1: 0, z1: 0, width: 48, roleId: plateId },
  ]);

  const mirror = new MemberMirror(buffer, 2);
  expect(mirror.count).toBe(2);

  const stud = mirror.row(0);
  expect(stud.role).toBe("stud");
  expect(stud.z1 - stud.z0).toBe(2928); // extends up the wall
  expect(stud.x0).toBe(stud.x1);

  const plate = mirror.row(1);
  expect(plate.role).toBe("plate");
  expect(plate.x1 - plate.x0).toBe(3840); // runs along the baseline
  expect(plate.z0).toBe(plate.z1);

  expect(mirror.rows()).toHaveLength(2);
});

test("count bounds the rows read — stale tail bytes are ignored", () => {
  const buffer = encode([
    { x0: 1, roleId: 0 },
    { x0: 2, roleId: 0 },
    { x0: 3, roleId: 0 },
  ]);
  // Worker reported only 1 live row even though 3 were encoded.
  const mirror = new MemberMirror(buffer, 1);
  expect(mirror.rows()).toHaveLength(1);
  expect(() => mirror.row(1)).toThrow();
});

test("views are zero-copy — mutating the backing buffer shows through", () => {
  const buffer = encode([{ x0: 10, roleId: 0 }]);
  const mirror = new MemberMirror(buffer, 1);
  expect(mirror.row(0).x0).toBe(10);
  new DataView(buffer).setInt32(L.columns[0]!.byteOffset, 999, true);
  expect(mirror.row(0).x0).toBe(999); // same bytes, no copy
});

test("rejects a buffer that is too small for the layout", () => {
  expect(() => new MemberMirror(new ArrayBuffer(16), 0)).toThrow();
});

test("assertLayout guards the keystone hash", () => {
  expect(() => assertLayout(LAYOUT_HASH)).not.toThrow();
  expect(() => assertLayout("layout-deadbeef")).toThrow(/mismatch/);
});
