import { describe, expect, test } from "bun:test";
import {
  catalogKeys,
  runnerBackedKeys,
  TOOL_CHROME,
  toolChrome,
} from "./tool-chrome";

describe("tool-chrome registry", () => {
  test("every runner-backed tool has a matching TOOL_CATALOG entry (ADR 0012 §6)", () => {
    // A runner-backed chrome key that isn't in the catalog would throw "unknown tool" on activation.
    const catalog = new Set(catalogKeys());
    for (const key of runnerBackedKeys()) {
      expect(catalog.has(key)).toBe(true);
    }
  });

  test("tool keys are unique", () => {
    const keys = TOOL_CHROME.map((tool) => tool.key);
    expect(new Set(keys).size).toBe(keys.length);
  });

  test("shortcuts are unique among the tools that declare one", () => {
    const shortcuts = TOOL_CHROME.map((tool) => tool.shortcut).filter(
      (s): s is string => s !== null
    );
    expect(new Set(shortcuts).size).toBe(shortcuts.length);
  });

  test("toolChrome resolves a registered key and misses an unknown one", () => {
    expect(toolChrome("footprint")?.label).toBe("Footprint");
    expect(toolChrome("nope")).toBeUndefined();
  });

  describe("enablement", () => {
    const base = {
      hasMass: false,
      footprintVertices: 0,
      pendingPicks: 0,
      heightFeet: null,
      selectedKind: null,
    };

    test("push/pull is disabled without a mass, enabled with one", () => {
      const pushpull = toolChrome("pushpull");
      expect(pushpull?.enabled(base)).toBe(false);
      expect(pushpull?.enabled({ ...base, hasMass: true })).toBe(true);
    });

    test("footprint is always available", () => {
      expect(toolChrome("footprint")?.enabled(base)).toBe(true);
    });

    test("select is always available", () => {
      expect(toolChrome("select")?.enabled(base)).toBe(true);
    });
  });

  describe("select status copy", () => {
    const select = toolChrome("select");
    const base = {
      hasMass: false,
      footprintVertices: 0,
      pendingPicks: 0,
      heightFeet: null,
      selectedKind: null,
    };

    test("nothing selected: prompts to select and edit", () => {
      expect(select?.status(base)).toContain("drag a vertex to move it");
    });

    test("a selected vertex names the move + delete verbs", () => {
      expect(select?.status({ ...base, selectedKind: "vertex" })).toBe(
        "Selected a vertex — drag to move, Delete to remove, Esc to clear"
      );
    });

    test("a selected edge names the insert verb", () => {
      expect(select?.status({ ...base, selectedKind: "edge" })).toBe(
        "Selected an edge — drag it to add a vertex, Esc to clear"
      );
    });
  });

  describe("footprint status copy", () => {
    const footprint = toolChrome("footprint");
    const base = {
      hasMass: false,
      footprintVertices: 0,
      pendingPicks: 0,
      heightFeet: null,
      selectedKind: null,
    };

    test("idle: prompts to place vertices", () => {
      expect(footprint?.status(base)).toContain("click to place vertices");
    });

    test("mid-draw with 3+ picks: prompts to close", () => {
      expect(footprint?.status({ ...base, pendingPicks: 3 })).toContain(
        "click the first vertex to close"
      );
    });

    test("committed with a mass: reports vertices and height", () => {
      expect(
        footprint?.status({
          ...base,
          footprintVertices: 4,
          heightFeet: 8,
        })
      ).toBe("Footprint: 4 vertices · mass 8.0ft tall");
    });
  });
});
