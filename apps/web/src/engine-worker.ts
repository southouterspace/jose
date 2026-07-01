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

/** Ship both canonical space buffers back as one `space` response — the single recompute that feeds
 *  both panes (ADR 0008 §5). Each snapshot's backing buffer is transferred (zero-copy handoff). */
function postSpace(engineRef: Engine): void {
  const footprintBuffer = engineRef.footprintSnapshot().buffer as ArrayBuffer;
  const volumeBuffer = engineRef.volumeSnapshot().buffer as ArrayBuffer;
  postMessage(
    {
      kind: "space",
      footprintCount: engineRef.footprintCount(),
      footprintBuffer,
      volumeCount: engineRef.volumeCount(),
      volumeBuffer,
      canUndo: engineRef.canUndo(),
      canRedo: engineRef.canRedo(),
    } satisfies EngineResponse,
    [footprintBuffer, volumeBuffer]
  );
}

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
    return;
  }

  if (request.kind === "drawFootprint") {
    const reason = engine.drawFootprint(
      Int32Array.from(request.xs),
      Int32Array.from(request.ys)
    );
    if (reason) {
      postMessage({
        kind: "rejected",
        command: "drawFootprint",
        reason,
      } satisfies EngineResponse);
      return;
    }
    postSpace(engine);
    return;
  }

  if (request.kind === "editFootprint") {
    // Same ABI as drawFootprint, but the engine re-extrudes at the current height (ADR 0015). An
    // empty reason means it took; a non-empty reason is a rejection (canonical state unchanged).
    const reason = engine.editFootprint(
      Int32Array.from(request.xs),
      Int32Array.from(request.ys)
    );
    if (reason) {
      postMessage({
        kind: "rejected",
        command: "editFootprint",
        reason,
      } satisfies EngineResponse);
      return;
    }
    postSpace(engine);
    return;
  }

  if (request.kind === "pushPull") {
    // The engine validates the face + resulting height; an empty reason means it took, and the
    // snapshot reflects the new canonical volume for both panes. A non-empty reason is a rejection.
    const reason = engine.pushPull(
      request.volumeId,
      request.faceIndex,
      request.distance
    );
    if (reason) {
      postMessage({
        kind: "rejected",
        command: "pushPull",
        reason,
      } satisfies EngineResponse);
      return;
    }
    postSpace(engine);
    return;
  }

  if (request.kind === "undo") {
    // Reship the snapshot only when the history actually moved; a no-op undo changes nothing.
    if (engine.undo()) {
      postSpace(engine);
    }
    return;
  }

  if (request.kind === "redo" && engine.redo()) {
    postSpace(engine);
  }
};
