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
    };

/** Channel B + acks (worker → main thread). */
export type EngineResponse =
  | { readonly kind: "ready"; readonly layoutHash: string }
  | {
      readonly kind: "members";
      readonly count: number;
      readonly buffer: ArrayBuffer;
    };
