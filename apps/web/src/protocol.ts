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
      /** Push/pull a volume's top cap. Mirrors wasm `pushPull(volumeId, faceIndex, distance)`:
       *  `faceIndex` must be the kernel's `TOP_FACE`; `distance` is a signed tick delta. The engine
       *  validates the face and rejects a non-positive resulting height. */
      readonly kind: "pushPull";
      readonly volumeId: number;
      readonly faceIndex: number;
      readonly distance: number;
    };

/** Channel B + acks (worker → main thread). */
export type EngineResponse =
  | { readonly kind: "ready"; readonly layoutHash: string }
  | {
      readonly kind: "members";
      readonly count: number;
      readonly buffer: ArrayBuffer;
    }
  | {
      /** One recompute, all three space buffers: footprint vertices + the extruded volume (ADR 0008
       *  §5) + the framed perimeter members (ADR 0012). Each `buffer` is the canonical SoA snapshot
       *  bytes; the `*Count`s bound the live rows. */
      readonly kind: "space";
      readonly footprintCount: number;
      readonly footprintBuffer: ArrayBuffer;
      readonly volumeCount: number;
      readonly volumeBuffer: ArrayBuffer;
      readonly memberCount: number;
      readonly memberBuffer: ArrayBuffer;
    };
