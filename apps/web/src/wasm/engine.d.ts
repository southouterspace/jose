/**
 * Ambient contract for the wasm-pack output of the `bim-wasm` crate.
 *
 * The real files (`bim_wasm.js`, `bim_wasm_bg.wasm`, `bim_wasm.d.ts`) are produced under
 * `./pkg/` by `bun run build:wasm` (wasm-pack). This declaration mirrors the `#[wasm_bindgen]`
 * surface so the app typechecks without the generated artifact present — the same shape wasm-pack
 * emits, kept in sync with `crates/bim-wasm/src/lib.rs`.
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
      spacingInches: number,
    ): number;
    /** Live member count. */
    memberCount(): number;
    /** Channel B: a copy of the canonical SoA buffer bytes. */
    snapshot(): Uint8Array;
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
