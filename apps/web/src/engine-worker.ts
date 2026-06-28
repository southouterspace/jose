/**
 * The engine worker — the Rust "brain" off the render thread.
 *
 * It loads the `bim-wasm` module, owns the canonical [`Engine`], and speaks the two channels:
 * intents in ([`EngineRequest`]), the SoA buffer snapshot out ([`EngineResponse`]). It holds no DOM
 * and does no rendering — it only recomputes and ships bytes.
 */

import type { EngineRequest, EngineResponse } from "./protocol";
import init, { Engine } from "./wasm/pkg/bim_wasm.js";

let engine: Engine | null = null;

self.onmessage = async (event: MessageEvent<EngineRequest>): Promise<void> => {
  const request = event.data;

  if (request.kind === "init") {
    await init();
    engine = new Engine();
    postMessage({
      kind: "ready",
      layoutHash: engine.layoutHash(),
    } satisfies EngineResponse);
    return;
  }

  if (!engine) {
    throw new Error("engine-worker: received a command before init");
  }

  if (request.kind === "drawWall") {
    const count = engine.drawWall(
      request.x0,
      request.y0,
      request.x1,
      request.y1,
      request.height,
      request.spacingInches
    );
    // Transfer the snapshot's backing buffer — zero-copy handoff of Channel B to the main thread.
    const bytes = engine.snapshot();
    const buffer = bytes.buffer as ArrayBuffer;
    postMessage({ kind: "members", count, buffer } satisfies EngineResponse, [
      buffer,
    ]);
  }
};
