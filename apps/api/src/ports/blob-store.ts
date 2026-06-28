/**
 * The blob-store port — large opaque artifacts (exported drawing sets, geometry caches) that do
 * not belong in Postgres rows.
 *
 * In production this is Cloudflare R2; here an in-memory adapter stands in. Like the snapshot
 * store, it is deliberately ignorant of the domain: it moves bytes by key, nothing more.
 */

/** A stored blob: its bytes plus the content type the consumer should serve it as. */
export interface Blob {
  /** The raw bytes. */
  readonly body: Uint8Array;
  /** MIME type, e.g. `application/pdf` for an exported drawing set. */
  readonly contentType: string;
}

/** The blob-storage port: put and get bytes by key (an R2 object key). */
export interface BlobStore {
  /** Fetch the blob at `key`, or `undefined` if absent. */
  get(key: string): Promise<Blob | undefined>;
  /** Store `body` under `key`, replacing any existing object. */
  put(key: string, body: Uint8Array, contentType?: string): Promise<void>;
}

/** An in-process {@link BlobStore} — the default until an R2 bucket is configured. */
export class InMemoryBlobStore implements BlobStore {
  readonly #byKey = new Map<string, Blob>();

  put(
    key: string,
    body: Uint8Array,
    contentType = "application/octet-stream"
  ): Promise<void> {
    this.#byKey.set(key, { body, contentType });
    return Promise.resolve();
  }

  get(key: string): Promise<Blob | undefined> {
    return Promise.resolve(this.#byKey.get(key));
  }
}
