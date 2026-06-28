/**
 * The HTTP surface of the persistence boundary, built on Hono so the same app runs on the Bun
 * runtime and on Cloudflare Workers. It is a thin shell: it validates requests, stamps snapshots
 * with the schema version, and delegates to the {@link SnapshotStore} / {@link BlobStore} ports.
 * No domain logic lives here.
 */

import { Hono } from "hono";
import { LAYOUT_HASH, MODEL_VERSION } from "@jose/model-types";
import type { SnapshotEnvelope, SnapshotStore } from "./ports/snapshot-store";
import type { BlobStore } from "./ports/blob-store";

/** The adapters the app composes — injected so tests and prod wire different stores. */
export interface AppDeps {
  /** The versioned-snapshot store (Drizzle/Neon in prod, in-memory in dev/test). */
  readonly snapshots: SnapshotStore;
  /** The blob store (R2 in prod, in-memory in dev/test). */
  readonly blobs: BlobStore;
}

/** The accepted body when saving a snapshot. */
interface SaveSnapshotBody {
  readonly revision?: number;
  readonly payload: unknown;
}

/** Narrow an unknown JSON body to {@link SaveSnapshotBody}; returns `undefined` if malformed. */
function parseSaveBody(body: unknown): SaveSnapshotBody | undefined {
  if (typeof body !== "object" || body === null || !("payload" in body)) {
    return undefined;
  }
  const candidate = body as { revision?: unknown; payload: unknown };
  if (candidate.revision !== undefined && typeof candidate.revision !== "number") {
    return undefined;
  }
  return { revision: candidate.revision, payload: candidate.payload };
}

/** Build the persistence-boundary HTTP app over the given adapters. */
export function buildApp(deps: AppDeps): Hono {
  const app = new Hono();

  // Liveness + the schema version this build serves (so a client can detect a stale backend).
  app.get("/health", (c) =>
    c.json({ status: "ok", modelVersion: MODEL_VERSION, layoutHash: LAYOUT_HASH }),
  );

  // Save a new snapshot of a project's domain state. The payload is opaque; the boundary stamps
  // it with the schema version + layout hash it is being persisted under.
  app.post("/projects/:projectId/estimates", async (c) => {
    const projectId = c.req.param("projectId");
    const parsed = parseSaveBody(await c.req.json().catch(() => null));
    if (parsed === undefined) {
      return c.json({ error: "body must be { payload, revision? }" }, 400);
    }
    const snapshot: SnapshotEnvelope = {
      id: crypto.randomUUID(),
      projectId,
      revision: parsed.revision ?? 1,
      modelVersion: MODEL_VERSION,
      layoutHash: LAYOUT_HASH,
      createdAt: new Date().toISOString(),
      payload: parsed.payload,
    };
    await deps.snapshots.save(snapshot);
    return c.json({ id: snapshot.id, revision: snapshot.revision }, 201);
  });

  // List a project's snapshots, newest first.
  app.get("/projects/:projectId/estimates", async (c) => {
    const rows = await deps.snapshots.listByProject(c.req.param("projectId"));
    return c.json({ snapshots: rows });
  });

  // Load one snapshot. A snapshot produced under a different BufferLayout is rejected loudly
  // (409) rather than returned to drift against the current reader — the keystone guard, enforced
  // at the persistence boundary too.
  app.get("/estimates/:id", async (c) => {
    const snapshot = await deps.snapshots.load(c.req.param("id"));
    if (snapshot === undefined) {
      return c.json({ error: "not found" }, 404);
    }
    if (snapshot.layoutHash !== LAYOUT_HASH) {
      return c.json(
        {
          error: "stale layout",
          snapshotLayout: snapshot.layoutHash,
          currentLayout: LAYOUT_HASH,
        },
        409,
      );
    }
    return c.json(snapshot);
  });

  // Store a large opaque artifact (e.g. an exported drawing-set PDF) in the blob store.
  app.put("/blobs/:key", async (c) => {
    const key = c.req.param("key");
    const bytes = new Uint8Array(await c.req.arrayBuffer());
    const contentType = c.req.header("content-type") ?? "application/octet-stream";
    await deps.blobs.put(key, bytes, contentType);
    return c.json({ key, bytes: bytes.byteLength }, 201);
  });

  // Fetch a stored blob.
  app.get("/blobs/:key", async (c) => {
    const blob = await deps.blobs.get(c.req.param("key"));
    if (blob === undefined) {
      return c.json({ error: "not found" }, 404);
    }
    return new Response(blob.body, { headers: { "Content-Type": blob.contentType } });
  });

  return app;
}
