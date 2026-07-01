import { describe, expect, test } from "bun:test";
import {
  deleteVertex,
  insertOnEdge,
  MIN_RING_VERTICES,
  moveVertex,
} from "./footprint-edit";

/** A unit square ring in ticks (CCW). */
const square = [
  { x: 0, y: 0 },
  { x: 100, y: 0 },
  { x: 100, y: 100 },
  { x: 0, y: 100 },
];

describe("moveVertex", () => {
  test("replaces the target vertex, leaving the rest and the length", () => {
    const moved = moveVertex(square, 1, { x: 150, y: 20 });
    expect(moved).toEqual([
      { x: 0, y: 0 },
      { x: 150, y: 20 },
      { x: 100, y: 100 },
      { x: 0, y: 100 },
    ]);
  });

  test("an out-of-range index returns the ring untouched (a fresh copy)", () => {
    const same = moveVertex(square, 9, { x: 1, y: 1 });
    expect(same).toEqual(square);
    expect(same).not.toBe(square);
  });
});

describe("insertOnEdge", () => {
  test("splices a vertex after the edge's start, growing the ring by one", () => {
    const grown = insertOnEdge(square, 1, { x: 100, y: 50 });
    expect(grown).toEqual([
      { x: 0, y: 0 },
      { x: 100, y: 0 },
      { x: 100, y: 50 },
      { x: 100, y: 100 },
      { x: 0, y: 100 },
    ]);
  });

  test("inserting on the closing edge appends before wrap (after the last vertex)", () => {
    const grown = insertOnEdge(square, 3, { x: 0, y: 50 });
    expect(grown).toHaveLength(5);
    expect(grown[4]).toEqual({ x: 0, y: 50 });
  });
});

describe("deleteVertex", () => {
  test("removes the target vertex from a ring above the minimum", () => {
    const shorter = deleteVertex(square, 2);
    expect(shorter).toEqual([
      { x: 0, y: 0 },
      { x: 100, y: 0 },
      { x: 0, y: 100 },
    ]);
  });

  test("refuses a delete that would drop below the minimum ring", () => {
    const triangle = square.slice(0, MIN_RING_VERTICES);
    expect(deleteVertex(triangle, 0)).toBeNull();
  });

  test("an out-of-range index is a no-op (null)", () => {
    expect(deleteVertex(square, 9)).toBeNull();
  });
});
