/**
 * The `apps/api` entry point — the deployable persistence boundary.
 *
 * It wires the in-memory adapters by default and exposes a Bun/Workers-style default export
 * (`{ port, fetch }`). When `DATABASE_URL` / `R2_BUCKET` are configured, the Drizzle/Neon snapshot
 * store and the R2 blob store drop in behind the same {@link AppDeps} ports — the app code above
 * does not change. This is the "persistence is orthogonal to the domain" boundary the plan calls
 * for: the engine and its contexts know nothing about it.
 */

import { buildApp } from "./app";
import { loadConfig } from "./config";
import { InMemoryBlobStore } from "./ports/blob-store";
import { InMemorySnapshotStore } from "./ports/snapshot-store";

const config = loadConfig();

// In-memory by default. The Drizzle/Neon + R2 adapters slot in here once configured:
//   const snapshots = config.databaseUrl ? drizzleSnapshotStore(config.databaseUrl) : ...
const app = buildApp({
  snapshots: new InMemorySnapshotStore(),
  blobs: new InMemoryBlobStore(),
});

export default {
  port: config.port,
  fetch: app.fetch,
};
