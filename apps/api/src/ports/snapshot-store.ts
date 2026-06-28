/**
 * The snapshot-store port — the persistence seam for versioned domain snapshots.
 *
 * Persistence is *orthogonal to the domain*: the store never understands an `Estimate` or a
 * `MemberBuffer`, it only round-trips an opaque, version-stamped payload. The domain serializes
 * itself into a {@link SnapshotEnvelope}; the store saves and loads envelopes. A Drizzle/Neon
 * adapter and this in-memory one are interchangeable behind the interface.
 */

/**
 * A versioned, persisted snapshot of a project's domain state. The `payload` is opaque to the
 * persistence layer; `modelVersion` + `layoutHash` stamp the schema it was produced under so a
 * stale read is caught at load, not at runtime corruption.
 */
export interface SnapshotEnvelope {
  /** ISO-8601 creation timestamp. */
  readonly createdAt: string;
  /** Stable snapshot id (UUID). */
  readonly id: string;
  /** The BufferLayout keystone hash at production time — the cross-language drift guard. */
  readonly layoutHash: string;
  /** The domain MODEL version the payload was produced under (e.g. `1.0.1`). */
  readonly modelVersion: string;
  /** The serialized domain snapshot — opaque to persistence. */
  readonly payload: unknown;
  /** The project/model this snapshot belongs to. */
  readonly projectId: string;
  /** Optimistic-lock revision; a re-solve or re-quote bumps it. */
  readonly revision: number;
}

/** The persistence port: save, load, and list versioned snapshots. */
export interface SnapshotStore {
  /** List a project's snapshots, newest revision first. */
  listByProject(projectId: string): Promise<readonly SnapshotEnvelope[]>;
  /** Load a snapshot by id, or `undefined` if unknown. */
  load(id: string): Promise<SnapshotEnvelope | undefined>;
  /** Persist a snapshot (insert or replace by id). */
  save(snapshot: SnapshotEnvelope): Promise<void>;
}

/**
 * An in-process {@link SnapshotStore} — the default until a Neon connection string is configured.
 * Backed by a `Map`; durable only for the process lifetime, which is exactly what local dev and
 * the test suite need.
 */
export class InMemorySnapshotStore implements SnapshotStore {
  readonly #byId = new Map<string, SnapshotEnvelope>();

  save(snapshot: SnapshotEnvelope): Promise<void> {
    this.#byId.set(snapshot.id, snapshot);
    return Promise.resolve();
  }

  load(id: string): Promise<SnapshotEnvelope | undefined> {
    return Promise.resolve(this.#byId.get(id));
  }

  listByProject(projectId: string): Promise<readonly SnapshotEnvelope[]> {
    const rows = [...this.#byId.values()]
      .filter((s) => s.projectId === projectId)
      .sort((a, b) => b.revision - a.revision);
    return Promise.resolve(rows);
  }
}
