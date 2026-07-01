/**
 * The app's small shared store: it owns the engine worker, the active tool, and the latest
 * canonical snapshot read back from the worker. The store holds **no** geometry of its own — the
 * footprint it exposes is a read-only `FootprintMirror` over the engine's snapshot bytes (the only
 * source of truth, ADR 0008 §5). Mid-draw picks are transient UI state owned by the view, not here.
 */

import {
  assertLayout,
  FootprintMirror,
  VolumeMirror,
} from "@jose/render-mirror";
import {
  type Command,
  type DraftPoint,
  type PickOptions,
  type Point,
  TOOL_CATALOG,
  ToolRunner,
} from "@jose/tool-runner";
import { useCallback, useEffect, useRef, useState } from "react";
import type { EngineRequest, EngineResponse } from "./protocol";

/** The store surface the app shell and views consume. */
export interface EngineStore {
  /** Switch the active tool, cancelling any in-progress draw. */
  readonly activate: (toolKey: string) => void;
  /** The active drawing tool's catalog key (e.g. `footprint`). */
  readonly activeTool: string;
  /** Abort the in-progress draw, keeping the active tool (Escape / value-entry bail-out). */
  readonly cancelDraw: () => void;
  /** Resolve a raw world point into the live preview a click would land on (rubber band, alignment
   *  guides, close target) — read-only; never mutates the in-progress draw. */
  readonly draft: (point: Point, options?: PickOptions) => DraftPoint;
  /** The latest canonical footprint, as a read-only mirror — `null` until the first draw returns. */
  readonly footprint: FootprintMirror | null;
  /** The mid-draw picks for the active tool (transient UI; never canonical geometry). */
  readonly pendingPicks: readonly { x: number; y: number }[];
  /** Register a snapped world pick on the active tool; emits a command into the worker on commit.
   *  `options.axisLock` constrains the pick to the X/Y axis of the prior pick (hold-Shift drawing). */
  readonly pick: (
    point: { x: number; y: number },
    options?: PickOptions
  ) => void;
  /** Dispatch a push/pull on a volume's top cap (the 3D view's gesture output). */
  readonly pushPull: (
    volumeId: number,
    faceIndex: number,
    distance: number
  ) => void;
  /** Whether the worker has finished init and passed the layout drift check. */
  readonly ready: boolean;
  /** The latest canonical volume (mass), as a read-only mirror — `null` until the first draw. */
  readonly volume: VolumeMirror | null;
}

function send(worker: Worker, request: EngineRequest): void {
  worker.postMessage(request);
}

/** Mount the engine worker and expose the shared store. One instance lives at the app root. */
export function useEngineStore(): EngineStore {
  const workerRef = useRef<Worker | null>(null);
  // Construct the ToolRunner once (lazy ref init) — useRef keeps only the first
  // value, so a bare `useRef(new ToolRunner())` would build one on every render.
  const runnerRef = useRef<ToolRunner | null>(null);
  runnerRef.current ??= new ToolRunner(undefined, "footprint");
  const runner = runnerRef.current;

  const [ready, setReady] = useState(false);
  const [activeTool, setActiveTool] = useState(runner.activeKey);
  const [pendingPicks, setPendingPicks] = useState<
    readonly { x: number; y: number }[]
  >([]);
  const [footprint, setFootprint] = useState<FootprintMirror | null>(null);
  const [volume, setVolume] = useState<VolumeMirror | null>(null);

  useEffect(() => {
    const worker = new Worker(new URL("./engine-worker.ts", import.meta.url), {
      type: "module",
    });
    workerRef.current = worker;

    worker.onmessage = (event: MessageEvent<EngineResponse>): void => {
      const response = event.data;
      if (response.kind === "ready") {
        assertLayout(response.layoutHash);
        setReady(true);
        return;
      }
      if (response.kind === "space") {
        // One recompute → fresh zero-copy mirrors over both canonical buffers; both panes re-read.
        setFootprint(
          new FootprintMirror(response.footprintBuffer, response.footprintCount)
        );
        setVolume(
          new VolumeMirror(response.volumeBuffer, response.volumeCount)
        );
      }
    };

    send(worker, { kind: "init" });
    return () => {
      worker.terminate();
      workerRef.current = null;
    };
  }, []);

  const dispatch = useCallback((command: Command): void => {
    const worker = workerRef.current;
    if (!worker) {
      return;
    }
    if (command.kind === "drawFootprint") {
      send(worker, {
        kind: "drawFootprint",
        xs: command.xs,
        ys: command.ys,
      });
    } else if (command.kind === "pushPull") {
      send(worker, {
        kind: "pushPull",
        volumeId: command.volumeId,
        faceIndex: command.faceIndex,
        distance: command.distance,
      });
    } else if (command.kind === "drawWall") {
      send(worker, command);
    }
  }, []);

  const activate = useCallback(
    (toolKey: string): void => {
      // `activeTool` is UI state spanning two kinds of tool: ToolRunner-backed *pick* tools (e.g.
      // `footprint`, which collect plan clicks) and 3D-only *gesture* tools (`pushpull`, handled by
      // three-view's drag — gated on `activeTool === "pushpull"`). The runner only knows its catalog
      // keys, so forwarding a non-runner key throws ("unknown tool"). Branch on what the runner knows.
      if (toolKey in TOOL_CATALOG) {
        runner.activate(toolKey);
        setActiveTool(runner.activeKey);
      } else {
        // A gesture tool: never reaches the runner. Cancel any in-progress footprint draw so the
        // half-drawn ring doesn't linger, then set the UI state directly.
        runner.cancel();
        setActiveTool(toolKey);
      }
      setPendingPicks([]);
    },
    [runner]
  );

  const pick = useCallback(
    (point: { x: number; y: number }, options?: PickOptions): void => {
      const command = runner.pick(point, options);
      setPendingPicks([...runner.pendingPicks]);
      if (command) {
        dispatch(command);
      }
    },
    [dispatch, runner]
  );

  const pushPull = useCallback(
    (volumeId: number, faceIndex: number, distance: number): void => {
      if (distance === 0) {
        return; // A no-op drag changes nothing — skip the recompute.
      }
      dispatch({ kind: "pushPull", volumeId, faceIndex, distance });
    },
    [dispatch]
  );

  // Pure preview: reads the runner's in-progress picks without mutating them, so calling it during
  // render (for the live cursor) is safe.
  const draft = useCallback(
    (point: { x: number; y: number }, options?: PickOptions): DraftPoint =>
      runner.draft(point, options),
    [runner]
  );

  const cancelDraw = useCallback((): void => {
    runner.cancel();
    setPendingPicks([]);
  }, [runner]);

  return {
    activeTool,
    activate,
    cancelDraw,
    draft,
    pick,
    pushPull,
    pendingPicks,
    footprint,
    volume,
    ready,
  };
}
