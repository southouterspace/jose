import { expect, test } from "bun:test";
import { LAYOUT_HASH } from "@jose/model-types";
import { buildApp } from "./app";
import { InMemoryBlobStore } from "./ports/blob-store";
import { InMemorySnapshotStore } from "./ports/snapshot-store";

function freshApp() {
  return buildApp({
    snapshots: new InMemorySnapshotStore(),
    blobs: new InMemoryBlobStore(),
  });
}

test("a snapshot round-trips through save and load, stamped with the layout hash", async () => {
  const app = freshApp();
  const saved = await app.fetch(
    new Request("http://localhost/projects/p1/estimates", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ revision: 2, payload: { grandTotal: 1234.5 } }),
    })
  );
  expect(saved.status).toBe(201);
  const { id } = (await saved.json()) as { id: string };

  const loaded = await app.fetch(
    new Request(`http://localhost/estimates/${id}`)
  );
  expect(loaded.status).toBe(200);
  const snap = (await loaded.json()) as {
    revision: number;
    layoutHash: string;
  };
  expect(snap.revision).toBe(2);
  expect(snap.layoutHash).toBe(LAYOUT_HASH);
});

test("a snapshot from a different BufferLayout is rejected as stale (409)", async () => {
  const snapshots = new InMemorySnapshotStore();
  await snapshots.save({
    id: "00000000-0000-0000-0000-000000000001",
    projectId: "p1",
    revision: 1,
    modelVersion: "0.0.0",
    layoutHash: "layout-stale",
    createdAt: new Date(0).toISOString(),
    payload: {},
  });
  const app = buildApp({ snapshots, blobs: new InMemoryBlobStore() });
  const loaded = await app.fetch(
    new Request(
      "http://localhost/estimates/00000000-0000-0000-0000-000000000001"
    )
  );
  expect(loaded.status).toBe(409);
});

test("a blob round-trips through put and get with its content type", async () => {
  const app = freshApp();
  const put = await app.fetch(
    new Request("http://localhost/blobs/sheet.pdf", {
      method: "PUT",
      headers: { "content-type": "application/pdf" },
      body: new Uint8Array([1, 2, 3]),
    })
  );
  expect(put.status).toBe(201);

  const got = await app.fetch(new Request("http://localhost/blobs/sheet.pdf"));
  expect(got.headers.get("content-type")).toBe("application/pdf");
  const bytes = new Uint8Array(await got.arrayBuffer());
  expect(bytes.length).toBe(3);
});

test("an unknown snapshot is a 404", async () => {
  const app = freshApp();
  const loaded = await app.fetch(
    new Request(
      "http://localhost/estimates/00000000-0000-0000-0000-0000000000ff"
    )
  );
  expect(loaded.status).toBe(404);
});
