import { describe, expect, test } from "bun:test";
import { rejectionMessage } from "./rejection";

describe("rejectionMessage", () => {
  test("maps every known RejectReason code to a distinct sentence", () => {
    const codes = [
      "too_few_vertices",
      "zero_area",
      "self_intersecting",
      "not_top_face",
      "non_positive_height",
      "no_target",
    ];
    const messages = codes.map(rejectionMessage);
    // Each code gets its own copy (no accidental fall-through to the generic line).
    expect(new Set(messages).size).toBe(codes.length);
    for (const m of messages) {
      expect(m.length).toBeGreaterThan(0);
      expect(m).not.toBe("That action can't be applied here.");
    }
  });

  test("falls back to a generic line for an unknown code", () => {
    expect(rejectionMessage("something_new")).toBe(
      "That action can't be applied here."
    );
  });
});
