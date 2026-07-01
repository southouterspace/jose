/**
 * The tool-chrome registry — the front-end twin of `tool-runner`'s `TOOL_CATALOG` (ADR 0012).
 *
 * `TOOL_CATALOG` is the pure *commit grammar* (picks → `Command`, ticks only; no pixels, by
 * boundary rule). This registry is the *presentation* contract: the single place the toolbar, status
 * bar, keybindings — and, as they migrate, the value box and HUD — read a tool's chrome from, keyed
 * by the same tool key so a click/shortcut resolves to the right behavior. Adding a tool becomes "a
 * catalog row + a chrome row" instead of edits scattered across the view files.
 *
 * This is the skeleton: the toolbar, status bar, and single-key shortcuts consume it now; `surfaces`
 * and `value` are declared (the contract the value-box/HUD wiring lands on next) and called out where
 * they aren't wired yet.
 */

import { TOOL_CATALOG } from "@jose/tool-runner";
import type { SelectionKind } from "./plan-selection";

/** A viewport a tool operates in. */
export type Surface = "3d" | "plan";

/** What a typed value in the measurement box means for a tool (the VCB semantics of ADR 0012 §4).
 *  `length` = a plan edge (with optional `< angle`), `size` = a rectangle's `W,D`, `height` = a mass
 *  height. */
export type ValueGrammar = "height" | "length" | "none" | "size";

/** A minimal projection of the store the chrome reads to resolve enablement and status copy — not
 *  the store itself, so descriptors stay pure and testable. */
export interface ChromeState {
  /** Vertices in the committed footprint (0 before the first draw). */
  readonly footprintVertices: number;
  /** A mass (extruded volume) exists to act on. */
  readonly hasMass: boolean;
  /** Current mass height in feet, or `null` when there is no mass. */
  readonly heightFeet: number | null;
  /** Mid-draw picks for the active tool (transient). */
  readonly pendingPicks: number;
  /** What is currently selected, or `null` — drives the Select tool's status copy (ADR 0013). */
  readonly selectedKind: SelectionKind | null;
}

/** A user-facing tool's presentation contract. Keyed by the same key the runner / gesture layer
 *  uses; `runnerBacked` says which of the two owns its commit behavior. */
export interface ToolChrome {
  /** Whether the tool is currently usable (e.g. push/pull needs a mass). */
  enabled(state: ChromeState): boolean;
  /** Catalog/gesture key — matches `TOOL_CATALOG` when `runnerBacked`, else a view gesture. */
  readonly key: string;
  /** Toolbar label. */
  readonly label: string;
  /** `true` when a `ToolRunner` catalog entry backs the tool (it collects plan picks); `false` for a
   *  gesture tool handled directly by a view (e.g. push/pull). Gates the catalog-parity check. */
  readonly runnerBacked: boolean;
  /** Single-key shortcut to activate the tool (SketchUp-style), or `null` for none. */
  readonly shortcut: string | null;
  /** The contextual status-bar line for this tool given the live state. */
  status(state: ChromeState): string;
  /** Viewport(s) the tool operates in. */
  readonly surfaces: readonly Surface[];
  /** What the measurement box's typed value means for this tool. */
  readonly value: ValueGrammar;
}

/** The footprint tool's status line — the draw phases (placing → close → committed), copy preserved
 *  verbatim from the previous hand-wired ladder (owned by `product-design/.../copy.md`). */
function footprintStatus(state: ChromeState): string {
  if (state.pendingPicks > 0) {
    const closeHint =
      state.pendingPicks >= 3
        ? "click the first vertex to close"
        : "keep placing vertices";
    return `Drawing footprint — ${state.pendingPicks} point(s); ${closeHint}, type a length to set the next edge, or Esc to cancel`;
  }
  if (state.footprintVertices >= 3) {
    return state.heightFeet === null
      ? `Footprint: ${state.footprintVertices} vertices`
      : `Footprint: ${state.footprintVertices} vertices · mass ${state.heightFeet.toFixed(1)}ft tall`;
  }
  return "Ready — Footprint tool active; click to place vertices (hold Shift to lock to an axis)";
}

/** The rectangle tool's status line — corner-then-corner, with the typed-size hint. */
function rectangleStatus(state: ChromeState): string {
  if (state.pendingPicks > 0) {
    return "Rectangle — click the opposite corner, or type a size like 24', 16', or Esc to cancel";
  }
  return "Ready — Rectangle tool active; click the first corner";
}

/** The Select tool's status line once a piece is picked — each names the edit verb it now enables
 *  (P2 #9): a vertex moves or deletes, an edge splits into a new vertex, the footprint has no verb yet
 *  (whole-footprint move is P2 #10). */
const SELECTED_STATUS: Record<SelectionKind, string> = {
  vertex: "Selected a vertex — drag to move, Delete to remove, Esc to clear",
  edge: "Selected an edge — drag it to add a vertex, Esc to clear",
  footprint: "Selected the footprint — Esc to clear",
};

/** The Select tool's status line: what's picked and what you can do to it, or how to pick + edit. */
function selectStatus(state: ChromeState): string {
  if (state.selectedKind) {
    return SELECTED_STATUS[state.selectedKind];
  }
  return "Select — click to select; drag a vertex to move it, or an edge to add one";
}

/** The user-facing tools, in toolbar order. */
export const TOOL_CHROME: readonly ToolChrome[] = [
  {
    key: "select",
    label: "Select",
    shortcut: "s",
    surfaces: ["plan"],
    value: "none",
    runnerBacked: false,
    enabled: () => true,
    status: selectStatus,
  },
  {
    key: "footprint",
    label: "Footprint",
    shortcut: "f",
    surfaces: ["plan"],
    value: "length",
    runnerBacked: true,
    enabled: () => true,
    status: footprintStatus,
  },
  {
    key: "rectangle",
    label: "Rectangle",
    shortcut: "r",
    surfaces: ["plan"],
    value: "size",
    runnerBacked: true,
    enabled: () => true,
    status: rectangleStatus,
  },
  {
    key: "pushpull",
    label: "Push/Pull",
    shortcut: "p",
    surfaces: ["3d"],
    value: "height",
    runnerBacked: false,
    // Enabled once a **closed footprint** exists — its top cap is what push/pull acts on, whether the
    // face is still flat (the first extrude lifts it into a mass) or already a mass (later pushes grow
    // or shrink it). `footprintVertices` counts the canonical, closed committed ring (the mid-draw
    // polyline lives in `pendingPicks`), so this can't fire on a half-drawn outline. Gating on
    // `hasMass` was the bug: you need push/pull to *make* the first mass, so it can't require one.
    enabled: (state) => state.footprintVertices >= 3,
    status: () =>
      "Push/Pull active — drag the top cap in 3D to set the mass height",
  },
];

/** Look up a tool's chrome by key, or `undefined` if none is registered. */
export function toolChrome(key: string): ToolChrome | undefined {
  return TOOL_CHROME.find((tool) => tool.key === key);
}

/** The runner-backed chrome keys — every one must exist in `TOOL_CATALOG`, or activating it throws
 *  ("unknown tool"). Exposed so a test can assert the parity (ADR 0012 §6). */
export function runnerBackedKeys(): readonly string[] {
  return TOOL_CHROME.filter((tool) => tool.runnerBacked).map(
    (tool) => tool.key
  );
}

/** The catalog keys the runner knows — the parity target for {@link runnerBackedKeys}. */
export function catalogKeys(): readonly string[] {
  return Object.keys(TOOL_CATALOG);
}
