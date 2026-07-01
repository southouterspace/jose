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

/** Push/pull a volume's top cap to a new height. Mirrors the engine's `pushPull(volumeId,
 *  faceIndex, distance)` ABI: `distance` is a signed tick delta (positive raises the mass, negative
 *  lowers it); `faceIndex` is the kernel's canonical face (`TOP_FACE`) the gesture resolved to.
 *  The 3D view never invents a face — it names one the engine defined (ADR 0008 §3). */
export interface PushPullCommand {
  readonly distance: number;
  readonly faceIndex: number;
  readonly kind: "pushPull";
  readonly volumeId: number;
}

/** The immutable intents the runner can emit (one variant per modeled tool). */
export type Command = DrawFootprintCommand | DrawWallCommand | PushPullCommand;

/**
 * Map a vertical pointer drag (screen pixels) to a signed push/pull tick distance.
 *
 * Screen Y grows *downward*, so dragging **up** (the mass getting taller) is a *negative* pixel
 * delta — we negate it so dragging up yields a positive distance (raise the top cap). `scale` is
 * world ticks per pixel; a zero delta yields exactly zero (no recompute). Pure: no rounding policy
 * beyond `Math.round` so the engine receives whole ticks.
 *
 * @param deltaPixels screen-space `pointerY - startY` (down-positive)
 * @param scale       world ticks per screen pixel (must be > 0)
 */
export function pushPullDistance(deltaPixels: number, scale: number): number {
  // `+ 0` collapses a `-0` (from `Math.round(-0)`) to `0` — a zero drag is exactly no recompute.
  return Math.round(-deltaPixels * scale) + 0;
}

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
  /** Close-the-ring radius for the footprint tool, ticks: a pick (or hovering cursor) within this of
   *  the first vertex snaps onto it and closes the ring (with ≥3 vertices down). */
  readonly closeThresholdTicks: number;
  /** Snap grid spacing, ticks (e.g. 32 = 1in). */
  readonly gridTicks: number;
  /** Inference band, ticks: a draft point within this of an existing vertex's X (or Y) snaps to that
   *  vertex's column (or row), surfacing a dashed alignment guide ("perpendicular to existing points"). */
  readonly inferenceToleranceTicks: number;
  /** On-center module applied to drawn walls, real inches. */
  readonly spacingInches: number;
  /** Wall height applied to drawn walls, ticks. */
  readonly wallHeightTicks: number;
}

/** 8ft tall, 16in OC, 1in snap grid, 6in ring-close radius, ~4in alignment-inference band. */
export const DEFAULT_SETTINGS: ToolSettings = {
  wallHeightTicks: 8 * 384, // 8ft (1ft = 384 ticks)
  spacingInches: 16,
  gridTicks: 32, // 1in
  closeThresholdTicks: 192, // 6in — generous enough to land the closing click reliably
  inferenceToleranceTicks: 120, // ~3.75in
};

const snapValue = (v: number, grid: number): number =>
  grid > 0 ? Math.round(v / grid) * grid : v;

/** Per-pick modifiers the view supplies from the live input state (e.g. a held modifier key). */
export interface PickOptions {
  /** Constrain this pick onto the dominant axis (world-X or world-Y) relative to the previous pick,
   *  so the new segment runs orthogonally — the "hold Shift to draw straight" gesture. */
  readonly axisLock?: boolean;
  /** Take the point as an exact world location — round to whole ticks but skip grid snapping and
   *  alignment inference. The value-entry path uses this: a typed length already names the endpoint,
   *  so re-snapping it to the grid would distort the length the user asked for. */
  readonly exact?: boolean;
}

/** A dashed alignment guide surfaced while drawing: a draft point that shares an existing vertex's
 *  column (`vertical`) or row (`horizontal`) snaps onto it, and the view draws this guide line. */
export interface AlignmentGuide {
  /** The constant coordinate the guide line sits at (X for `vertical`, Y for `horizontal`), ticks. */
  readonly atTicks: number;
  /** A `vertical` guide is a constant-X line; a `horizontal` guide is a constant-Y line. */
  readonly orientation: "horizontal" | "vertical";
  /** Index into the in-progress picks of the vertex this guide aligns to. */
  readonly sourceIndex: number;
}

