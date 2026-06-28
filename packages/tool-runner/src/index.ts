/**
 * @jose/tool-runner — the "hands": the data-driven drawing tool runner.
 *
 * SketchUp's whole tool set is one runner plus a flyweight catalog of [`ToolDefinition`]s, never N
 * tool classes. The runner collects snapped world picks for the active tool and, when the tool is
 * satisfied, emits an immutable [`Command`] — the intent that crosses Channel A to the Rust engine.
 *
 * Committed world points are integer ticks (1/32in); screen-space/pixels are the app's concern and
 * never appear here. The runner only ever *emits* intents — it never mutates canonical geometry.
 */

/** A committed world point in plan, integer ticks (1/32in). */
export interface Point {
  readonly x: number;
  readonly y: number;
}

/** Draw (or redraw) a wall from two plan-baseline endpoints. Mirrors the engine's `drawWall` ABI:
 *  linear inputs are ticks, the on-center module is real inches. */
export interface DrawWallCommand {
  readonly height: number;
  readonly kind: "drawWall";
  readonly spacingInches: number;
  readonly x0: number;
  readonly x1: number;
  readonly y0: number;
  readonly y1: number;
}

/** Draw (or redraw) a space's footprint from a closed ring of plan vertices. Mirrors the engine's
 *  `drawFootprint(xs, ys)` ABI: parallel tick columns, one entry per vertex, the closing edge
 *  implicit. The mid-draw polyline is client-only UI; only the closed ring crosses the boundary. */
export interface DrawFootprintCommand {
  readonly kind: "drawFootprint";
  readonly xs: readonly number[];
  readonly ys: readonly number[];
}

/** The immutable intents the runner can emit (one variant per modeled tool). */
export type Command = DrawFootprintCommand | DrawWallCommand;

/** How a tool decides it is satisfied and should commit.
 *  - `count`: commit once `picks` snapped points are collected (e.g. the two-pick wall).
 *  - `ring`: an open-ended polyline that closes — and commits — when a pick lands within the close
 *    threshold of the first vertex, once there are ≥3 vertices (the footprint). */
export type CommitRule = "count" | "ring";

/** A flyweight tool description — the catalog row, not a class. */
export interface ToolDefinition {
  /** How the tool commits. */
  readonly commit: CommitRule;
  /** Open catalog key, e.g. `wall`. */
  readonly key: string;
  /** Human label for the toolbar. */
  readonly label: string;
  /** For `count` tools: world picks required before commit. Unused by `ring` tools. */
  readonly picks: number;
}

/** The modeled tool catalog. A new tool adds a row here (label + commit rule); its commit logic —
 *  how its completed picks become a [`Command`] — is added to `ToolRunner.commit`. */
export const TOOL_CATALOG: Record<string, ToolDefinition> = {
  wall: { key: "wall", label: "Wall", commit: "count", picks: 2 },
  footprint: { key: "footprint", label: "Footprint", commit: "ring", picks: 0 },
};

/** Runner settings — the modal value grammar (height, OC module) and the snap grid. */
export interface ToolSettings {
  /** Close-the-ring radius for the footprint tool, ticks: a pick within this of the first vertex
   *  closes the ring (with ≥3 vertices down). */
  readonly closeThresholdTicks: number;
  /** Snap grid spacing, ticks (e.g. 32 = 1in). */
  readonly gridTicks: number;
  /** On-center module applied to drawn walls, real inches. */
  readonly spacingInches: number;
  /** Wall height applied to drawn walls, ticks. */
  readonly wallHeightTicks: number;
}

/** 8ft tall, 16in OC, 1in snap grid, ~1in ring-close radius. */
export const DEFAULT_SETTINGS: ToolSettings = {
  wallHeightTicks: 8 * 384, // 8ft (1ft = 384 ticks)
  spacingInches: 16,
  gridTicks: 32, // 1in
  closeThresholdTicks: 32, // 1in
};

const snapValue = (v: number, grid: number): number =>
  grid > 0 ? Math.round(v / grid) * grid : v;

/**
 * The single drawing state machine. Holds the active tool, the accumulated snapped picks, and the
 * value-grammar settings. `pick` advances the tool and returns a [`Command`] the instant the tool
 * is satisfied, otherwise `null`.
 */
export class ToolRunner {
  private settings: ToolSettings;
  private active: ToolDefinition;
  private picks: Point[] = [];

  constructor(settings: ToolSettings = DEFAULT_SETTINGS, toolKey = "wall") {
    this.settings = settings;
    this.active = requireTool(toolKey);
  }

  /** The active tool's catalog key. */
  get activeKey(): string {
    return this.active.key;
  }

  /** Picks committed so far this operation (snapped). */
  get pendingPicks(): readonly Point[] {
    return this.picks;
  }

  /** Switch the active tool, cancelling any in-progress operation. */
  activate(toolKey: string): void {
    this.active = requireTool(toolKey);
    this.picks = [];
  }

  /** Replace the value-grammar settings (height / OC module / grid). */
  configure(patch: Partial<ToolSettings>): void {
    this.settings = { ...this.settings, ...patch };
  }

  /** Snap a raw world point onto the tick grid. */
  snap(raw: Point): Point {
    return {
      x: snapValue(raw.x, this.settings.gridTicks),
      y: snapValue(raw.y, this.settings.gridTicks),
    };
  }

  /** Abort the in-progress operation, keeping the active tool. */
  cancel(): void {
    this.picks = [];
  }

  /**
   * Register a world pick. Returns the emitted [`Command`] when this pick satisfies the active
   * tool (and resets for the next operation), otherwise `null`.
   */
  pick(raw: Point): Command | null {
    const snapped = this.snap(raw);

    if (this.active.commit === "ring") {
      // A near-first click with ≥3 vertices down closes the ring (and is itself the closing edge,
      // not a new vertex). Otherwise it extends the open polyline.
      const first = this.picks[0];
      if (first && this.picks.length >= 3 && this.nearFirst(snapped, first)) {
        const command = this.commitRing();
        this.picks = [];
        return command;
      }
      this.picks.push(snapped);
      return null;
    }

    this.picks.push(snapped);
    if (this.picks.length < this.active.picks) {
      return null;
    }
    const command = this.commit();
    this.picks = [];
    return command;
  }

  /** A snapped pick is "near" the first vertex when within the close threshold (Chebyshev radius). */
  private nearFirst(p: Point, first: Point): boolean {
    const r = this.settings.closeThresholdTicks;
    return Math.abs(p.x - first.x) <= r && Math.abs(p.y - first.y) <= r;
  }

  private commitRing(): DrawFootprintCommand {
    return {
      kind: "drawFootprint",
      xs: this.picks.map((p) => p.x),
      ys: this.picks.map((p) => p.y),
    };
  }

  private commit(): Command {
    switch (this.active.key) {
      case "wall": {
        const a = this.picks[0];
        const b = this.picks[1];
        if (!(a && b)) {
          throw new Error("tool-runner: 'wall' requires two picks");
        }
        return {
          kind: "drawWall",
          x0: a.x,
          y0: a.y,
          x1: b.x,
          y1: b.y,
          height: this.settings.wallHeightTicks,
          spacingInches: this.settings.spacingInches,
        };
      }
      default:
        throw new Error(
          `tool-runner: tool "${this.active.key}" has no commit rule`
        );
    }
  }
}

function requireTool(key: string): ToolDefinition {
  const tool = TOOL_CATALOG[key];
  if (!tool) {
    throw new Error(`tool-runner: unknown tool "${key}"`);
  }
  return tool;
}
