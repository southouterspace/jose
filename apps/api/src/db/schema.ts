/**
 * The Drizzle/Postgres schema for the persistence boundary.
 *
 * These table definitions are the durable shape behind {@link SnapshotStore} when a Neon
 * connection string is configured — the Drizzle adapter inserts/selects against them. They are
 * pure schema (no driver), so they compile and migrate without a live database. The domain
 * snapshot rides in a single `jsonb` `payload` column: persistence stores the serialized model,
 * it does not model the domain.
 */

import { integer, jsonb, pgTable, text, timestamp, uuid } from "drizzle-orm/pg-core";

/** A project/model the estimates and snapshots belong to. */
export const projects = pgTable("projects", {
  /** Stable project id. */
  id: uuid("id").primaryKey().defaultRandom(),
  /** Human name, e.g. "Smith Residence". */
  name: text("name").notNull(),
  /** Creation timestamp. */
  createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
});

/**
 * A versioned snapshot of a project's domain state — the table behind {@link SnapshotEnvelope}.
 * `(project_id, revision)` is the optimistic-lock coordinate; `payload` is the opaque serialized
 * model, stamped with the schema version + layout hash it was produced under.
 */
export const estimateSnapshots = pgTable("estimate_snapshots", {
  /** Stable snapshot id. */
  id: uuid("id").primaryKey().defaultRandom(),
  /** → the owning project. */
  projectId: uuid("project_id")
    .notNull()
    .references(() => projects.id),
  /** Optimistic-lock revision. */
  revision: integer("revision").notNull(),
  /** The domain MODEL version the payload was produced under. */
  modelVersion: text("model_version").notNull(),
  /** The BufferLayout keystone hash at production time. */
  layoutHash: text("layout_hash").notNull(),
  /** The serialized domain snapshot. */
  payload: jsonb("payload").notNull(),
  /** Creation timestamp. */
  createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
});

/** A row read out of {@link projects}. */
export type ProjectRow = typeof projects.$inferSelect;
/** A row read out of {@link estimateSnapshots}. */
export type SnapshotRow = typeof estimateSnapshots.$inferSelect;
