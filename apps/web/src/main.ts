/**
 * The main thread — the "hands & eyes". Wires the full **draw → recompute → render** slice:
 *
 *   pointer picks → ToolRunner (a DrawWall command) → worker (Rust recompute) → SoA snapshot →
 *   MemberMirror (zero-copy views) → canvas elevation.
 *
 * It never touches canonical geometry: it sends intents one way and reads back a read-only mirror.
 */

import { assertLayout, MemberMirror } from "@jose/render-mirror";
import { ToolRunner } from "@jose/tool-runner";
import type { EngineRequest, EngineResponse } from "./protocol";
import { renderMembers } from "./render";

/** Pixels per tick for mapping pointer picks into world ticks (1/32in). */
const INPUT_SCALE = 0.1;

const canvas = document.getElementById("scene") as HTMLCanvasElement;
const ctx = canvas.getContext("2d");
const status = document.getElementById("status");
if (!(ctx && status)) {
  throw new Error("web: #scene canvas or #status element missing");
}

const worker = new Worker(new URL("./engine-worker.ts", import.meta.url), {
  type: "module",
});
const runner = new ToolRunner();

worker.onmessage = (event: MessageEvent<EngineResponse>): void => {
  const message = event.data;
  if (message.kind === "ready") {
    // Guard the keystone: the engine's generated layout must match this build's.
    assertLayout(message.layoutHash);
    status.textContent = "Engine ready — click two points to draw a wall.";
  } else if (message.kind === "members") {
    renderMembers(ctx, new MemberMirror(message.buffer, message.count));
    status.textContent = `${message.count} members framed.`;
  }
};

worker.postMessage({ kind: "init" } satisfies EngineRequest);

canvas.addEventListener("pointerdown", (event: PointerEvent): void => {
  const rect = canvas.getBoundingClientRect();
  const command = runner.pick({
    x: Math.round((event.clientX - rect.left) / INPUT_SCALE),
    y: Math.round((event.clientY - rect.top) / INPUT_SCALE),
  });
  if (!command) {
    status.textContent = "Pick the wall's end point…";
    return;
  }
  const request: EngineRequest = {
    kind: "drawWall",
    x0: command.x0,
    y0: command.y0,
    x1: command.x1,
    y1: command.y1,
    height: command.height,
    spacingInches: command.spacingInches,
  };
  worker.postMessage(request);
});
