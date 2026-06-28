/**
 * The app's small shared store: it owns the engine worker, the active tool, and the latest
 * canonical snapshot read back from the worker. The store holds **no** geometry of its own — the
 * footprint it exposes is a read-only `FootprintMirror` over the engine's snapshot bytes (the only
 * source of truth, ADR 0008 §5). Mid-draw picks are transient UI state owned by the view, not here.
 */

import { assertLayout, FootprintMirror } from "@jose/render-mirror";
import { type Command, ToolRunner } from "@jose/tool-runner";
import { useCallback, useEffect, useRef, useState } from "react";
import type { EngineRequest, EngineResponse } from "./protocol";

/** The store surface the app shell and views consume. */
export interface EngineStore {
  /** Switch the active tool, cancelling any in-progress draw. */
  readonly activate: (toolKey: string) => void;
  /** The active drawing tool's catalog key (e.g. `footprint`). */
  readonly activeTool: string;
  /** The latest canonical footprint, as a read-only mirror — `null` until the first draw returns. */
  readonly footprint: FootprintMirror | null;
  /** The mid-draw picks for the active tool (transient UI; never canonical geometry). */
  readonly pendingPicks: readonly { x: number; y: number }[];
  /** Register a snapped world pick on the active tool; emits a command into the worker on commit. */
  readonly pick: (point: { x: number; y: number }) => void;
  /** Whether the worker has finished init and passed the layout drift check. */
  readonly ready: boolean;
}

function send(worker: Worker, request: EngineRequest): void {
  worker.postMessage(request);
}

/** Mount the engine worker and expose the shared store. One instance lives at the app root. */
export function useEngineStore(): EngineStore {
  const workerRef = useRef<Worker | null>(null);
  const runnerRef = useRef<ToolRunner>(new ToolRunner(undefined, "footprint"));

  const [ready, setReady] = useState(false);
  const [activeTool, setActiveTool] = useState(runnerRef.current.activeKey);
  const [pendingPicks, setPendingPicks] = useState<
    readonly { x: number; y: number }[]
  >([]);
  const [footprint, setFootprint] = useState<FootprintMirror | null>(null);

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
        // Re-read the engine's canonical ring through a fresh zero-copy mirror.
        setFootprint(
          new FootprintMirror(response.footprintBuffer, response.footprintCount)
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
    } else if (command.kind === "drawWall") {
      send(worker, command);
    }
  }, []);

  const activate = useCallback((toolKey: string): void => {
    runnerRef.current.activate(toolKey);
    setActiveTool(runnerRef.current.activeKey);
    setPendingPicks([]);
  }, []);

  const pick = useCallback(
    (point: { x: number; y: number }): void => {
      const command = runnerRef.current.pick(point);
      setPendingPicks([...runnerRef.current.pendingPicks]);
      if (command) {
        dispatch(command);
      }
    },
    [dispatch]
  );

  return { activeTool, activate, pick, pendingPicks, footprint, ready };
}
