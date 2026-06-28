# api — persistence boundary

The backend service: a thin, **domain-orthogonal** persistence boundary on the Bun runtime
(deployable to Cloudflare Workers via the same Hono app). It stores versioned domain *snapshots*
in Neon/Postgres (via Drizzle) and large opaque artifacts in Cloudflare R2 — it never models the
domain itself.

```
src/
├─ index.ts              entry: wires adapters, exports { port, fetch }
├─ app.ts                the Hono HTTP surface (validate → stamp → delegate)
├─ config.ts             env-driven config (DATABASE_URL, R2_BUCKET, PORT)
├─ db/schema.ts          Drizzle/Postgres tables (projects, estimate_snapshots)
└─ ports/
   ├─ snapshot-store.ts  SnapshotStore port + InMemorySnapshotStore
   └─ blob-store.ts      BlobStore port + InMemoryBlobStore
```

## Why a snapshot envelope

Persistence is orthogonal to the domain (per [`docs/plans/repo-scaffold.md`](../../docs/plans/repo-scaffold.md)
§1): the store round-trips an opaque `payload` wrapped in a `SnapshotEnvelope` stamped with the
`MODEL_VERSION` and the `LAYOUT_HASH` keystone (from `@jose/model-types`). A snapshot produced
under a different `BufferLayout` is rejected at load with **409 stale layout** rather than allowed
to drift against the current reader — the keystone guarantee, enforced at the persistence boundary
too.

## Routes

| Method & path | Purpose |
|---|---|
| `GET /health` | liveness + the served `modelVersion` / `layoutHash` |
| `POST /projects/:projectId/estimates` | save a snapshot (`{ payload, revision? }`) |
| `GET /projects/:projectId/estimates` | list a project's snapshots, newest first |
| `GET /estimates/:id` | load a snapshot (409 if its layout is stale) |
| `PUT /blobs/:key` | store a blob (e.g. an exported drawing-set PDF) in R2 |
| `GET /blobs/:key` | fetch a stored blob |

## Run

```bash
bun run --filter api dev        # hot-reload dev server (in-memory stores)
bun run --filter api typecheck
bun run --filter api test
```

Set `DATABASE_URL` (Neon) and `R2_BUCKET` to swap the in-memory adapters for the Drizzle snapshot
store and the R2 blob store behind the same ports — the app code does not change.
