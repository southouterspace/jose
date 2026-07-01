/**
 * Ambient contract for the wasm-pack output of the `bim-wasm` crate.
 *
 * The real files (`bim_wasm.js`, `bim_wasm_bg.wasm`, `bim_wasm.d.ts`) are produced under
 * `./pkg/` by `bun run build:wasm` (wasm-pack). This declaration mirrors the `#[wasm_bindgen]`
 * surface so the app typechecks without the generated artifact present — the same shape wasm-pack
 * emits, kept in sync with `crates/bim-wasm/src/lib.rs`.
 *
 * This mirror is a fallback only: once `./pkg/bim_wasm.d.ts` exists, the concrete file wins over
 * this wildcard module. CI's `wasm-types` job builds the real package and typechecks the app
 * against it, so any drift between this mirror and the real engine surface fails the build.
 */
declare module "*/bim_wasm.js" {
  /** The engine handle — see `crates/bim-wasm`'s `Engine`. */
  export class Engine {
    constructor();
    /** Channel A: draw/redraw the wall; returns the live member count. */
    drawWall(
      x0: number,
      y0: number,
      x1: number,
      y1: number,
      height: number,
      spacingInches: number
    ): number;
    /** Channel A: draw/redraw the current space's footprint from a closed ring of plan vertices
     *  (parallel tick columns). Returns `""` when accepted, or a stable rejection code
     *  (`RejectReason::code`) when the ring is degenerate and state is unchanged. */
    drawFootprint(xs: Int32Array, ys: Int32Array): string;
    /** Channel A: push/pull a volume's face (the 3D top-cap gesture) by a signed tick distance.
     *  Returns `""` when accepted, or a stable rejection code when the move is refused. */
    pushPull(volumeId: number, faceIndex: number, distance: number): string;
    /** Channel A: step back to the previous space state; `true` when the model changed. */
    undo(): boolean;
    /** Channel A: reinstate the most recently undone space state; `true` when it changed. */
    redo(): boolean;
    /** Whether there is a prior state to undo to. */
    canUndo(): boolean;
    /** Whether there is an undone state to redo. */
    canRedo(): boolean;
    /** Live member count. */
    memberCount(): number;
    /** Live footprint vertex count. */
    footprintCount(): number;
    /** Live volume (mass) count. */
    volumeCount(): number;
    /** Channel B: a copy of the canonical SoA buffer bytes. */
    snapshot(): Uint8Array;
    /** Channel B: a copy of the canonical footprint SoA bytes for the plan view. */
    footprintSnapshot(): Uint8Array;
    /** Channel B: a copy of the canonical volume SoA bytes for the 3D view. */
    volumeSnapshot(): Uint8Array;
    /** The generated `BufferLayout` digest, for the startup drift assertion. */
    layoutHash(): string;
    /** Bytes per logical element. */
    elementStride(): number;
    /** Release the wasm-side handle. */
    free(): void;
  }

  /** wasm-pack's default init: instantiates the module (fetching the `.wasm` when no input given). */
  export default function init(input?: unknown): Promise<unknown>;
}
