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

/** A viewport a tool operates in. */
export type Surface = "3d" | "plan";

/** What a typed value in the measurement box means for a tool (the VCB semantics of ADR 0012 §4).
 *  Declared now; the value box generalizes onto it in a later slice (today it is plan-only "length"). */
export type ValueGrammar = "height" | "length" | "none";

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

/** The user-facing tools, in toolbar order. */
export const TOOL_CHROME: readonly ToolChrome[] = [
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
    key: "pushpull",
    label: "Push/Pull",
    shortcut: "p",
    surfaces: ["3d"],
    value: "height",
    runnerBacked: false,
    enabled: (state) => state.hasMass,
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
