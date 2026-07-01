/**
 * The main-thread ↔ worker message protocol — the wire shape of the two channels, shared by both
 * sides so they typecheck against the same contract. Kept dependency-free (no DOM, no WebWorker
 * globals) so it compiles under both the main and worker tsconfigs.
 */

/** Channel A intents (main thread → worker). */
export type EngineRequest =
  | { readonly kind: "init" }
  | {
      readonly kind: "drawWall";
      readonly x0: number;
      readonly y0: number;
      readonly x1: number;
      readonly y1: number;
      readonly height: number;
      readonly spacingInches: number;
    }
  | {
      /** Draw/redraw the current space's footprint from a closed ring of plan vertices (ticks).
       *  `xs`/`ys` are parallel columns; the closing edge is implicit. */
      readonly kind: "drawFootprint";
      readonly xs: readonly number[];
      readonly ys: readonly number[];
    }
  | {
      /** Edit the current space's footprint from the mutated ring of plan vertices (ticks). Mirrors
       *  wasm `editFootprint(xs, ys)`: parallel columns, the closing edge implicit. Unlike
       *  `drawFootprint`, the engine re-extrudes at the current mass height rather than flattening it
       *  (ADR 0015); a degenerate edit is rejected and canonical state is unchanged. */
      readonly kind: "editFootprint";
      readonly xs: readonly number[];
      readonly ys: readonly number[];
    }
  | {
      /** Push/pull a volume's top cap. Mirrors wasm `pushPull(volumeId, faceIndex, distance)`:
       *  `faceIndex` must be the kernel's `TOP_FACE`; `distance` is a signed tick delta. The engine
       *  validates the face and rejects a non-positive resulting height. */
      readonly kind: "pushPull";
      readonly volumeId: number;
      readonly faceIndex: number;
      readonly distance: number;
    }
  /** Step back to / forward through the engine's space-state history (undo/redo). No payload; the
   *  engine reships a `space` snapshot when the model actually changes. */
  | { readonly kind: "undo" }
  | { readonly kind: "redo" };

/** Channel B + acks (worker → main thread). */
export type EngineResponse =
  | { readonly kind: "ready"; readonly layoutHash: string }
  | {
      readonly kind: "members";
      readonly count: number;
      readonly buffer: ArrayBuffer;
    }
  | {
      /** One recompute, both space buffers: footprint vertices + the extruded volume (ADR 0008 §5).
       *  Each `buffer` is the canonical SoA snapshot bytes; the `*Count`s bound the live rows.
       *  `canUndo`/`canRedo` carry the engine's live history availability so the toolbar stays in
       *  sync after every mutation (draw, push/pull, undo, redo). */
      readonly kind: "space";
      readonly footprintCount: number;
      readonly footprintBuffer: ArrayBuffer;
      readonly volumeCount: number;
      readonly volumeBuffer: ArrayBuffer;
      readonly canUndo: boolean;
      readonly canRedo: boolean;
    }
  | {
      /** A command the engine refused (a degenerate footprint, an out-of-model push/pull). Canonical
       *  state is unchanged; `reason` is the stable code (`RejectReason::code`) the UI maps to copy. */
      readonly kind: "rejected";
      readonly command: "drawFootprint" | "editFootprint" | "pushPull";
      readonly reason: string;
    };