/** The resolved live preview for the active draw: where a click would land, the alignment guides in
 *  play, and whether that click would close the ring. Transient UI only — never canonical geometry. */
export interface DraftPoint {
  /** True when the point sits on the first vertex and a click would close the ring (≥3 vertices). */
  readonly closing: boolean;
  /** The dashed alignment guides currently snapping the point (empty when none apply). */
  readonly guides: readonly AlignmentGuide[];
  /** The resolved world point (ticks) a click would commit. */
  readonly point: Point;
}

/** Feet token: a number followed by a feet mark (`'`, `ft`, `feet`). */
const FEET_RE = /(\d+(?:\.\d+)?)\s*(?:'|ft|feet)/;
/** Inches token: a number followed by an inch mark (`"`, `in`, `inch`, `inches`). */
const INCHES_RE = /(\d+(?:\.\d+)?)\s*(?:"|in|inch|inches)/;

/** Parse a feet/inches value-entry string into whole ticks (1ft = 384, 1in = 32), or `null` if it
 *  names no positive length. Accepts `10`, `10.5`, `10'`, `10' 6"`, `10'6`, `6"`, `6in`, `10ft 6in`.
 *  A bare number is read as feet — the plan grid is 1ft, so feet is the natural default unit. */
export function parseLength(input: string): number | null {
  const s = input.trim().toLowerCase();
  if (!s) {
    return null;
  }
  const ft = s.match(FEET_RE);
  const inch = s.match(INCHES_RE);
  let feet = 0;
  let inches = 0;
  if (ft) {
    feet = Number.parseFloat(ft[1] ?? "0");
  }
  if (inch) {
    inches = Number.parseFloat(inch[1] ?? "0");
  }
  if (!(ft || inch)) {
    const bare = Number(s);
    if (!Number.isFinite(bare)) {
      return null;
    }
    feet = bare;
  } else if (ft && !inch) {
    // A trailing number after the feet mark with no inch mark is inches: `10' 6`.
    const rest = s.slice((ft.index ?? 0) + ft[0].length).trim();
    const trailing = Number(rest);
    if (rest && Number.isFinite(trailing)) {
      inches = trailing;
    }
  }
  const ticks = Math.round(feet * 384 + inches * 32);
  return Number.isFinite(ticks) && ticks > 0 ? ticks : null;
}

/** Format whole ticks as a feet-and-inches string for display (e.g. `12' 0"`, `3' 7.5"`). Ticks
 *  never surface to the user (CLAUDE.md / copy.md); this is the one place plan lengths become feet. */
export function formatLength(ticks: number): string {
  const totalInches = ticks / 32;
  const feet = Math.floor(totalInches / 12);
  const inches = totalInches - feet * 12;
  // Round to 1/100in so diagonal lengths read cleanly; whole inches show without a decimal tail.
  const rounded = Math.round(inches * 100) / 100;
  return `${feet}' ${rounded}"`;
}

/** The point `distanceTicks` away from `from`, in the direction of `toward` (rounded to whole ticks).
 *  With `axisLock`, the direction is first collapsed onto the dominant world axis. A zero-length
 *  direction defaults to +X so a typed length always produces a segment. */
export function pointAtDistance(
  from: Point,
  toward: Point,
  distanceTicks: number,
  axisLock = false
): Point {
  let dx = toward.x - from.x;
  let dy = toward.y - from.y;
  if (axisLock) {
    if (Math.abs(dx) >= Math.abs(dy)) {
      dy = 0;
    } else {
      dx = 0;
    }
  }
  const len = Math.hypot(dx, dy);
  if (len === 0) {
    return { x: Math.round(from.x + distanceTicks), y: from.y };
  }
  return {
    x: Math.round(from.x + (dx / len) * distanceTicks),
    y: Math.round(from.y + (dy / len) * distanceTicks),
  };
}

/** Snap a point onto alignment with existing vertices: if its X (or Y) is within `toleranceTicks` of
 *  a vertex's, adopt that column (or row) and report the guide. At most one vertical and one
 *  horizontal guide (each the nearest qualifying vertex) — the SketchUp-style inference engine. */
export function inferAlignment(
  point: Point,
  vertices: readonly Point[],
  toleranceTicks: number
): { guides: AlignmentGuide[]; point: Point } {
  let vAt: number | null = null;
  let vIndex = -1;
  let vBest = Number.POSITIVE_INFINITY;
  let hAt: number | null = null;
  let hIndex = -1;
  let hBest = Number.POSITIVE_INFINITY;
  vertices.forEach((v, i) => {
    const dx = Math.abs(point.x - v.x);
    if (dx <= toleranceTicks && dx < vBest) {
      vBest = dx;
      vAt = v.x;
      vIndex = i;
    }
    const dy = Math.abs(point.y - v.y);
    if (dy <= toleranceTicks && dy < hBest) {
      hBest = dy;
      hAt = v.y;
      hIndex = i;
    }
  });
  const guides: AlignmentGuide[] = [];
  let { x, y } = point;
  if (vAt !== null) {
    x = vAt;
    guides.push({ orientation: "vertical", atTicks: vAt, sourceIndex: vIndex });
  }
  if (hAt !== null) {
    y = hAt;
    guides.push({
      orientation: "horizontal",
      atTicks: hAt,
      sourceIndex: hIndex,
    });
  }
  return { point: { x, y }, guides };
}

/** Lock a point onto the dominant axis relative to an anchor: keep whichever of X / Y moved further
 *  and snap the other back to the anchor, so the segment runs straight along world-X or world-Y. */
function constrainToAxis(p: Point, anchor: Point): Point {
  return Math.abs(p.x - anchor.x) >= Math.abs(p.y - anchor.y)
    ? { x: p.x, y: anchor.y }
    : { x: anchor.x, y: p.y };
}

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
   * Resolve a raw world point into the draft a click would land on — applying axis lock, ring-close
   * snapping, and alignment inference — **without** committing. The view calls this on pointer-move
   * to render the live preview (rubber band, guides, close target); `pick` resolves identically, so
   * the committed vertex always matches the preview the user saw.
   */
  draft(raw: Point, options: PickOptions = {}): DraftPoint {
    let point = options.exact
      ? { x: Math.round(raw.x), y: Math.round(raw.y) }
      : this.snap(raw);
    if (options.axisLock) {
      const anchor = this.picks.at(-1);
      if (anchor) {
        point = constrainToAxis(point, anchor);
      }
    }

    if (this.active.commit === "ring") {
      const first = this.picks[0];
      if (first && this.picks.length >= 3 && this.nearFirst(point, first)) {
        // Snap onto the first vertex so the closing click lands exactly on it.
        return { point: first, guides: [], closing: true };
      }
    }

    // Axis lock and exact entry already fix the direction/endpoint — leave them untouched. Otherwise
    // let the point snap onto alignment with existing vertices (the dashed guides).
    if (options.axisLock || options.exact) {
      return { point, guides: [], closing: false };
    }
    const aligned = inferAlignment(
      point,
      this.picks,
      this.settings.inferenceToleranceTicks
    );
    return { point: aligned.point, guides: aligned.guides, closing: false };
  }

  /**
   * Register a world pick. Returns the emitted [`Command`] when this pick satisfies the active
   * tool (and resets for the next operation), otherwise `null`. The pick is resolved through
   * [`draft`] first, so it honors axis lock, alignment inference, and ring-close snapping.
   */
  pick(raw: Point, options: PickOptions = {}): Command | null {
    const resolved = this.draft(raw, options);

    if (this.active.commit === "ring") {
      // A near-first click with ≥3 vertices down closes the ring (and is itself the closing edge,
      // not a new vertex). Otherwise it extends the open polyline.
      if (resolved.closing) {
        const command = this.commitRing();
        this.picks = [];
        return command;
      }
      this.picks.push(resolved.point);
      return null;
    }

    this.picks.push(resolved.point);
    if (this.picks.length < this.active.picks) {
      return null;
    }
    const command = this.commit();
    this.picks = [];
    return command;
  }

  /** A point is "near" the first vertex when within the close threshold (Chebyshev radius). */
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
